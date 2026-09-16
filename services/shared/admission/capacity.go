// Package admission owns scheduler capacity error identity shared by producer
// and gateway. Transport ResourceExhausted errors are not placement evidence.
package admission

import (
	"google.golang.org/genproto/googleapis/rpc/errdetails"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

const domain = "agentenv.scheduler"
const capacityReason = "CAPACITY_UNAVAILABLE"

func CapacityUnavailable(message string) error {
	result, err := status.New(codes.ResourceExhausted, message).WithDetails(&errdetails.ErrorInfo{Domain: domain, Reason: capacityReason})
	if err != nil {
		return status.Error(codes.Internal, "failed to encode capacity refusal")
	}
	return result.Err()
}

func IsCapacityUnavailable(err error) bool {
	result, ok := status.FromError(err)
	if !ok || result.Code() != codes.ResourceExhausted {
		return false
	}
	for _, detail := range result.Details() {
		if info, ok := detail.(*errdetails.ErrorInfo); ok && info.Domain == domain && info.Reason == capacityReason {
			return true
		}
	}
	return false
}
