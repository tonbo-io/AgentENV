package processio

import (
	"context"
	"errors"
	"testing"
)

func TestFrozenEvidenceRejectsMissingOrAmbiguousAcknowledgement(t *testing.T) {
	for _, input := range []string{"", "populated 1\n", "frozen 2\n", "frozen 1\nfrozen 0\n", "frozen 1 extra"} {
		if _, err := frozenEvent(input); err == nil {
			t.Fatalf("accepted %q", input)
		}
	}
	for _, state := range []string{"0", "1"} {
		got, err := frozenEvent("populated 1\nfrozen " + state + "\nfuture_event 0\n")
		if err != nil || got != (state == "1") {
			t.Fatalf("state %s: %v %v", state, got, err)
		}
	}
}

func TestFreezeWaitRequiresAcknowledgementAndKeepsCancellation(t *testing.T) {
	calls := 0
	if err := awaitFrozen(context.Background(), true, func() (string, error) {
		calls++
		if calls < 3 {
			return "frozen 0\n", nil
		}
		return "frozen 1\n", nil
	}); err != nil || calls != 3 {
		t.Fatalf("early acknowledgement: calls=%d err=%v", calls, err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	err := awaitFrozen(ctx, false, func() (string, error) {
		cancel()
		return "frozen 0\n", nil
	})
	if !errors.Is(err, context.Canceled) {
		t.Fatalf("lost cancellation: %v", err)
	}
	want := errors.New("cgroup disappeared")
	if err := awaitFrozen(context.Background(), true, func() (string, error) { return "", want }); !errors.Is(err, want) {
		t.Fatalf("lost kernel read error: %v", err)
	}
}
