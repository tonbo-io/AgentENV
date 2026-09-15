package process

import (
	"context"
	"errors"
	"io"
	"os/user"
	"testing"
	"time"

	"connectrpc.com/connect"
	"github.com/e2b-dev/infra/packages/envd/internal/execcontext"
	"github.com/e2b-dev/infra/packages/envd/internal/processio"
	"github.com/e2b-dev/infra/packages/envd/internal/services/cgroups"
	"github.com/e2b-dev/infra/packages/envd/internal/services/process/handler"
	rpc "github.com/e2b-dev/infra/packages/envd/internal/services/spec/process"
	"github.com/e2b-dev/infra/packages/envd/internal/utils"
	"github.com/rs/zerolog"
)

func fixtureService() *Service {
	return &Service{recoverable: make(map[string]*handler.Handler), processes: utils.NewMap[uint32, *handler.Handler]()}
}
func TestRecoverableSelectorCannotFallBackToPIDOrTag(t *testing.T) {
	s := fixtureService()
	j, err := processio.New()
	if err != nil {
		t.Fatal(err)
	}
	tag := "fixture"
	p := &handler.Handler{IO: j, Tag: &tag}
	s.recoverable[j.ID()] = p
	s.storeProcess(123, p)
	for _, selector := range []*rpc.ProcessSelector{
		{Selector: &rpc.ProcessSelector_Pid{Pid: 123}},
		{Selector: &rpc.ProcessSelector_Tag{Tag: tag}},
	} {
		if _, err := s.getProcess(selector); connect.CodeOf(err) != connect.CodeFailedPrecondition {
			t.Fatal(err)
		}
	}
	if got, err := s.getProcess(&rpc.ProcessSelector{Selector: &rpc.ProcessSelector_Incarnation{Incarnation: j.ID()}}); err != nil || got != p {
		t.Fatal(got, err)
	}
	if _, err := s.getProcess(&rpc.ProcessSelector{Selector: &rpc.ProcessSelector_Incarnation{Incarnation: "expired"}}); connect.CodeOf(err) != connect.CodeNotFound {
		t.Fatal(err)
	}
}
func TestPIDReuseDoesNotLoseNewProcessOrOldIncarnation(t *testing.T) {
	s := fixtureService()
	j, err := processio.New()
	if err != nil {
		t.Fatal(err)
	}
	old, newProcess := &handler.Handler{IO: j}, &handler.Handler{}
	s.recoverable[j.ID()] = old
	s.storeProcess(123, old)
	s.storeProcess(123, newProcess)
	s.retireProcess(123, old)
	if got, _ := s.processes.Load(123); got != newProcess {
		t.Fatal("new PID occupant deleted")
	}
	if got, err := s.getProcess(&rpc.ProcessSelector{Selector: &rpc.ProcessSelector_Incarnation{Incarnation: j.ID()}}); err != nil || got != old {
		t.Fatal(got, err)
	}
}
func TestRecoveryCapacityMustBeReservedBeforeProcessCreation(t *testing.T) {
	s := fixtureService()
	var releases []func()
	for range maxRecoverableProcesses {
		release, err := s.reserveRecovery(true)
		if err != nil {
			t.Fatal(err)
		}
		releases = append(releases, release)
	}
	if _, err := s.reserveRecovery(true); connect.CodeOf(err) != connect.CodeResourceExhausted {
		t.Fatal(err)
	}
	if release, err := s.reserveRecovery(false); err != nil || release != nil {
		t.Fatal("legacy API cannot reserve a recovery slot")
	}
	releases[0]()
	release, err := s.reserveRecovery(true)
	if err != nil {
		t.Fatal(err)
	}
	release()
	for _, release := range releases[1:] {
		release()
	}
	if s.recoverableCount != 0 {
		t.Fatal(s.recoverableCount)
	}
}

// Uses a real child pipe and the service methods, without a VM or remote API.
// The tools-image Linux build runs this with the race detector before packaging.
func TestLostInputAckAndDisconnectedOutputKeepOneProcessResult(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	account, err := user.Current()
	if err != nil {
		t.Fatal(err)
	}
	logger := zerolog.Nop()
	cwd := t.TempDir()
	enabled := true
	proc, err := handler.New(ctx, account, &rpc.StartRequest{
		Process: &rpc.ProcessConfig{Cmd: "/bin/sh", Args: []string{"-c", `while IFS= read -r line; do printf '%s\n' "$line"; done`}, Cwd: &cwd},
		Stdin:   &enabled, RecoverableIo: true,
	}, &logger, &execcontext.Defaults{}, cgroups.NewNoopManager(), cancel)
	if err != nil {
		t.Fatal(err)
	}
	s := fixtureService()
	release, err := s.reserveRecovery(true)
	if err != nil {
		t.Fatal(err)
	}
	// Disconnect immediately after the Start event. The child must survive.
	if err := s.startRecoverable(ctx, proc, 10*time.Second, release, func(*rpc.ProcessEvent) error { return io.ErrClosedPipe }); !errors.Is(err, io.ErrClosedPipe) {
		t.Fatalf("start disconnect: %v", err)
	}
	selector := &rpc.ProcessSelector{Selector: &rpc.ProcessSelector_Incarnation{Incarnation: proc.Incarnation()}}
	request := connect.NewRequest(&rpc.SendInputRequest{Process: selector, Sequence: 1,
		Input: &rpc.ProcessInput{Input: &rpc.ProcessInput_Stdin{Stdin: []byte("one\n")}},
	})
	first, err := s.SendInput(ctx, request)
	if err != nil {
		t.Fatal(err)
	}
	// Pretend the response was lost; exact replay must return the same receipt.
	again, err := s.SendInput(ctx, request)
	if err != nil || first.Msg.Sequence != again.Msg.Sequence || first.Msg.Sha256 != again.Msg.Sha256 || again.Msg.WrittenBytes != 4 {
		t.Fatal(again, err)
	}
	if _, err := s.CloseStdin(ctx, connect.NewRequest(&rpc.CloseStdinRequest{Process: selector})); err != nil {
		t.Fatal(err)
	}
	var output []byte
	var cursor uint64
	ended := false
	// Disconnect once after consuming the first retained output frame.
	err = streamRecoverable(ctx, proc, 0, func(event *rpc.ProcessEvent) error {
		if data := event.GetData(); data != nil {
			output = append(output, data.GetStdout()...)
			cursor = event.Sequence
			return io.ErrClosedPipe
		}
		return nil
	})
	if !errors.Is(err, io.ErrClosedPipe) || cursor == 0 {
		t.Fatal(err, cursor)
	}
	err = streamRecoverable(ctx, proc, cursor, func(event *rpc.ProcessEvent) error {
		if event.Sequence > 0 && event.Sequence <= cursor {
			t.Fatal("replayed a consumed frame")
		}
		if data := event.GetData(); data != nil {
			output = append(output, data.GetStdout()...)
		}
		if end := event.GetEnd(); end != nil {
			ended = end.GetExited() && end.GetExitCode() == 0
		}
		return nil
	})
	if err != nil || string(output) != "one\n" || !ended {
		t.Fatalf("output=%q ended=%v error=%v", output, ended, err)
	}
}
