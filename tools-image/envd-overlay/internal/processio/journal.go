// Package processio owns bounded process I/O recovery for one live supervisor
// incarnation. It cannot establish delivery after that authority is lost.
package processio

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"io"
	"math"
	"sync"
)

var (
	ErrIncarnation = errors.New("process incarnation mismatch")
	ErrSequence    = errors.New("invalid input or output sequence")
	ErrConflict    = errors.New("input sequence already names different bytes or channel")
	ErrExpired     = errors.New("requested I/O history has expired")
	ErrUncertain   = errors.New("process input delivery is uncertain")
	ErrClosed      = errors.New("process I/O is closed")
	ErrLimit       = errors.New("process I/O limit exceeded")
)

const (
	MaxInputBytes   = 1 << 20
	MaxReceipts     = 1024
	MaxOutputBytes  = 8 << 20
	MaxOutputEvents = 4096
)

// Receipt confirms bytes written to the process pipe, not application processing
// or external effects. An uncertain write permanently fences subsequent input.
type Receipt struct {
	Sequence  uint64
	Digest    string
	Written   int
	Uncertain bool
}

type inputRecord struct {
	receipt Receipt
	channel string
}

// Event is an immutable numbered output frame. Payload encoding belongs to the
// existing process RPC; retention and cursor semantics belong to this journal.
type Event struct {
	Sequence uint64
	Payload  []byte
}

type Journal struct {
	id            string
	inputMu       sync.Mutex
	nextInput     uint64
	inputs        map[uint64]inputRecord
	uncertain     bool
	inputClosed   bool
	inputCloseErr error
	outputMu      sync.Mutex
	nextOutput    uint64
	output        []Event
	outputBytes   int
	outputClosed  bool
	outputLost    bool
	changed       chan struct{}
}

func New() (*Journal, error) {
	var id [32]byte
	if _, err := rand.Read(id[:]); err != nil {
		return nil, err
	}
	return &Journal{id: hex.EncodeToString(id[:]), nextInput: 1, inputs: make(map[uint64]inputRecord), nextOutput: 1, changed: make(chan struct{})}, nil
}

func (j *Journal) ID() string { return j.id }

// Write serializes receipt lookup and physical delivery. A disconnected caller
// does not cancel a pipe write: a retry joins this authority and observes the
// same result. Raw stdin/PTY writes must not bypass it for a recoverable process.
func (j *Journal) Write(id string, sequence uint64, channel string, payload []byte, deliver func([]byte) (int, error)) (Receipt, error) {
	if id != j.id {
		return Receipt{}, ErrIncarnation
	}
	if sequence == 0 || sequence == math.MaxUint64 || (channel != "stdin" && channel != "pty") {
		return Receipt{}, ErrSequence
	}
	if len(payload) > MaxInputBytes {
		return Receipt{}, ErrLimit
	}
	owned := append([]byte(nil), payload...)
	digest := sha256.Sum256(owned)
	hash := hex.EncodeToString(digest[:])
	j.inputMu.Lock()
	defer j.inputMu.Unlock()
	if previous, ok := j.inputs[sequence]; ok {
		if previous.channel != channel || previous.receipt.Digest != hash {
			return Receipt{}, ErrConflict
		}
		if previous.receipt.Uncertain {
			return previous.receipt, ErrUncertain
		}
		return previous.receipt, nil
	}
	if sequence < j.nextInput {
		return Receipt{}, ErrExpired
	}
	if sequence != j.nextInput {
		return Receipt{}, ErrSequence
	}
	if j.uncertain {
		return Receipt{}, ErrUncertain
	}
	if j.inputClosed {
		return Receipt{}, ErrClosed
	}
	// If delivery panics, the RPC server may recover; retries must still fail closed.
	j.uncertain = true
	n, err := deliver(owned)
	receipt := Receipt{Sequence: sequence, Digest: hash, Written: n, Uncertain: err != nil || n != len(owned)}
	j.inputs[sequence] = inputRecord{receipt: receipt, channel: channel}
	j.nextInput++
	if sequence > MaxReceipts {
		delete(j.inputs, sequence-MaxReceipts)
	}
	if receipt.Uncertain {
		j.uncertain = true
		return receipt, ErrUncertain
	}
	j.uncertain = false
	return receipt, nil
}

// CloseInput runs close behind the same lock as writes. A replay of an already
// retained receipt stays valid after closure, but no new write is accepted.
func (j *Journal) CloseInput(id string, closePipe func() error) error {
	if id != j.id {
		return ErrIncarnation
	}
	j.inputMu.Lock()
	defer j.inputMu.Unlock()
	if j.inputClosed {
		return j.inputCloseErr
	}
	j.inputClosed = true
	// A panic during close must never turn a replay into success.
	j.inputCloseErr = ErrUncertain
	j.inputCloseErr = closePipe()
	return j.inputCloseErr
}

// Append never waits on a reader. A slow reader eventually receives ErrExpired,
// instead of blocking process output or silently skipping missing frames.
func (j *Journal) Append(payload []byte) error {
	j.outputMu.Lock()
	defer j.outputMu.Unlock()
	if j.outputClosed {
		return ErrClosed
	}
	if len(payload) > MaxOutputBytes || j.nextOutput == math.MaxUint64 {
		j.outputLost = true
		j.outputClosed = true
		j.notify()
		return ErrLimit
	}
	j.output = append(j.output, Event{Sequence: j.nextOutput, Payload: append([]byte(nil), payload...)})
	j.nextOutput++
	j.outputBytes += len(payload)
	for len(j.output) > MaxOutputEvents || j.outputBytes > MaxOutputBytes {
		j.outputBytes -= len(j.output[0].Payload)
		j.output[0] = Event{}
		j.output = j.output[1:]
	}
	j.notify()
	return nil
}

func (j *Journal) notify() { close(j.changed); j.changed = make(chan struct{}) }

func (j *Journal) CloseOutput() {
	j.outputMu.Lock()
	defer j.outputMu.Unlock()
	if !j.outputClosed {
		j.outputClosed = true
		j.notify()
	}
}

// Read returns frames strictly after the last consumed sequence. The returned
// notification channel is captured under the same lock: no output can fall
// between checking the journal and subscribing. A caller advances its cursor
// only after consuming a frame. io.EOF means a completely consumed closed log.
func (j *Journal) Read(id string, after uint64) ([]Event, <-chan struct{}, error) {
	if id != j.id {
		return nil, nil, ErrIncarnation
	}
	j.outputMu.Lock()
	defer j.outputMu.Unlock()
	if j.outputLost {
		return nil, nil, ErrExpired
	}
	if after >= j.nextOutput {
		return nil, nil, ErrSequence
	}
	first := j.nextOutput
	if len(j.output) > 0 {
		first = j.output[0].Sequence
	}
	if after < first-1 {
		return nil, nil, ErrExpired
	}
	var result []Event
	for _, event := range j.output {
		if event.Sequence > after {
			result = append(result, Event{Sequence: event.Sequence, Payload: append([]byte(nil), event.Payload...)})
		}
	}
	if len(result) == 0 && j.outputClosed {
		return nil, nil, io.EOF
	}
	return result, j.changed, nil
}
