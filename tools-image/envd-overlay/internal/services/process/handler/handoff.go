package handler

import (
	"context"
	"fmt"

	"github.com/e2b-dev/infra/packages/envd/internal/processio"
)

type processResidency interface {
	SetFrozen(context.Context, bool) error
	Remove() error
}

func (p *Handler) PrepareHandoff(ctx context.Context, epoch uint64, operation string) error {
	if p.IO == nil || p.residency == nil {
		return fmt.Errorf("process has no physical residency barrier: %w", processio.ErrHandoff)
	}
	return p.IO.PrepareHandoff(p.Incarnation(), epoch, operation, func() error {
		return p.residency.SetFrozen(ctx, true)
	})
}

// ResumeHandoff requires the caller to have fenced the source residency and
// restored target credentials/filesystem. This does not itself grant authority.
func (p *Handler) ResumeHandoff(ctx context.Context, epoch uint64, operation string) error {
	if p.IO == nil || p.residency == nil {
		return fmt.Errorf("process has no physical residency barrier: %w", processio.ErrHandoff)
	}
	return p.IO.ResumeHandoff(p.Incarnation(), epoch, operation, func() error {
		return p.residency.SetFrozen(ctx, false)
	})
}

// CleanupResidency is separate from parent exit: a child can outlive its parent
// after closing inherited output. The service retains its capacity reservation
// until the kernel accepts removal of the entire empty process subtree.
func (p *Handler) CleanupResidency() error {
	if p.residency == nil {
		return nil
	}
	return p.residency.Remove()
}
