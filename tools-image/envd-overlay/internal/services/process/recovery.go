package process

import (
	"context"
	"errors"
	"fmt"
	"io"
	"time"

	"connectrpc.com/connect"
	"github.com/e2b-dev/infra/packages/envd/internal/processio"
	"github.com/e2b-dev/infra/packages/envd/internal/services/process/handler"
	rpc "github.com/e2b-dev/infra/packages/envd/internal/services/spec/process"
	"google.golang.org/protobuf/proto"
)

const maxRecoverableProcesses = 64
const exitedProcessRetention = 10 * time.Minute

// Reserve before Handler.New, which may start a PTY. Active and retained exited
// processes share the same cap, so disconnected clients cannot grow the journal
// registry without bound. Existing legacy clients retain their existing API.
func (s *Service) reserveRecovery(enabled bool) (func(), error) {
	if !enabled {
		return nil, nil
	}
	s.recoverableMu.Lock()
	defer s.recoverableMu.Unlock()
	if s.recoverableCount >= maxRecoverableProcesses {
		return nil, connect.NewError(connect.CodeResourceExhausted, fmt.Errorf("recoverable process retention capacity exhausted"))
	}
	s.recoverableCount++
	return func() { s.recoverableMu.Lock(); s.recoverableCount--; s.recoverableMu.Unlock() }, nil
}

func recoveryError(err error) error {
	code := connect.CodeFailedPrecondition
	switch {
	case errors.Is(err, context.DeadlineExceeded):
		code = connect.CodeDeadlineExceeded
	case errors.Is(err, context.Canceled):
		code = connect.CodeCanceled
	case errors.Is(err, processio.ErrIncarnation):
		code = connect.CodeFailedPrecondition
	case errors.Is(err, processio.ErrSequence):
		code = connect.CodeInvalidArgument
	case errors.Is(err, processio.ErrConflict):
		code = connect.CodeAlreadyExists
	case errors.Is(err, processio.ErrExpired):
		code = connect.CodeOutOfRange
	case errors.Is(err, processio.ErrLimit):
		code = connect.CodeResourceExhausted
	}
	return connect.NewError(code, err)
}

func (s *Service) startRecoverable(ctx context.Context, proc *handler.Handler, timeout time.Duration, release func(), send func(*rpc.ProcessEvent) error) error {
	pid, err := proc.Start(timeout)
	if err != nil {
		proc.AbortStart()
		go func() { waitResidencyCleanup(proc); release() }()
		return connect.NewError(connect.CodeInvalidArgument, err)
	}
	s.recoverableMu.Lock()
	s.recoverable[proc.Incarnation()] = proc
	s.recoverableMu.Unlock()
	s.storeProcess(pid, proc)
	go func() {
		proc.Wait()
		// A PID may have been reused after exit; never delete the new process.
		// The PID map is an active listing only. Incarnation lookup owns recovery.
		// Handler.Wait follows process exit closely, but compare under the map's
		// process mutex is required before removing a reused key.
		s.retireProcess(pid, proc)
		waitResidencyCleanup(proc)
		timer := time.NewTimer(exitedProcessRetention)
		defer timer.Stop()
		<-timer.C
		s.recoverableMu.Lock()
		delete(s.recoverable, proc.Incarnation())
		s.recoverableMu.Unlock()
		release()
	}()
	return streamRecoverable(ctx, proc, 0, send)
}

// A parent exit is not proof that all children stopped. Keep the bounded
// recovery reservation while descendants or an uncertain kernel result prevent
// removal. Never thaw or kill them as a side effect of journal expiration.
func waitResidencyCleanup(proc *handler.Handler) {
	for proc.CleanupResidency() != nil {
		time.Sleep(time.Second)
	}
}

func streamRecoverable(ctx context.Context, proc *handler.Handler, after uint64, send func(*rpc.ProcessEvent) error) error {
	// Validate before sending any success-shaped event, including expired cursors.
	if _, _, err := proc.IO.Read(proc.Incarnation(), after); err != nil && !errors.Is(err, io.EOF) {
		return recoveryError(err)
	}
	if err := send(&rpc.ProcessEvent{Event: &rpc.ProcessEvent_Start{Start: &rpc.ProcessEvent_StartEvent{Pid: proc.Pid(), Incarnation: proc.Incarnation()}}}); err != nil {
		return err
	}
	ticker := time.NewTicker(10 * time.Second)
	defer ticker.Stop()
	for {
		events, changed, err := proc.IO.Read(proc.Incarnation(), after)
		if errors.Is(err, io.EOF) {
			return nil
		}
		if err != nil {
			return recoveryError(err)
		}
		for _, frame := range events {
			event := &rpc.ProcessEvent{}
			if err := proto.Unmarshal(frame.Payload, event); err != nil {
				return connect.NewError(connect.CodeInternal, fmt.Errorf("invalid retained output frame"))
			}
			event.Sequence = frame.Sequence
			if err := send(event); err != nil {
				return err
			}
			after = frame.Sequence
		}
		// Re-read after a batch; the close notification can precede this read.
		if len(events) > 0 {
			continue
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-changed:
		case <-ticker.C:
			if err := send(&rpc.ProcessEvent{Event: &rpc.ProcessEvent_Keepalive{Keepalive: &rpc.ProcessEvent_KeepAlive{}}}); err != nil {
				return err
			}
		}
	}
}

func (s *Service) storeProcess(pid uint32, proc *handler.Handler) {
	s.processMu.Lock()
	defer s.processMu.Unlock()
	s.processes.Store(pid, proc)
}
func (s *Service) retireProcess(pid uint32, proc *handler.Handler) {
	s.processMu.Lock()
	defer s.processMu.Unlock()
	if current, ok := s.processes.Load(pid); ok && current == proc {
		s.processes.Delete(pid)
	}
}
