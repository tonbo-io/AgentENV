// Package nodecompatibility owns the conservative snapshot placement domain.
package nodecompatibility

import (
	schedulerv1 "agentenv/services/api/proto"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
)

// Key is opaque to placement consumers. Missing identity fails closed.
func Key(node *schedulerv1.ObservedNode) string {
	if node == nil || node.GetMachineInfo() == nil {
		return ""
	}
	m := node.GetMachineInfo()
	fields := []string{node.GetClusterId(), node.GetVersion(), node.GetCommit(), m.GetCpuArchitecture(), m.GetCpuFamily(), m.GetCpuModel(), m.GetCpuConfigJson()}
	for _, field := range fields {
		if field == "" {
			return ""
		}
	}
	encoded, _ := json.Marshal(fields)
	sum := sha256.Sum256(encoded)
	return "snapshot-v1:" + hex.EncodeToString(sum[:])
}
