//go:build linux

package main

import (
	"errors"
	"fmt"
	"log"
	"os"
	"os/exec"
	"os/signal"
	"syscall"

	"golang.org/x/sys/unix"
)

const envdPath = "/agentenv/envd"

func main() {
	if isNiceInvocation(os.Args[0]) {
		if err := runNice(os.Args[1:]); err != nil {
			fmt.Fprintf(os.Stderr, "nice: %v\n", err)
			os.Exit(125)
		}
		return
	}

	logger := log.New(os.Stderr, "agentenv-init: ", log.LstdFlags|log.Lmicroseconds)
	if os.Getpid() != 1 {
		logger.Fatalf("init mode must run as PID 1, got PID %d", os.Getpid())
	}
	if err := bootstrap(logger); err != nil {
		logger.Printf("bootstrap failed: %v", err)
		powerOff(logger)
	}

	signals := make(chan os.Signal, 8)
	signal.Notify(signals, syscall.SIGCHLD, syscall.SIGTERM, syscall.SIGINT, syscall.SIGQUIT)
	defer signal.Stop(signals)

	if err := os.Setenv("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"); err != nil {
		logger.Printf("set PATH: %v", err)
		powerOff(logger)
	}
	if err := os.Setenv("HOME", "/root"); err != nil {
		logger.Printf("set HOME: %v", err)
		powerOff(logger)
	}

	cmd := exec.Command(envdPath)
	cmd.Env = os.Environ()
	cmd.Stdin = os.Stdin
	envdLog, err := os.OpenFile("/run/agentenv/envd.log", os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o600)
	if err != nil {
		logger.Printf("open envd log: %v", err)
		powerOff(logger)
	}
	cmd.Stdout = envdLog
	cmd.Stderr = envdLog
	if err := cmd.Start(); err != nil {
		_ = envdLog.Close()
		logger.Printf("start envd: %v", err)
		powerOff(logger)
	}
	if err := envdLog.Close(); err != nil {
		logger.Printf("close parent envd log descriptor: %v", err)
	}
	logger.Printf("started envd pid=%d", cmd.Process.Pid)

	if err := reapUntilEnvdExits(cmd.Process.Pid, signals); err != nil {
		logger.Printf("envd exited: %v", err)
	} else {
		logger.Printf("envd exited")
	}
	powerOff(logger)
}

func reapUntilEnvdExits(envdPID int, signals <-chan os.Signal) error {
	for sig := range signals {
		switch sig {
		case syscall.SIGTERM, syscall.SIGINT, syscall.SIGQUIT:
			return fmt.Errorf("received %s", sig)
		case syscall.SIGCHLD:
			for {
				var status unix.WaitStatus
				pid, err := unix.Wait4(-1, &status, unix.WNOHANG, nil)
				switch {
				case errors.Is(err, unix.ECHILD), pid == 0:
					break
				case err != nil:
					return fmt.Errorf("wait for child: %w", err)
				case pid == envdPID:
					return fmt.Errorf("pid=%d status=%s", pid, formatWaitStatus(status))
				default:
					continue
				}
				break
			}
		}
	}
	return errors.New("signal channel closed")
}

func formatWaitStatus(status unix.WaitStatus) string {
	switch {
	case status.Exited():
		return fmt.Sprintf("exit-code=%d", status.ExitStatus())
	case status.Signaled():
		return fmt.Sprintf("signal=%s", status.Signal())
	default:
		return fmt.Sprintf("raw=%d", status)
	}
}

func powerOff(logger *log.Logger) {
	logger.Printf("powering off guest")
	unix.Sync()
	if err := unix.Reboot(unix.LINUX_REBOOT_CMD_POWER_OFF); err != nil {
		logger.Printf("power off failed: %v; guest remains unavailable", err)
	}
	for {
		_ = unix.Pause()
	}
}
