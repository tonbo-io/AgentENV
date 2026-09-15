package logs

import (
	"bytes"
	"context"
	"net/http/httptest"
	"strings"
	"testing"

	"connectrpc.com/connect"
	rpc "github.com/e2b-dev/infra/packages/envd/internal/services/spec/process"
	"github.com/rs/zerolog"
)

func TestProcessInputPayloadIsNotLogged(t *testing.T) {
	var output bytes.Buffer
	logger := zerolog.New(&output)
	const method = "/process.Process/SendInput"
	handler := connect.NewUnaryHandler(method,
		func(ctx context.Context, request *connect.Request[rpc.SendInputRequest]) (*connect.Response[rpc.SendInputResponse], error) {
			return connect.NewResponse(&rpc.SendInputResponse{Sha256: "test-receipt"}), nil
		}, connect.WithInterceptors(NewUnaryLogInterceptor(&logger)))
	server := httptest.NewServer(handler)
	defer server.Close()
	client := connect.NewClient[rpc.SendInputRequest, rpc.SendInputResponse](server.Client(), server.URL+method)
	_, err := client.CallUnary(context.Background(), connect.NewRequest(&rpc.SendInputRequest{
		Input: &rpc.ProcessInput{Input: &rpc.ProcessInput_Stdin{Stdin: []byte("private-prompt-sentinel")}},
	}))
	if err != nil {
		t.Fatal(err)
	}
	text := output.String()
	if !strings.Contains(text, method) || strings.Contains(text, `"request"`) || strings.Contains(text, `"response"`) || strings.Contains(text, "private-prompt") {
		t.Fatalf("unsafe or absent process operation log: %s", text)
	}
}

func TestProcessStartPayloadIsNotLogged(t *testing.T) {
	var output bytes.Buffer
	logger := zerolog.New(&output)
	const method = "/process.Process/Start"
	handler := connect.NewServerStreamHandler(method,
		func(ctx context.Context, request *connect.Request[rpc.StartRequest], stream *connect.ServerStream[rpc.StartResponse]) error {
			return LogServerStreamWithoutEvents(ctx, &logger, request, stream,
				func(context.Context, *connect.Request[rpc.StartRequest], *connect.ServerStream[rpc.StartResponse]) error {
					return nil
				})
		})
	server := httptest.NewServer(handler)
	defer server.Close()
	client := connect.NewClient[rpc.StartRequest, rpc.StartResponse](server.Client(), server.URL+method)
	stream, err := client.CallServerStream(context.Background(), connect.NewRequest(&rpc.StartRequest{
		Process:       &rpc.ProcessConfig{Cmd: "private-command-sentinel", Envs: map[string]string{"TOKEN": "private-env-sentinel"}},
		RecoverableIo: true,
	}))
	if err != nil {
		t.Fatal(err)
	}
	for stream.Receive() {
	}
	if err := stream.Err(); err != nil {
		t.Fatal(err)
	}
	text := output.String()
	if !strings.Contains(text, method) || strings.Contains(text, `"request"`) || strings.Contains(text, "private-") {
		t.Fatalf("unsafe or absent start operation log: %s", text)
	}
}
