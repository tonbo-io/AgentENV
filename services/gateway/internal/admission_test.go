package gateway

import (
	"context"
	"errors"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	schedulerv1 "agentenv/services/api/proto"
	"go.uber.org/zap"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

type admissionTransport func(*http.Request) (*http.Response, error)

func (f admissionTransport) RoundTrip(r *http.Request) (*http.Response, error) { return f(r) }

func TestCreateAdmissionRefusalIsPreDispatchOnly(t *testing.T) {
	for _, path := range []string{"/sandboxes", "/sandboxes-cold", "/sandboxes-cold/"} {
		for _, tc := range []struct {
			name   string
			err    error
			reason string
		}{
			{"capacity", status.Error(codes.ResourceExhausted, "no eligible nodes"), "capacity_unavailable"},
			{"scheduler transport", status.Error(codes.Unavailable, "no nodes available"), "scheduler_error"},
			{"deadline", status.Error(codes.DeadlineExceeded, "deadline"), "scheduler_error"},
		} {
			t.Run(path+tc.name, func(t *testing.T) {
				srv, err := NewServer(zap.NewNop(), stubSchedulerClient{scheduleFunc: func(context.Context, *schedulerv1.ScheduleRequest, ...grpc.CallOption) (*schedulerv1.ScheduleResponse, error) {
					return nil, tc.err
				}}, ServerOptions{APIKey: "test", RequestTimeout: time.Second})
				if err != nil {
					t.Fatal(err)
				}
				srv.proxyTransport = admissionTransport(func(*http.Request) (*http.Response, error) {
					t.Fatal("refused request was dispatched")
					return nil, errors.New("unexpected dispatch")
				})
				req := httptest.NewRequest(http.MethodPost, path, strings.NewReader(`{"image":"example"}`))
				req.Header.Set(headerAPIKey, "test")
				response := httptest.NewRecorder()
				srv.Handler().ServeHTTP(response, req)
				if response.Header().Get(headerDispatchOutcome) != "not_dispatched" || response.Header().Get(headerAdmissionReason) != tc.reason {
					t.Fatalf("unexpected refusal: %d %v", response.Code, response.Header())
				}
				if tc.reason == "capacity_unavailable" && response.Code != http.StatusServiceUnavailable {
					t.Fatalf("capacity status %d", response.Code)
				}
			})
		}
	}
}

func TestDispatchedCreateNeverCarriesNotDispatchedReceipt(t *testing.T) {
	for _, failure := range []bool{false, true} {
		t.Run(map[bool]string{false: "runtime 503", true: "lost response"}[failure], func(t *testing.T) {
			srv, err := NewServer(zap.NewNop(), stubSchedulerClient{scheduleFunc: func(context.Context, *schedulerv1.ScheduleRequest, ...grpc.CallOption) (*schedulerv1.ScheduleResponse, error) {
				return &schedulerv1.ScheduleResponse{Node: &schedulerv1.Node{NodeId: "node", Endpoint: "http://runtime"}}, nil
			}}, ServerOptions{APIKey: "test", RequestTimeout: time.Second})
			if err != nil {
				t.Fatal(err)
			}
			dispatched := 0
			srv.proxyTransport = admissionTransport(func(r *http.Request) (*http.Response, error) {
				dispatched++
				if failure {
					return nil, errors.New("response lost after dispatch")
				}
				return &http.Response{StatusCode: 503, Header: http.Header{http.CanonicalHeaderKey(headerDispatchOutcome): {"not_dispatched"}, http.CanonicalHeaderKey(headerAdmissionReason): {"capacity_unavailable"}}, Body: io.NopCloser(strings.NewReader("no nodes available")), Request: r}, nil
			})
			req := httptest.NewRequest(http.MethodPost, "/sandboxes-cold", strings.NewReader(`{"image":"example"}`))
			req.Header.Set(headerAPIKey, "test")
			response := httptest.NewRecorder()
			srv.Handler().ServeHTTP(response, req)
			if dispatched != 1 || response.Header().Get(headerDispatchOutcome) != "" || response.Header().Get(headerAdmissionReason) != "" {
				t.Fatalf("ambiguous dispatch was certified: count=%d headers=%v", dispatched, response.Header())
			}
		})
	}
}

func TestNonCreateDoesNotCarryAdmissionReceipt(t *testing.T) {
	srv, err := NewServer(zap.NewNop(), stubSchedulerClient{scheduleFunc: func(context.Context, *schedulerv1.ScheduleRequest, ...grpc.CallOption) (*schedulerv1.ScheduleResponse, error) {
		return nil, status.Error(codes.ResourceExhausted, "no nodes")
	}}, ServerOptions{APIKey: "test", RequestTimeout: time.Second})
	if err != nil {
		t.Fatal(err)
	}
	req := httptest.NewRequest(http.MethodGet, "/templates", nil)
	req.Header.Set(headerAPIKey, "test")
	response := httptest.NewRecorder()
	srv.Handler().ServeHTTP(response, req)
	if response.Header().Get(headerDispatchOutcome) != "" || response.Header().Get(headerAdmissionReason) != "" {
		t.Fatalf("non-create admission receipt: %v", response.Header())
	}
}
