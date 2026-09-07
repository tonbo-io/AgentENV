package nodecompatibility

import (
	schedulerv1 "agentenv/services/api/proto"
	"google.golang.org/protobuf/proto"
	"testing"
)

func TestKeyRequiresCompleteIdentityAndSeparatesSnapshotDomains(t *testing.T) {
	n := &schedulerv1.ObservedNode{ClusterId: "cluster", Version: "version", Commit: "commit", MachineInfo: &schedulerv1.MachineInfo{CpuArchitecture: "arm64", CpuFamily: "family", CpuModel: "model", CpuConfigJson: "config"}}
	key := Key(n)
	if key == "" || Key(proto.Clone(n).(*schedulerv1.ObservedNode)) != key {
		t.Fatal("matching domain lost")
	}
	for _, field := range []string{"cluster", "version", "commit", "architecture", "family", "model", "config"} {
		for _, value := range []string{"", "changed"} {
			copy := proto.Clone(n).(*schedulerv1.ObservedNode)
			switch field {
			case "cluster":
				copy.ClusterId = value
			case "version":
				copy.Version = value
			case "commit":
				copy.Commit = value
			case "architecture":
				copy.MachineInfo.CpuArchitecture = value
			case "family":
				copy.MachineInfo.CpuFamily = value
			case "model":
				copy.MachineInfo.CpuModel = value
			case "config":
				copy.MachineInfo.CpuConfigJson = value
			}
			got := Key(copy)
			if got == key || (value == "" && got != "") {
				t.Fatalf("domain did not fail closed: %s=%q", field, value)
			}
		}
	}
	if Key(nil) != "" {
		t.Fatal("nil identity accepted")
	}
}
