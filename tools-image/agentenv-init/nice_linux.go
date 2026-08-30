//go:build linux

package main

import (
	"fmt"
	"os"

	"golang.org/x/sys/unix"
)

func runNice(arguments []string) error {
	adjustment, command, err := parseNiceArguments(arguments)
	if err != nil {
		return err
	}

	rawPriority, err := unix.Getpriority(unix.PRIO_PROCESS, 0)
	if err != nil {
		return fmt.Errorf("get current priority: %w", err)
	}
	// Linux returns 20 - nice from getpriority(2), while setpriority(2)
	// accepts the actual nice value.
	target := 20 - rawPriority + adjustment
	if target < -20 {
		target = -20
	}
	if target > 19 {
		target = 19
	}
	if err := unix.Setpriority(unix.PRIO_PROCESS, 0, target); err != nil {
		return fmt.Errorf("set priority to %d: %w", target, err)
	}

	executable, err := resolveExecutable(command[0], os.Getenv("PATH"))
	if err != nil {
		return err
	}
	return unix.Exec(executable, command, os.Environ())
}
