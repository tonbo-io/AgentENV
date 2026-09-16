package process

import (
	"context"
	"testing"

	"connectrpc.com/connect"
	"github.com/e2b-dev/infra/packages/envd/internal/processio"
	"github.com/e2b-dev/infra/packages/envd/internal/services/process/handler"
	rpc "github.com/e2b-dev/infra/packages/envd/internal/services/spec/process"
)

func TestHandoffRPCRejectsLegacyUnknownAndUnfreezableProcesses(t *testing.T) {
	s := fixtureService()
	j, err := processio.New()
	if err != nil {
		t.Fatal(err)
	}
	s.recoverable[j.ID()] = &handler.Handler{IO: j}
	for _, prepare := range []bool{true, false} {
		for _, tc := range []struct {
			selector *rpc.ProcessSelector
			code     connect.Code
		}{
			{nil, connect.CodeFailedPrecondition},
			{&rpc.ProcessSelector{Selector: &rpc.ProcessSelector_Pid{Pid: 1}}, connect.CodeFailedPrecondition},
			{&rpc.ProcessSelector{Selector: &rpc.ProcessSelector_Incarnation{Incarnation: "unknown"}}, connect.CodeNotFound},
			{&rpc.ProcessSelector{Selector: &rpc.ProcessSelector_Incarnation{Incarnation: j.ID()}}, connect.CodeFailedPrecondition},
		} {
			result, err := s.handoff(context.Background(), &rpc.ProcessHandoffRequest{Process: tc.selector, Epoch: 1, OperationId: "move"}, prepare)
			if result != nil || connect.CodeOf(err) != tc.code {
				t.Fatalf("prepare=%v: result=%v err=%v", prepare, result, err)
			}
		}
	}
	if connect.CodeOf(recoveryError(context.DeadlineExceeded)) != connect.CodeDeadlineExceeded {
		t.Fatal("lost deadline status")
	}
	if connect.CodeOf(recoveryError(context.Canceled)) != connect.CodeCanceled {
		t.Fatal("lost cancellation status")
	}
}
