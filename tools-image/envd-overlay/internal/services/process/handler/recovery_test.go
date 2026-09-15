package handler

import (
	"errors"
	"io"
	"testing"

	"github.com/e2b-dev/infra/packages/envd/internal/processio"
	rpc "github.com/e2b-dev/infra/packages/envd/internal/services/spec/process"
	"google.golang.org/protobuf/proto"
)

type testPipe struct {
	calls, closes int
	data          []byte
	partial       bool
}

func (p *testPipe) Write(b []byte) (int, error) {
	p.calls++
	p.data = append(p.data, b...)
	if p.partial {
		return 1, io.ErrClosedPipe
	}
	return len(b), nil
}
func (p *testPipe) Close() error { p.closes++; return nil }
func recoveringHandler(t *testing.T) (*Handler, *testPipe) {
	t.Helper()
	j, err := processio.New()
	if err != nil {
		t.Fatal(err)
	}
	pipe := &testPipe{}
	return &Handler{IO: j, stdin: pipe}, pipe
}
func input(text string) *rpc.ProcessInput {
	return &rpc.ProcessInput{Input: &rpc.ProcessInput_Stdin{Stdin: []byte(text)}}
}

func TestRecoverableInputCannotBeBypassedByLegacyWrites(t *testing.T) {
	p, pipe := recoveringHandler(t)
	if err := p.WriteStdin([]byte("raw")); !errors.Is(err, processio.ErrIncarnation) {
		t.Fatal(err)
	}
	if err := p.WriteTty([]byte("raw")); !errors.Is(err, processio.ErrIncarnation) {
		t.Fatal(err)
	}
	if pipe.calls != 0 {
		t.Fatal(pipe.calls)
	}
	first, err := p.WriteRecoverable(1, input("prompt\n"))
	if err != nil {
		t.Fatal(err)
	}
	again, err := p.WriteRecoverable(1, input("prompt\n"))
	if err != nil || first != again || pipe.calls != 1 {
		t.Fatal(again, err, pipe.calls)
	}
	if err := p.CloseStdin(); err != nil {
		t.Fatal(err)
	}
	if _, err := p.WriteRecoverable(2, input("later")); !errors.Is(err, processio.ErrClosed) {
		t.Fatal(err)
	}
	if _, err := p.WriteRecoverable(1, input("prompt\n")); err != nil {
		t.Fatal(err)
	}
	if pipe.closes != 1 {
		t.Fatal(pipe.closes)
	}
}

func TestRealWriteCountControlsReceiptAndFencesPartialInput(t *testing.T) {
	p, pipe := recoveringHandler(t)
	pipe.partial = true
	receipt, err := p.WriteRecoverable(1, input("prompt"))
	if !errors.Is(err, processio.ErrUncertain) || receipt.Written != 1 {
		t.Fatal(receipt, err)
	}
	if _, err := p.WriteRecoverable(1, input("prompt")); !errors.Is(err, processio.ErrUncertain) {
		t.Fatal(err)
	}
	if pipe.calls != 1 {
		t.Fatal(pipe.calls)
	}
}

func TestHandlerJournalsProtobufOutputWithoutSubscribers(t *testing.T) {
	p, _ := recoveringHandler(t)
	p.emitData(rpc.ProcessEvent_Data{Data: &rpc.ProcessEvent_DataEvent{Output: &rpc.ProcessEvent_DataEvent_Stdout{Stdout: []byte("result")}}})
	p.appendEvent(&rpc.ProcessEvent{Event: &rpc.ProcessEvent_End{End: &rpc.ProcessEvent_EndEvent{Exited: true}}})
	p.IO.CloseOutput()
	events, _, err := p.IO.Read(p.Incarnation(), 0)
	if err != nil || len(events) != 2 {
		t.Fatal(len(events), err)
	}
	var first, last rpc.ProcessEvent
	if err := proto.Unmarshal(events[0].Payload, &first); err != nil {
		t.Fatal(err)
	}
	if err := proto.Unmarshal(events[1].Payload, &last); err != nil {
		t.Fatal(err)
	}
	if string(first.GetData().GetStdout()) != "result" || !last.GetEnd().GetExited() {
		t.Fatal(&first, &last)
	}
	if events[0].Sequence != 1 || events[1].Sequence != 2 {
		t.Fatal(events)
	}
}
