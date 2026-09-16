package handler

import (
	"context"
	"errors"
	"testing"

	"github.com/e2b-dev/infra/packages/envd/internal/processio"
)

type testResidency struct {
	freezeErr               error
	freezes, thaws, removes int
}

func (r *testResidency) SetFrozen(_ context.Context, frozen bool) error {
	if frozen {
		r.freezes++
		return r.freezeErr
	}
	r.thaws++
	return nil
}
func (r *testResidency) Remove() error { r.removes++; return r.freezeErr }

func TestHandoffUsesPhysicalBarrierAndRetainsExactInputReceipt(t *testing.T) {
	p, pipe := recoveringHandler(t)
	r := &testResidency{freezeErr: context.DeadlineExceeded}
	p.residency = r
	ctx := context.Background()
	if _, err := p.WriteRecoverable(1, input("before")); err != nil {
		t.Fatal(err)
	}
	if err := p.PrepareHandoff(ctx, 1, "move"); !errors.Is(err, context.DeadlineExceeded) {
		t.Fatal(err)
	}
	if _, err := p.WriteRecoverable(2, input("after")); !errors.Is(err, processio.ErrHandoff) {
		t.Fatal(err)
	}
	if _, err := p.WriteRecoverable(1, input("before")); err != nil || pipe.calls != 1 {
		t.Fatal(err, pipe.calls)
	}
	r.freezeErr = nil
	if err := p.PrepareHandoff(ctx, 1, "move"); err != nil {
		t.Fatal(err)
	}
	if err := p.ResumeHandoff(ctx, 1, "other"); !errors.Is(err, processio.ErrConflict) {
		t.Fatal(err)
	}
	if r.thaws != 0 {
		t.Fatal("conflicting operation thawed process")
	}
	if err := p.ResumeHandoff(ctx, 1, "move"); err != nil {
		t.Fatal(err)
	}
	if err := p.ResumeHandoff(ctx, 1, "move"); err != nil || r.thaws != 1 {
		t.Fatal(err, r.thaws)
	}
	if _, err := p.WriteRecoverable(2, input("after")); err != nil || pipe.calls != 2 {
		t.Fatal(err, pipe.calls)
	}
}

func TestHandoffCannotClaimPhysicalEvidenceWithoutBackend(t *testing.T) {
	p, _ := recoveringHandler(t)
	if err := p.PrepareHandoff(context.Background(), 1, "move"); !errors.Is(err, processio.ErrHandoff) {
		t.Fatal(err)
	}
	if err := p.ResumeHandoff(context.Background(), 1, "move"); !errors.Is(err, processio.ErrHandoff) {
		t.Fatal(err)
	}
}
