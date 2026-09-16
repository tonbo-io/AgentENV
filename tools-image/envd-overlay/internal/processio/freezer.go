package processio

import (
	"context"
	"fmt"
	"strings"
	"time"
)

// frozenEvent requires the kernel acknowledgement, not the requested state in
// cgroup.freeze. Additional event keys are allowed; missing or corrupt evidence
// cannot establish that the descendants have stopped.
func frozenEvent(events string) (bool, error) {
	found, frozen := false, false
	for _, line := range strings.Split(events, "\n") {
		fields := strings.Fields(line)
		if len(fields) == 0 || fields[0] != "frozen" {
			continue
		}
		if found || len(fields) != 2 || (fields[1] != "0" && fields[1] != "1") {
			return false, fmt.Errorf("invalid cgroup frozen acknowledgement")
		}
		found, frozen = true, fields[1] == "1"
	}
	if !found {
		return false, fmt.Errorf("missing cgroup frozen acknowledgement")
	}
	return frozen, nil
}

func awaitFrozen(ctx context.Context, want bool, read func() (string, error)) error {
	for {
		if err := ctx.Err(); err != nil {
			return err
		}
		events, err := read()
		if err != nil {
			return err
		}
		got, err := frozenEvent(events)
		if err != nil {
			return err
		}
		if got == want {
			return ctx.Err()
		}
		timer := time.NewTimer(5 * time.Millisecond)
		select {
		case <-ctx.Done():
			timer.Stop()
			return ctx.Err()
		case <-timer.C:
		}
	}
}
