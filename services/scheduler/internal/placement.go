package scheduler

import (
	"strings"
	"time"

	schedulerv1 "agentenv/services/api/proto"

	"go.uber.org/zap"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

const (
	maxPlacementReferences = 32
	maxSandboxIDLength     = 200
)

func (s *Service) filterPlacementCandidates(nodes []RichNode, hint *schedulerv1.ScheduleRequestHint, now time.Time) ([]RichNode, error) {
	newSandbox := hint.GetNewSandbox()
	if newSandbox == nil || newSandbox.GetPlacement() == nil {
		return nodes, nil
	}
	placement := newSandbox.GetPlacement()
	differentNodeFrom := placement.GetDifferentNodeFrom()
	snapshotCompatibleWith := placement.GetSnapshotCompatibleWith()
	if len(differentNodeFrom) == 0 && len(snapshotCompatibleWith) == 0 {
		return nodes, nil
	}
	if err := validatePlacementReferences("different_node_from", differentNodeFrom); err != nil {
		return nil, err
	}
	if err := validatePlacementReferences("snapshot_compatible_with", snapshotCompatibleWith); err != nil {
		return nil, err
	}

	referenceIDs := make([]string, 0, len(differentNodeFrom)+len(snapshotCompatibleWith))
	seenReferenceIDs := make(map[string]struct{}, cap(referenceIDs))
	for _, references := range [][]string{differentNodeFrom, snapshotCompatibleWith} {
		for _, reference := range references {
			if _, seen := seenReferenceIDs[reference]; seen {
				continue
			}
			seenReferenceIDs[reference] = struct{}{}
			referenceIDs = append(referenceIDs, reference)
		}
	}

	referenceNodes := make(map[string]*schedulerv1.ObservedNode, len(referenceIDs))
	for _, reference := range referenceIDs {
		referenceNode, ok, err := s.store.Get(reference, now)
		if err != nil {
			s.logger.Warn("scheduler placement reference lookup failed", zap.String("sandbox_id", reference), zap.Error(err))
			return nil, status.Error(codes.Unavailable, "binding store unavailable")
		}
		if !ok {
			return nil, status.Error(codes.FailedPrecondition, "placement reference assignment not found")
		}
		referenceObserved, ok := s.nodes.GetObserved(referenceNode.ID, "", now)
		if !ok || !isLivePlacementReference(referenceObserved.GetSnapshot().GetStatus()) {
			return nil, status.Error(codes.FailedPrecondition, "placement reference node is not live")
		}
		referenceNodes[reference] = referenceObserved
	}

	excludedNodes := make(map[string]struct{}, len(differentNodeFrom))
	for _, reference := range differentNodeFrom {
		excludedNodes[referenceNodes[reference].GetNodeId()] = struct{}{}
	}
	compatibilitySources := make([]*schedulerv1.ObservedNode, 0, len(snapshotCompatibleWith))
	for _, reference := range snapshotCompatibleWith {
		compatibilitySources = append(compatibilitySources, referenceNodes[reference])
	}

	compatible := make([]RichNode, 0, len(nodes))
	for _, candidate := range nodes {
		if _, excluded := excludedNodes[candidate.ID]; excluded {
			continue
		}
		if len(compatibilitySources) == 0 {
			compatible = append(compatible, candidate)
			continue
		}
		observed, ok := s.nodes.GetObserved(candidate.ID, compatibilitySources[0].GetClusterId(), now)
		if !ok || observed.GetSnapshot().GetStatus() != schedulerv1.NodeStatus_NODE_STATUS_READY {
			continue
		}
		matchesAll := true
		for _, source := range compatibilitySources {
			if !sameSnapshotCompatibilityDomain(source, observed) {
				matchesAll = false
				break
			}
		}
		if !matchesAll {
			continue
		}
		compatible = append(compatible, candidate)
	}
	if len(compatible) == 0 {
		if len(compatibilitySources) > 0 {
			return nil, status.Error(codes.Unavailable, "no snapshot-compatible placement target available")
		}
		return nil, status.Error(codes.Unavailable, "no placement target available")
	}
	return compatible, nil
}

func validatePlacementReferences(field string, references []string) error {
	if len(references) > maxPlacementReferences {
		return status.Errorf(codes.InvalidArgument, "%s exceeds the limit of %d sandbox ids", field, maxPlacementReferences)
	}
	seenReferences := make(map[string]struct{}, len(references))
	for _, reference := range references {
		normalizedReference := strings.TrimSpace(reference)
		if normalizedReference == "" || normalizedReference != reference {
			return status.Errorf(codes.InvalidArgument, "%s contains an invalid sandbox id", field)
		}
		if len(reference) > maxSandboxIDLength {
			return status.Errorf(codes.InvalidArgument, "%s sandbox id exceeds %d bytes", field, maxSandboxIDLength)
		}
		if _, duplicate := seenReferences[reference]; duplicate {
			return status.Errorf(codes.InvalidArgument, "%s contains a duplicate sandbox id", field)
		}
		seenReferences[reference] = struct{}{}
	}
	return nil
}

func isLivePlacementReference(status schedulerv1.NodeStatus) bool {
	return status == schedulerv1.NodeStatus_NODE_STATUS_READY || status == schedulerv1.NodeStatus_NODE_STATUS_LINGERING
}

// sameSnapshotCompatibilityDomain is intentionally conservative. The target
// runtime still validates the snapshot itself before it can become ready.
func sameSnapshotCompatibilityDomain(source, target *schedulerv1.ObservedNode) bool {
	if source == nil || target == nil || source.GetClusterId() == "" || source.GetClusterId() != target.GetClusterId() {
		return false
	}
	if source.GetVersion() == "" || source.GetVersion() != target.GetVersion() || source.GetCommit() == "" || source.GetCommit() != target.GetCommit() {
		return false
	}
	sourceMachine, targetMachine := source.GetMachineInfo(), target.GetMachineInfo()
	if sourceMachine == nil || targetMachine == nil || sourceMachine.GetCpuArchitecture() == "" || sourceMachine.GetCpuFamily() == "" || sourceMachine.GetCpuModel() == "" || sourceMachine.GetCpuConfigJson() == "" {
		return false
	}
	return sourceMachine.GetCpuArchitecture() == targetMachine.GetCpuArchitecture() &&
		sourceMachine.GetCpuFamily() == targetMachine.GetCpuFamily() &&
		sourceMachine.GetCpuModel() == targetMachine.GetCpuModel() &&
		sourceMachine.GetCpuConfigJson() == targetMachine.GetCpuConfigJson()
}
