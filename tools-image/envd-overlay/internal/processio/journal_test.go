package processio

import (
	"errors"
	"io"
	"sync"
	"sync/atomic"
	"testing"
)

func journal(t *testing.T) *Journal {
	t.Helper()
	j, err := New()
	if err != nil {
		t.Fatal(err)
	}
	return j
}

func TestLostAcknowledgementAndConcurrentRetriesWriteOnce(t *testing.T) {
	j := journal(t)
	var writes atomic.Int32
	started, release := make(chan struct{}), make(chan struct{})
	deliver := func(b []byte) (int, error) { writes.Add(1); close(started); <-release; return len(b), nil }
	var wg sync.WaitGroup
	for range 20 {
		wg.Add(1)
		go func() {
			defer wg.Done()
			r, err := j.Write(j.ID(), 1, "stdin", []byte("prompt\n"), deliver)
			if err != nil || r.Sequence != 1 || r.Written != 7 {
				t.Errorf("receipt=%+v error=%v", r, err)
			}
		}()
	}
	<-started
	close(release)
	wg.Wait()
	if writes.Load() != 1 {
		t.Fatal(writes.Load())
	}
}

func TestInputIdentityOrderConflictAndExpiration(t *testing.T) {
	j := journal(t)
	calls := 0
	deliver := func(b []byte) (int, error) { calls++; return len(b), nil }
	for _, c := range []struct {
		id      string
		seq     uint64
		channel string
		err     error
	}{
		{journal(t).ID(), 1, "stdin", ErrIncarnation},
		{j.ID(), 0, "stdin", ErrSequence}, {j.ID(), 2, "stdin", ErrSequence}, {j.ID(), 1, "other", ErrSequence},
	} {
		if _, err := j.Write(c.id, c.seq, c.channel, []byte("a"), deliver); !errors.Is(err, c.err) {
			t.Fatalf("%+v: %v", c, err)
		}
	}
	if calls != 0 {
		t.Fatal(calls)
	}
	first, err := j.Write(j.ID(), 1, "stdin", []byte("a"), deliver)
	if err != nil {
		t.Fatal(err)
	}
	for _, c := range []struct{ channel, payload string }{{"stdin", "b"}, {"pty", "a"}} {
		if _, err := j.Write(j.ID(), 1, c.channel, []byte(c.payload), deliver); !errors.Is(err, ErrConflict) {
			t.Fatal(err)
		}
	}
	replay, err := j.Write(j.ID(), 1, "stdin", []byte("a"), deliver)
	if err != nil || replay != first || calls != 1 {
		t.Fatalf("%+v %v %d", replay, err, calls)
	}
	for seq := uint64(2); seq <= MaxReceipts+1; seq++ {
		if _, err := j.Write(j.ID(), seq, "stdin", nil, deliver); err != nil {
			t.Fatal(err)
		}
	}
	if len(j.inputs) != MaxReceipts {
		t.Fatal(len(j.inputs))
	}
	if _, err := j.Write(j.ID(), 1, "stdin", []byte("a"), deliver); !errors.Is(err, ErrExpired) {
		t.Fatal(err)
	}
}

func TestPartialWriteAndErrorsPermanentlyFenceInput(t *testing.T) {
	for _, result := range []struct {
		n   int
		err error
	}{{1, nil}, {0, io.ErrClosedPipe}, {2, io.ErrClosedPipe}} {
		j := journal(t)
		calls := 0
		deliver := func([]byte) (int, error) { calls++; return result.n, result.err }
		first, err := j.Write(j.ID(), 1, "stdin", []byte("ab"), deliver)
		if !errors.Is(err, ErrUncertain) || !first.Uncertain {
			t.Fatalf("%+v %v", first, err)
		}
		again, err := j.Write(j.ID(), 1, "stdin", []byte("ab"), deliver)
		if !errors.Is(err, ErrUncertain) || again != first {
			t.Fatalf("%+v %v", again, err)
		}
		if _, err := j.Write(j.ID(), 2, "stdin", nil, deliver); !errors.Is(err, ErrUncertain) {
			t.Fatal(err)
		}
		if calls != 1 {
			t.Fatal(calls)
		}
	}
}

func TestDeliveryPanicCannotMakeAWriteRetryable(t *testing.T) {
	j := journal(t)
	func() {
		defer func() {
			if recover() == nil {
				t.Error("expected injected panic")
			}
		}()
		_, _ = j.Write(j.ID(), 1, "stdin", nil, func([]byte) (int, error) { panic("after delivery") })
	}()
	if _, err := j.Write(j.ID(), 1, "stdin", nil, func([]byte) (int, error) { t.Fatal("replayed uncertain delivery"); return 0, nil }); !errors.Is(err, ErrUncertain) {
		t.Fatal(err)
	}
}

func TestClosedInputRetainsReceiptsAndRejectsNewWrites(t *testing.T) {
	j := journal(t)
	deliver := func(b []byte) (int, error) { return len(b), nil }
	first, err := j.Write(j.ID(), 1, "stdin", nil, deliver)
	if err != nil {
		t.Fatal(err)
	}
	closes := 0
	closePipe := func() error { closes++; return nil }
	if err := j.CloseInput("wrong", closePipe); !errors.Is(err, ErrIncarnation) {
		t.Fatal(err)
	}
	if err := j.CloseInput(j.ID(), closePipe); err != nil {
		t.Fatal(err)
	}
	if err := j.CloseInput(j.ID(), closePipe); err != nil || closes != 1 {
		t.Fatal(err, closes)
	}
	again, err := j.Write(j.ID(), 1, "stdin", nil, deliver)
	if err != nil || again != first {
		t.Fatal(again, err)
	}
	if _, err := j.Write(j.ID(), 2, "stdin", nil, deliver); !errors.Is(err, ErrClosed) {
		t.Fatal(err)
	}
}

func TestOutputReplayNotificationOwnershipAndEnd(t *testing.T) {
	j := journal(t)
	events, changed, err := j.Read(j.ID(), 0)
	if err != nil || len(events) != 0 {
		t.Fatal(events, err)
	}
	payload := []byte("hello")
	if err := j.Append(payload); err != nil {
		t.Fatal(err)
	}
	payload[0] = 'x'
	select {
	case <-changed:
	default:
		t.Fatal("missed notification")
	}
	events, _, err = j.Read(j.ID(), 0)
	if err != nil || len(events) != 1 || events[0].Sequence != 1 || string(events[0].Payload) != "hello" {
		t.Fatal(events, err)
	}
	events[0].Payload[0] = 'x'
	again, _, err := j.Read(j.ID(), 0)
	if err != nil || string(again[0].Payload) != "hello" {
		t.Fatal(again, err)
	}
	if _, _, err := j.Read("wrong", 0); !errors.Is(err, ErrIncarnation) {
		t.Fatal(err)
	}
	if _, _, err := j.Read(j.ID(), 2); !errors.Is(err, ErrSequence) {
		t.Fatal(err)
	}
	_, changed, err = j.Read(j.ID(), 1)
	if err != nil {
		t.Fatal(err)
	}
	j.CloseOutput()
	select {
	case <-changed:
	default:
		t.Fatal("missed close")
	}
	if _, _, err := j.Read(j.ID(), 1); !errors.Is(err, io.EOF) {
		t.Fatal(err)
	}
	if err := j.Append(nil); !errors.Is(err, ErrClosed) {
		t.Fatal(err)
	}
}

func TestSlowOutputReaderExpiresWithoutBlockingProducer(t *testing.T) {
	j := journal(t)
	for range MaxOutputEvents + 1 {
		if err := j.Append(nil); err != nil {
			t.Fatal(err)
		}
	}
	if len(j.output) != MaxOutputEvents {
		t.Fatal(len(j.output))
	}
	if _, _, err := j.Read(j.ID(), 0); !errors.Is(err, ErrExpired) {
		t.Fatal(err)
	}
	if events, _, err := j.Read(j.ID(), 1); err != nil || len(events) != MaxOutputEvents {
		t.Fatal(len(events), err)
	}
	j = journal(t)
	payload := make([]byte, MaxOutputBytes/2)
	for range 3 {
		if err := j.Append(payload); err != nil {
			t.Fatal(err)
		}
	}
	if j.outputBytes != MaxOutputBytes || len(j.output) != 2 {
		t.Fatal(j.outputBytes, len(j.output))
	}
	if _, _, err := j.Read(j.ID(), 0); !errors.Is(err, ErrExpired) {
		t.Fatal(err)
	}
}

func TestLimitsNeverAllowSilentOutputLossOrInputDelivery(t *testing.T) {
	j := journal(t)
	if _, err := j.Write(j.ID(), 1, "stdin", make([]byte, MaxInputBytes+1), func([]byte) (int, error) { t.Fatal("oversized input delivered"); return 0, nil }); !errors.Is(err, ErrLimit) {
		t.Fatal(err)
	}
	if err := j.Append(make([]byte, MaxOutputBytes+1)); !errors.Is(err, ErrLimit) {
		t.Fatal(err)
	}
	if _, _, err := j.Read(j.ID(), 0); !errors.Is(err, ErrExpired) {
		t.Fatal(err)
	}
}

func TestCloseFailureIsReplayed(t *testing.T) {
	j, err := New()
	if err != nil {
		t.Fatal(err)
	}
	calls := 0
	closePipe := func() error { calls++; return io.ErrClosedPipe }
	for i := 0; i < 2; i++ {
		if err := j.CloseInput(j.ID(), closePipe); !errors.Is(err, io.ErrClosedPipe) {
			t.Fatal(err)
		}
	}
	if calls != 1 {
		t.Fatal(calls)
	}
}
