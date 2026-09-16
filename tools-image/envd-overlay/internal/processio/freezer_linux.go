package processio

import (
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"regexp"
	"sync"
	"syscall"
	"time"
)

// ProcessGroup is a per-incarnation child of envd's existing process-type
// cgroup. Create it before starting the process and pass FD to CLONE_INTO_CGROUP;
// moving an already running PID would leave a fork race. Descendants inherit
// membership even when they create a new session. This is not a security fence
// against a privileged guest deliberately moving itself to another cgroup.
type ProcessGroup struct {
	mu        sync.Mutex
	parent    *os.Root
	root      *os.Root
	directory *os.File
	name      string
}

var groupIncarnation = regexp.MustCompile(`^[0-9a-f]{64}$`)

func NewProcessGroup(parentFD int, incarnation string) (*ProcessGroup, error) {
	if !groupIncarnation.MatchString(incarnation) {
		return nil, ErrIncarnation
	}
	parent, err := os.OpenRoot(fmt.Sprintf("/proc/self/fd/%d", parentFD))
	if err != nil {
		return nil, err
	}
	// Inspect our owned descriptor so a missing cgroup manager cannot silently
	// substitute a normal directory or the manager's shared process group.
	probe, err := parent.Open(".")
	if err != nil {
		parent.Close()
		return nil, err
	}
	var stat syscall.Statfs_t
	err = syscall.Fstatfs(int(probe.Fd()), &stat)
	probe.Close()
	if err != nil || stat.Type != 0x63677270 { // CGROUP2_SUPER_MAGIC
		parent.Close()
		return nil, errors.Join(fmt.Errorf("process handoff requires cgroup v2"), err)
	}
	name := "process-" + incarnation
	if err := parent.Mkdir(name, 0700); err != nil {
		parent.Close()
		return nil, err // Existing groups must never be adopted for a new process.
	}
	root, err := parent.OpenRoot(name)
	if err != nil {
		parent.Remove(name)
		parent.Close()
		return nil, err
	}
	dir, err := root.Open(".")
	if err != nil {
		root.Close()
		parent.Remove(name)
		parent.Close()
		return nil, err
	}
	return &ProcessGroup{parent: parent, root: root, directory: dir, name: name}, nil
}

// FD remains owned by ProcessGroup. The caller must serialize process launch
// against Remove; the descriptor must survive until cmd.Start has returned.
func (g *ProcessGroup) FD() int { return int(g.directory.Fd()) }

// SetFrozen waits for cgroup.events, which accounts for the whole subtree.
// Cancellation never thaws the group: an uncertain result must keep the journal
// admission barrier closed and retry the same handoff operation.
func (g *ProcessGroup) SetFrozen(ctx context.Context, frozen bool) error {
	ctx, cancel := context.WithTimeout(ctx, 10*time.Second)
	defer cancel()
	g.mu.Lock()
	defer g.mu.Unlock()
	if err := ctx.Err(); err != nil {
		return err
	}
	if g.root == nil {
		return os.ErrClosed
	}
	file, err := g.root.OpenFile("cgroup.freeze", os.O_WRONLY, 0)
	if err != nil {
		return err
	}
	value := "0"
	if frozen {
		value = "1"
	}
	n, writeErr := file.WriteString(value)
	closeErr := file.Close()
	if writeErr == nil && n != len(value) {
		writeErr = io.ErrShortWrite
	}
	if err := errors.Join(writeErr, closeErr); err != nil {
		return err
	}
	return awaitFrozen(ctx, frozen, func() (string, error) {
		data, err := g.root.ReadFile("cgroup.events")
		return string(data), err
	})
}

// Remove never kills, thaws, migrates or forgets live descendants. Kernel rmdir
// rejects populated groups and groups with child cgroups; retain descriptors on
// that failure so lifecycle cleanup can retry after actual termination.
func (g *ProcessGroup) Remove() error {
	g.mu.Lock()
	defer g.mu.Unlock()
	if g.root == nil {
		return nil
	}
	if err := g.parent.Remove(g.name); err != nil {
		return err
	}
	err := errors.Join(g.directory.Close(), g.root.Close(), g.parent.Close())
	g.root = nil
	return err
}
