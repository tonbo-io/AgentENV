package process

import (
	"context"
	"fmt"

	"connectrpc.com/connect"
	rpc "github.com/e2b-dev/infra/packages/envd/internal/services/spec/process"
)

func (s *Service) PrepareHandoff(ctx context.Context, req *connect.Request[rpc.ProcessHandoffRequest]) (*connect.Response[rpc.ProcessHandoffResponse], error) {
	return s.handoff(ctx, req.Msg, true)
}

func (s *Service) ResumeHandoff(ctx context.Context, req *connect.Request[rpc.ProcessHandoffRequest]) (*connect.Response[rpc.ProcessHandoffResponse], error) {
	return s.handoff(ctx, req.Msg, false)
}

func (s *Service) handoff(ctx context.Context, req *rpc.ProcessHandoffRequest, prepare bool) (*connect.Response[rpc.ProcessHandoffResponse], error) {
	if req.GetProcess().GetIncarnation() == "" {
		return nil, connect.NewError(connect.CodeFailedPrecondition, fmt.Errorf("handoff requires a process incarnation"))
	}
	proc, err := s.getProcess(req.GetProcess())
	if err != nil {
		return nil, err
	}
	if prepare {
		err = proc.PrepareHandoff(ctx, req.GetEpoch(), req.GetOperationId())
	} else {
		err = proc.ResumeHandoff(ctx, req.GetEpoch(), req.GetOperationId())
	}
	if err != nil {
		return nil, recoveryError(err)
	}
	return connect.NewResponse(&rpc.ProcessHandoffResponse{
		Incarnation: proc.Incarnation(), Epoch: req.GetEpoch(), OperationId: req.GetOperationId(),
	}), nil
}
