package processio

import (
	"errors"
	"math"
)

var ErrHandoff = errors.New("process residency handoff blocks input")

type handoffPhase uint8

const (
	handoffIdle handoffPhase = iota
	handoffPreparing
	handoffPrepared
	handoffResuming
	handoffReleased
)

type handoffState struct {
	epoch     uint64
	operation string
	phase     handoffPhase
}

func (h handoffState) blocked() bool {
	return h.phase != handoffIdle && h.phase != handoffReleased
}

// PrepareHandoff closes new input admission before freezing the process tree.
// The same operation may retry a failed/unknown freeze; the backend must make
// freeze idempotent and acknowledge physical quiescence, including descendants.
// An expired lease, pipe closure or stopped parent PID is not that evidence.
// Callbacks run under the input lock and must not reenter this journal.
// This primitive does not authorize a target residency or fence another VM.
func (j *Journal) PrepareHandoff(id string, epoch uint64, operation string, freeze func() error) error {
	if id != j.id {
		return ErrIncarnation
	}
	if epoch == 0 || epoch == math.MaxUint64 || len(operation) == 0 || len(operation) > 200 || freeze == nil {
		return ErrSequence
	}
	j.inputMu.Lock()
	defer j.inputMu.Unlock()
	h := &j.handoff
	if epoch == h.epoch {
		if operation != h.operation {
			return ErrConflict
		}
		if h.phase == handoffPrepared {
			return nil
		}
		if h.phase != handoffPreparing {
			return ErrHandoff
		}
	} else {
		if epoch != h.epoch+1 || h.blocked() {
			return ErrHandoff
		}
		*h = handoffState{epoch: epoch, operation: operation, phase: handoffPreparing}
	}
	// Failure or panic keeps admission closed. Repeating Prepare cannot release
	// the barrier or substitute a different operation after an unknown outcome.
	if err := freeze(); err != nil {
		return err
	}
	h.phase = handoffPrepared
	return nil
}

// ResumeHandoff is called only after the outer runtime authority has fenced
// the source and restored the destination's credentials and filesystem. The
// backend resumes the same frozen process tree; it must never spawn a new one.
// Unknown resume outcomes retain the input barrier and retry this exact epoch.
// Existing input receipts and the output journal remain unchanged throughout.
func (j *Journal) ResumeHandoff(id string, epoch uint64, operation string, resume func() error) error {
	if id != j.id {
		return ErrIncarnation
	}
	if epoch == 0 || len(operation) == 0 || resume == nil {
		return ErrSequence
	}
	j.inputMu.Lock()
	defer j.inputMu.Unlock()
	h := &j.handoff
	if epoch != h.epoch || operation != h.operation {
		return ErrConflict
	}
	if h.phase == handoffReleased {
		return nil
	}
	if h.phase != handoffPrepared && h.phase != handoffResuming {
		return ErrHandoff
	}
	h.phase = handoffResuming
	if err := resume(); err != nil {
		return err
	}
	h.phase = handoffReleased
	return nil
}
