package processio

import (
	"errors"
	"sync/atomic"
	"testing"
)

func TestHandoffPreservesReplayAndOutputButBlocksNewMutation(t *testing.T) {
	j := journal(t)
	var writes int
	deliver := func(p []byte) (int, error) { writes++; return len(p), nil }
	before, err := j.Write(j.ID(), 1, "stdin", []byte("prompt"), deliver)
	if err != nil {
		t.Fatal(err)
	}
	if err = j.Append([]byte("output")); err != nil {
		t.Fatal(err)
	}
	if err = j.PrepareHandoff(j.ID(), 1, "move", func() error { return nil }); err != nil {
		t.Fatal(err)
	}
	replay, err := j.Write(j.ID(), 1, "stdin", []byte("prompt"), deliver)
	if err != nil || replay != before || writes != 1 {
		t.Fatalf("lost receipt: %+v %v %d", replay, err, writes)
	}
	if _, err = j.Write(j.ID(), 2, "stdin", []byte("next"), deliver); !errors.Is(err, ErrHandoff) {
		t.Fatal(err)
	}
	if err = j.CloseInput(j.ID(), func() error { t.Fatal("closed pipe during handoff"); return nil }); !errors.Is(err, ErrHandoff) {
		t.Fatal(err)
	}
	events, _, err := j.Read(j.ID(), 0)
	if err != nil || len(events) != 1 || events[0].Sequence != 1 || string(events[0].Payload) != "output" {
		t.Fatalf("lost output: %+v %v", events, err)
	}
	if err = j.ResumeHandoff(j.ID(), 1, "move", func() error { return nil }); err != nil {
		t.Fatal(err)
	}
	if _, err = j.Write(j.ID(), 2, "stdin", []byte("next"), deliver); err != nil || writes != 2 {
		t.Fatalf("input sequence changed: %v %d", err, writes)
	}
}

func TestHandoffWaitsForInFlightInput(t *testing.T) {
	j := journal(t)
	entered, release := make(chan struct{}), make(chan struct{})
	writeDone, freezeDone := make(chan error, 1), make(chan error, 1)
	var delivered atomic.Bool
	go func() {
		_, err := j.Write(j.ID(), 1, "stdin", []byte("prompt"), func(p []byte) (int, error) { close(entered); <-release; delivered.Store(true); return len(p), nil })
		writeDone <- err
	}()
	<-entered
	go func() {
		freezeDone <- j.PrepareHandoff(j.ID(), 1, "move", func() error {
			if !delivered.Load() {
				return errors.New("froze before input finished")
			}
			return nil
		})
	}()
	close(release)
	if err := <-writeDone; err != nil {
		t.Fatal(err)
	}
	if err := <-freezeDone; err != nil {
		t.Fatal(err)
	}
}

func TestHandoffFailuresAndStaleOperationsCannotOpenAdmission(t *testing.T) {
	j := journal(t)
	unknown := errors.New("unknown physical outcome")
	if err := j.PrepareHandoff(j.ID(), 1, "move", func() error { return unknown }); !errors.Is(err, unknown) {
		t.Fatal(err)
	}
	forbidden := func() error { t.Fatal("unexpected physical callback"); return nil }
	if err := j.ResumeHandoff(j.ID(), 1, "move", forbidden); !errors.Is(err, ErrHandoff) {
		t.Fatal(err)
	}
	if err := j.PrepareHandoff(j.ID(), 1, "different", forbidden); !errors.Is(err, ErrConflict) {
		t.Fatal(err)
	}
	if err := j.PrepareHandoff(j.ID(), 2, "second", forbidden); !errors.Is(err, ErrHandoff) {
		t.Fatal(err)
	}
	if err := j.PrepareHandoff(j.ID(), 1, "move", func() error { return nil }); err != nil {
		t.Fatal(err)
	}
	if err := j.PrepareHandoff(j.ID(), 1, "move", forbidden); err != nil {
		t.Fatal(err)
	}
	if err := j.ResumeHandoff(j.ID(), 1, "move", func() error { return unknown }); !errors.Is(err, unknown) {
		t.Fatal(err)
	}
	if _, err := j.Write(j.ID(), 1, "stdin", nil, func([]byte) (int, error) { t.Fatal("input reopened on unknown resume"); return 0, nil }); !errors.Is(err, ErrHandoff) {
		t.Fatal(err)
	}
	if err := j.PrepareHandoff(j.ID(), 1, "move", forbidden); !errors.Is(err, ErrHandoff) {
		t.Fatal(err)
	}
	if err := j.ResumeHandoff(j.ID(), 1, "move", func() error { return nil }); err != nil {
		t.Fatal(err)
	}
	if err := j.ResumeHandoff(j.ID(), 1, "move", forbidden); err != nil {
		t.Fatal(err)
	}
	if err := j.PrepareHandoff(j.ID(), 1, "move", forbidden); !errors.Is(err, ErrHandoff) {
		t.Fatal(err)
	}
	if err := j.PrepareHandoff(j.ID(), 2, "second", func() error { return nil }); err != nil {
		t.Fatal(err)
	}
	if err := j.ResumeHandoff(j.ID(), 1, "move", forbidden); !errors.Is(err, ErrConflict) {
		t.Fatal(err)
	}
}

func TestHandoffPanicKeepsInputClosed(t *testing.T) {
	j := journal(t)
	func() {
		defer func() {
			if recover() == nil {
				t.Error("expected backend panic")
			}
		}()
		_ = j.PrepareHandoff(j.ID(), 1, "move", func() error { panic("backend") })
	}()
	if _, err := j.Write(j.ID(), 1, "stdin", nil, func([]byte) (int, error) { t.Fatal("panic reopened input"); return 0, nil }); !errors.Is(err, ErrHandoff) {
		t.Fatal(err)
	}
}
