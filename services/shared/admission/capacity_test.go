package admission

import (
	"google.golang.org/genproto/googleapis/rpc/errdetails"
	spb "google.golang.org/genproto/googleapis/rpc/status"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"google.golang.org/protobuf/proto"
	"testing"
)

func TestCapacityIdentitySurvivesWireWithoutTrustingCodeOrMessage(t *testing.T) {
	encoded, err := proto.Marshal(status.Convert(CapacityUnavailable("no nodes")).Proto())
	if err != nil {
		t.Fatal(err)
	}
	var wire spb.Status
	if err = proto.Unmarshal(encoded, &wire); err != nil {
		t.Fatal(err)
	}
	if !IsCapacityUnavailable(status.FromProto(&wire).Err()) {
		t.Fatal("capacity identity lost on wire")
	}
	foreign, err := status.New(codes.ResourceExhausted, "no nodes").WithDetails(&errdetails.ErrorInfo{Domain: "other.scheduler", Reason: capacityReason})
	if err != nil {
		t.Fatal(err)
	}
	for _, candidate := range []error{nil, status.Error(codes.ResourceExhausted, "no nodes"), status.Error(codes.Unavailable, "no nodes"), foreign.Err()} {
		if IsCapacityUnavailable(candidate) {
			t.Fatalf("non-capacity error accepted: %v", candidate)
		}
	}
}
