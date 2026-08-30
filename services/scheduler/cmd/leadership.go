package main

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"sync/atomic"
	"time"

	schedulerv1 "agentenv/services/api/proto"
	"agentenv/services/shared/config"

	"go.uber.org/zap"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/health"
	"google.golang.org/grpc/health/grpc_health_v1"
	"google.golang.org/grpc/status"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/tools/leaderelection"
	"k8s.io/client-go/tools/leaderelection/resourcelock"
)

const leadershipPublishTimeout = 5 * time.Second

type leadershipPublisher interface {
	setLeading(context.Context, bool) error
}

type podPatcher interface {
	Patch(context.Context, string, types.PatchType, []byte, metav1.PatchOptions, ...string) (*corev1.Pod, error)
}

type podLabelLeadershipPublisher struct {
	pods       podPatcher
	podName    string
	labelKey   string
	labelValue string
}

func newPodLabelLeadershipPublisher(client kubernetes.Interface, cfg config.SchedulerLeaderElectionConfig) leadershipPublisher {
	if strings.TrimSpace(cfg.ServiceLabelKey) == "" {
		return nil
	}
	return &podLabelLeadershipPublisher{
		pods:       client.CoreV1().Pods(cfg.LeaseNamespace),
		podName:    cfg.Identity,
		labelKey:   cfg.ServiceLabelKey,
		labelValue: cfg.ServiceLabelValue,
	}
}

func (p *podLabelLeadershipPublisher) setLeading(parent context.Context, leading bool) error {
	if p == nil || p.pods == nil {
		return fmt.Errorf("Kubernetes Pod client is required to publish scheduler leadership")
	}
	value := any(nil)
	if leading {
		value = p.labelValue
	}
	payload, err := json.Marshal(map[string]any{
		"metadata": map[string]any{
			"labels": map[string]any{p.labelKey: value},
		},
	})
	if err != nil {
		return fmt.Errorf("marshal scheduler leadership Pod label: %w", err)
	}
	ctx, cancel := context.WithTimeout(parent, leadershipPublishTimeout)
	defer cancel()
	if _, err := p.pods.Patch(ctx, p.podName, types.MergePatchType, payload, metav1.PatchOptions{}); err != nil {
		return fmt.Errorf("publish scheduler leadership Pod label: %w", err)
	}
	return nil
}

type leadershipGate struct {
	leader atomic.Bool
}

func newLeadershipGate(initialLeader bool) *leadershipGate {
	gate := &leadershipGate{}
	gate.leader.Store(initialLeader)
	return gate
}

func (g *leadershipGate) setLeader(leader bool) {
	g.leader.Store(leader)
}

func (g *leadershipGate) unaryServerInterceptor() grpc.UnaryServerInterceptor {
	return func(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (any, error) {
		if strings.HasPrefix(info.FullMethod, "/grpc.health.v1.Health/") || g.leader.Load() {
			return handler(ctx, req)
		}
		return nil, status.Error(codes.Unavailable, "scheduler replica is not leader")
	}
}

type schedulerHealthStatus struct {
	server *health.Server
}

func newSchedulerHealthStatus(schedulerServing bool) *schedulerHealthStatus {
	status := &schedulerHealthStatus{server: health.NewServer()}
	status.setProcessServing(true)
	status.setSchedulerServing(schedulerServing)
	return status
}

func (s *schedulerHealthStatus) setProcessServing(serving bool) {
	s.server.SetServingStatus("", servingStatus(serving))
}

func (s *schedulerHealthStatus) setSchedulerServing(serving bool) {
	s.server.SetServingStatus(schedulerv1.Scheduler_ServiceDesc.ServiceName, servingStatus(serving))
}

func servingStatus(serving bool) grpc_health_v1.HealthCheckResponse_ServingStatus {
	if serving {
		return grpc_health_v1.HealthCheckResponse_SERVING
	}
	return grpc_health_v1.HealthCheckResponse_NOT_SERVING
}

func fenceScheduler(gate *leadershipGate, healthStatus *schedulerHealthStatus) {
	gate.setLeader(false)
	healthStatus.setSchedulerServing(false)
}

func leaderElectionConfig(
	logger *zap.Logger,
	lock resourcelock.Interface,
	cfg config.SchedulerLeaderElectionConfig,
	onStartedLeading func(context.Context),
	onStoppedLeading func(),
) leaderelection.LeaderElectionConfig {
	return leaderelection.LeaderElectionConfig{
		Lock:          lock,
		LeaseDuration: cfg.LeaseDuration,
		RenewDeadline: cfg.RenewDeadline,
		RetryPeriod:   cfg.RetryPeriod,
		// Do not release the Lease while this process may still be handling an
		// RPC. Graceful shutdown closes the leadership gate before cancelling
		// election, and the next leader waits for the Lease to expire.
		ReleaseOnCancel: false,
		Name:            "agentenv-scheduler",
		Callbacks: leaderelection.LeaderCallbacks{
			OnStartedLeading: onStartedLeading,
			OnStoppedLeading: onStoppedLeading,
			OnNewLeader: func(identity string) {
				if identity == cfg.Identity {
					logger.Info("scheduler leadership acquired", zap.String("identity", identity))
					return
				}
				logger.Info("scheduler leader observed", zap.String("identity", identity))
			},
		},
	}
}

func newLeaderElectionLock(client kubernetes.Interface, cfg config.SchedulerLeaderElectionConfig) resourcelock.Interface {
	return &resourcelock.LeaseLock{
		LeaseMeta: metav1.ObjectMeta{
			Name:      cfg.LeaseName,
			Namespace: cfg.LeaseNamespace,
		},
		Client: client.CoordinationV1(),
		LockConfig: resourcelock.ResourceLockConfig{
			Identity: cfg.Identity,
		},
	}
}

func newLeaderElector(
	logger *zap.Logger,
	lock resourcelock.Interface,
	cfg config.SchedulerLeaderElectionConfig,
	onStartedLeading func(context.Context),
	onStoppedLeading func(),
) (*leaderelection.LeaderElector, error) {
	// NewLeaderElector is the authoritative client-go validation path. Keep
	// timing validation here instead of maintaining a partial copy in config.
	return leaderelection.NewLeaderElector(leaderElectionConfig(
		logger,
		lock,
		cfg,
		onStartedLeading,
		onStoppedLeading,
	))
}
