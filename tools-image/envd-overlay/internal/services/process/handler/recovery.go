package handler

import (
	"errors"

	"github.com/e2b-dev/infra/packages/envd/internal/processio"
	rpc "github.com/e2b-dev/infra/packages/envd/internal/services/spec/process"
	"google.golang.org/protobuf/proto"
)

func (p *Handler) Incarnation() string {
	if p.IO == nil {
		return ""
	}
	return p.IO.ID()
}

func (p *Handler) WriteRecoverable(sequence uint64, input *rpc.ProcessInput) (processio.Receipt, error) {
	if p.IO == nil {
		return processio.Receipt{}, processio.ErrIncarnation
	}
	channel := ""
	var data []byte
	switch input.GetInput().(type) {
	case *rpc.ProcessInput_Stdin:
		channel = "stdin"
		data = input.GetStdin()
	case *rpc.ProcessInput_Pty:
		channel = "pty"
		data = input.GetPty()
	default:
		return processio.Receipt{}, processio.ErrSequence
	}
	return p.IO.Write(p.IO.ID(), sequence, channel, data, func(payload []byte) (int, error) {
		if channel == "pty" {
			if p.tty == nil {
				return 0, errors.New("process has no PTY")
			}
			return p.tty.Write(payload)
		}
		p.stdinMu.Lock()
		defer p.stdinMu.Unlock()
		if p.tty != nil || p.stdin == nil {
			return 0, errors.New("process stdin unavailable")
		}
		return p.stdin.Write(payload)
	})
}

func (p *Handler) emitData(event rpc.ProcessEvent_Data) {
	if p.IO == nil {
		p.DataEvent.Source <- event
		return
	}
	p.appendEvent(&rpc.ProcessEvent{Event: &event})
}

func (p *Handler) appendEvent(event *rpc.ProcessEvent) {
	// Generated protobuf frames have no maps or user-supplied message implementations.
	payload, err := proto.Marshal(event)
	if err != nil {
		panic(err)
	}
	// Overflow permanently expires the output log. Readers receive an explicit
	// error; process stdout continues draining without subscriber backpressure.
	_ = p.IO.Append(payload)
}

// AbortStart releases the output/event machinery after exec.Cmd.Start fails.
// os/exec closes its failed-start pipes; no process or terminal result exists.
func (p *Handler) AbortStart() {
	p.cancel()
	p.outCancel()
	<-p.outputDone
	p.IO.CloseOutput()
	close(p.EndEvent.Source)
}
