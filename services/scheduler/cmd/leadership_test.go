package main

import (
	"context"
	"testing"
	"time"

	schedulerv1 "agentenv/services/api/proto"
	"agentenv/services/shared/config"

	"go.uber.org/zap"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/health/grpc_health_v1"
	"google.golang.org/grpc/status"
	"k8s.io/client-go/tools/leaderelection/resourcelock"
)

func TestLeadershipGateRejectsSchedulerRPCWhileFollower(t *testing.T) {
	gate := newLeadershipGate(false)
	handlerCalled := false
	handler := func(context.Context, any) (any, error) {
		handlerCalled = true
		return "ok", nil
	}

	_, err := gate.unaryServerInterceptor()(context.Background(), nil, &grpc.UnaryServerInfo{FullMethod: "/agentenv.scheduler.v1.Scheduler/Schedule"}, handler)
	if status.Code(err) != codes.Unavailable {
		t.Fatalf("expected follower RPC to be unavailable, got %v", err)
	}
	if handlerCalled {
		t.Fatal("follower invoked scheduler handler")
	}
}

func TestLeadershipGateAllowsHealthAndLeaderRPCs(t *testing.T) {
	gate := newLeadershipGate(false)
	handler := func(context.Context, any) (any, error) { return "ok", nil }

	response, err := gate.unaryServerInterceptor()(context.Background(), nil, &grpc.UnaryServerInfo{FullMethod: "/grpc.health.v1.Health/Check"}, handler)
	if err != nil || response != "ok" {
		t.Fatalf("health RPC should bypass leadership gate: response=%v error=%v", response, err)
	}

	gate.setLeader(true)
	response, err = gate.unaryServerInterceptor()(context.Background(), nil, &grpc.UnaryServerInfo{FullMethod: "/agentenv.scheduler.v1.Scheduler/Schedule"}, handler)
	if err != nil || response != "ok" {
		t.Fatalf("leader RPC should pass: response=%v error=%v", response, err)
	}
}

func TestFollowerIsLiveButNotSchedulerReady(t *testing.T) {
	healthStatus := newSchedulerHealthStatus(false)

	process, err := healthStatus.server.Check(context.Background(), &grpc_health_v1.HealthCheckRequest{})
	if err != nil || process.GetStatus() != grpc_health_v1.HealthCheckResponse_SERVING {
		t.Fatalf("follower process health = %v, %v; want SERVING", process.GetStatus(), err)
	}
	scheduler, err := healthStatus.server.Check(context.Background(), &grpc_health_v1.HealthCheckRequest{Service: schedulerv1.Scheduler_ServiceDesc.ServiceName})
	if err != nil || scheduler.GetStatus() != grpc_health_v1.HealthCheckResponse_NOT_SERVING {
		t.Fatalf("follower scheduler health = %v, %v; want NOT_SERVING", scheduler.GetStatus(), err)
	}

	healthStatus.setSchedulerServing(true)
	scheduler, err = healthStatus.server.Check(context.Background(), &grpc_health_v1.HealthCheckRequest{Service: schedulerv1.Scheduler_ServiceDesc.ServiceName})
	if err != nil || scheduler.GetStatus() != grpc_health_v1.HealthCheckResponse_SERVING {
		t.Fatalf("leader scheduler health = %v, %v; want SERVING", scheduler.GetStatus(), err)
	}
}

func TestFenceSchedulerRejectsRPCsBeforeProcessStops(t *testing.T) {
	gate := newLeadershipGate(true)
	healthStatus := newSchedulerHealthStatus(true)
	fenceScheduler(gate, healthStatus)

	handlerCalled := false
	_, err := gate.unaryServerInterceptor()(context.Background(), nil, &grpc.UnaryServerInfo{FullMethod: "/scheduler.v1.Scheduler/Schedule"}, func(context.Context, any) (any, error) {
		handlerCalled = true
		return nil, nil
	})
	if status.Code(err) != codes.Unavailable || handlerCalled {
		t.Fatalf("fenced scheduler accepted RPC: handlerCalled=%v error=%v", handlerCalled, err)
	}
	process, err := healthStatus.server.Check(context.Background(), &grpc_health_v1.HealthCheckRequest{})
	if err != nil || process.GetStatus() != grpc_health_v1.HealthCheckResponse_SERVING {
		t.Fatalf("fencing changed process liveness: %v, %v", process.GetStatus(), err)
	}
	scheduler, err := healthStatus.server.Check(context.Background(), &grpc_health_v1.HealthCheckRequest{Service: schedulerv1.Scheduler_ServiceDesc.ServiceName})
	if err != nil || scheduler.GetStatus() != grpc_health_v1.HealthCheckResponse_NOT_SERVING {
		t.Fatalf("fenced scheduler readiness = %v, %v; want NOT_SERVING", scheduler.GetStatus(), err)
	}
}

func TestLeaderElectionDoesNotReleaseLeaseOnCancellation(t *testing.T) {
	cfg := validLeaderElectionConfig()
	election := leaderElectionConfig(zap.NewNop(), testLeaderElectionLock(cfg.Identity), cfg, func(context.Context) {}, func() {})
	if election.ReleaseOnCancel {
		t.Fatal("leader election must leave the Lease to expire after cancellation")
	}
}

func TestLeaderElectionUsesClientGoTimingValidation(t *testing.T) {
	cfg := validLeaderElectionConfig()
	cfg.RenewDeadline = 9 * time.Second
	cfg.RetryPeriod = 8 * time.Second

	_, err := newLeaderElector(zap.NewNop(), testLeaderElectionLock(cfg.Identity), cfg, func(context.Context) {}, func() {})
	if err == nil {
		t.Fatal("expected client-go to reject renewDeadline <= retryPeriod*JitterFactor")
	}
}

func testLeaderElectionLock(identity string) resourcelock.Interface {
	return &resourcelock.LeaseLock{LockConfig: resourcelock.ResourceLockConfig{Identity: identity}}
}

func validLeaderElectionConfig() config.SchedulerLeaderElectionConfig {
	return config.SchedulerLeaderElectionConfig{
		Enabled:        true,
		LeaseName:      "agentenv-scheduler",
		LeaseNamespace: "agentenv-system",
		Identity:       "scheduler-0",
		LeaseDuration:  15 * time.Second,
		RenewDeadline:  10 * time.Second,
		RetryPeriod:    2 * time.Second,
	}
}
