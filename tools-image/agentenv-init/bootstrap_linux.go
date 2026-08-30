//go:build linux

package main

import (
	"bufio"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"golang.org/x/sys/unix"
)

const (
	userRoot = "/mnt/user"
	oldRoot  = ".agentenv-old-root"
)

func bootstrap(cmdline string) error {
	if err := mountInitialFilesystems(); err != nil {
		return err
	}
	if err := mountUserRoot(); err != nil {
		return err
	}
	if err := pivotToUserRoot(); err != nil {
		return err
	}
	if err := mountRuntimeFilesystems(cmdline); err != nil {
		return err
	}
	if err := mountExtraDrives(cmdline); err != nil {
		return err
	}
	if err := configureGuestFiles(cmdline); err != nil {
		return err
	}
	if err := bootstrapFailpoint(cmdline, "loopback"); err != nil {
		return err
	}
	if err := bringLoopbackUp(); err != nil {
		return fmt.Errorf("bring loopback up: %w", err)
	}
	return nil
}

func mountInitialFilesystems() error {
	if err := mount("proc", "/proc", "proc", 0, ""); err != nil {
		return fmt.Errorf("mount proc: %w", err)
	}
	if err := mount("sysfs", "/sys", "sysfs", 0, ""); err != nil {
		return fmt.Errorf("mount sysfs: %w", err)
	}
	if err := os.MkdirAll("/sys/fs/cgroup", 0o755); err != nil {
		return fmt.Errorf("create cgroup mount point: %w", err)
	}
	if err := mount("cgroup2", "/sys/fs/cgroup", "cgroup2", 0, ""); err == nil {
		enableCgroupControllers()
	}
	return nil
}

func enableCgroupControllers() {
	for _, controller := range []string{"cpu", "cpuset", "memory", "io", "pids"} {
		_ = os.WriteFile("/sys/fs/cgroup/cgroup.subtree_control", []byte("+"+controller), 0o644)
	}
}

func mountUserRoot() error {
	if err := os.MkdirAll(userRoot, 0o755); err != nil {
		return fmt.Errorf("create user root mount point: %w", err)
	}
	if err := mountExt4("/dev/vdb", userRoot); err != nil {
		return fmt.Errorf("mount user root /dev/vdb: %w", err)
	}

	for _, directory := range []string{"proc", "dev", "sys", "run", "tmp", "agentenv"} {
		if err := ensureDirectDirectory(filepath.Join(userRoot, directory), 0o755); err != nil {
			return err
		}
	}
	if err := os.Chmod(filepath.Join(userRoot, "tmp"), 0o1777); err != nil {
		return fmt.Errorf("set user tmp permissions: %w", err)
	}
	if err := mount("/agentenv", filepath.Join(userRoot, "agentenv"), "", unix.MS_BIND|unix.MS_REC, ""); err != nil {
		return fmt.Errorf("bind platform tools: %w", err)
	}
	return nil
}

func ensureDirectDirectory(target string, mode os.FileMode) error {
	info, err := os.Lstat(target)
	switch {
	case errors.Is(err, os.ErrNotExist):
		if err := os.Mkdir(target, mode); err != nil {
			return fmt.Errorf("create directory %s: %w", target, err)
		}
		return nil
	case err != nil:
		return fmt.Errorf("inspect directory %s: %w", target, err)
	case info.IsDir():
		return nil
	default:
		return fmt.Errorf("reserved mount point %s must be a directory", target)
	}
}

func pivotToUserRoot() error {
	if err := os.Mkdir(filepath.Join(userRoot, oldRoot), 0o700); err != nil && !errors.Is(err, os.ErrExist) {
		return fmt.Errorf("create old-root directory: %w", err)
	}
	if err := os.Chdir(userRoot); err != nil {
		return fmt.Errorf("change directory to user root: %w", err)
	}
	if err := unix.PivotRoot(".", oldRoot); err != nil {
		return fmt.Errorf("pivot to user root: %w", err)
	}
	if err := os.Chdir("/"); err != nil {
		return fmt.Errorf("change directory after pivot: %w", err)
	}

	for _, directory := range []string{"proc", "dev", "sys"} {
		if err := unix.Mount(filepath.Join("/", oldRoot, directory), filepath.Join("/", directory), "", unix.MS_MOVE, ""); err != nil {
			return fmt.Errorf("move %s mount: %w", directory, err)
		}
	}
	oldRootPath := filepath.Join("/", oldRoot)
	if err := unix.Unmount(oldRootPath, unix.MNT_DETACH); err != nil {
		return fmt.Errorf("detach old root: %w", err)
	}
	if err := os.Remove(oldRootPath); err != nil {
		return fmt.Errorf("remove old-root directory: %w", err)
	}
	return nil
}

func mountRuntimeFilesystems(cmdline string) error {
	if err := mount("tmpfs", "/run", "tmpfs", unix.MS_NOSUID|unix.MS_NODEV, "mode=0755"); err != nil {
		return fmt.Errorf("mount /run: %w", err)
	}
	for path, mode := range map[string]os.FileMode{
		"/run/lock":      0o755,
		"/run/agentenv":  0o700,
		"/dev/pts":       0o755,
		"/dev/shm":       0o1777,
		"/usr/local/bin": 0o755,
		"/usr/bin":       0o755,
		"/etc":           0o755,
	} {
		if err := os.MkdirAll(path, mode); err != nil {
			return fmt.Errorf("create %s: %w", path, err)
		}
		if err := os.Chmod(path, mode); err != nil {
			return fmt.Errorf("set %s permissions: %w", path, err)
		}
	}
	if err := bootstrapFailpoint(cmdline, "devpts"); err != nil {
		return err
	}
	if err := mount("devpts", "/dev/pts", "devpts", unix.MS_NOSUID|unix.MS_NOEXEC, "gid=5,mode=620,ptmxmode=0666"); err != nil {
		return fmt.Errorf("mount devpts: %w", err)
	}
	if err := bootstrapFailpoint(cmdline, "shared-memory"); err != nil {
		return err
	}
	if err := mount("tmpfs", "/dev/shm", "tmpfs", unix.MS_NOSUID|unix.MS_NODEV, "mode=1777"); err != nil {
		return fmt.Errorf("mount shared memory: %w", err)
	}
	return nil
}

func mount(source, target, filesystem string, flags uintptr, data string) error {
	err := unix.Mount(source, target, filesystem, flags, data)
	if errors.Is(err, unix.EBUSY) {
		return nil
	}
	return err
}

func mountExt4(source, target string) error {
	err := mount(source, target, "ext4", 0, "")
	if errors.Is(err, unix.EROFS) || errors.Is(err, unix.EACCES) {
		return mount(source, target, "ext4", unix.MS_RDONLY, "")
	}
	return err
}

func configureGuestFiles(cmdline string) error {
	for link, target := range map[string]string{
		"/dev/fd":     "/proc/self/fd",
		"/dev/stdin":  "/proc/self/fd/0",
		"/dev/stdout": "/proc/self/fd/1",
		"/dev/stderr": "/proc/self/fd/2",
		"/dev/ptmx":   "pts/ptmx",
	} {
		if err := ensureSymlink(target, link, false); err != nil {
			return err
		}
	}
	if _, err := os.Stat("/etc/hosts"); errors.Is(err, os.ErrNotExist) {
		if err := os.WriteFile("/etc/hosts", []byte("127.0.0.1 localhost\n::1 localhost\n"), 0o644); err != nil {
			return fmt.Errorf("write /etc/hosts: %w", err)
		}
	} else if err != nil {
		return fmt.Errorf("inspect /etc/hosts: %w", err)
	}
	if err := ensureSymlink(envdPath, "/usr/local/bin/envd", true); err != nil {
		return err
	}
	if !executableFile("/usr/bin/nice") {
		if err := ensureSymlink("/agentenv/agentenv-init", "/usr/bin/nice", true); err != nil {
			return err
		}
	}
	if err := bootstrapFailpoint(cmdline, "dns"); err != nil {
		return err
	}
	if err := writeResolvConf("/proc/net/pnp", "/etc/resolv.conf"); err != nil {
		return fmt.Errorf("configure DNS: %w", err)
	}
	return nil
}

func ensureSymlink(target, link string, replace bool) error {
	if _, err := os.Lstat(link); err == nil {
		if !replace {
			return nil
		}
		if err := os.Remove(link); err != nil {
			return fmt.Errorf("replace symlink %s: %w", link, err)
		}
	} else if !errors.Is(err, os.ErrNotExist) {
		return fmt.Errorf("inspect symlink %s: %w", link, err)
	}
	if err := os.Symlink(target, link); err != nil {
		return fmt.Errorf("create symlink %s: %w", link, err)
	}
	return nil
}

func writeResolvConf(source, target string) error {
	input, err := os.Open(source)
	if err != nil {
		return fmt.Errorf("open kernel DNS state: %w", err)
	}
	defer input.Close()

	var lines []string
	scanner := bufio.NewScanner(input)
	for scanner.Scan() {
		fields := strings.Fields(scanner.Text())
		if len(fields) >= 2 && (fields[0] == "nameserver" || fields[0] == "domain" || fields[0] == "search") {
			lines = append(lines, strings.Join(fields, " "))
		}
	}
	if err := scanner.Err(); err != nil {
		return fmt.Errorf("read kernel DNS state: %w", err)
	}
	if len(lines) == 0 {
		return errors.New("kernel DNS state contains no resolver entries")
	}

	temporaryFile, err := os.CreateTemp(filepath.Dir(target), ".agentenv-resolv-*")
	if err != nil {
		return fmt.Errorf("create temporary resolver config: %w", err)
	}
	temporary := temporaryFile.Name()
	defer os.Remove(temporary)
	if err := temporaryFile.Chmod(0o644); err != nil {
		temporaryFile.Close()
		return fmt.Errorf("set temporary resolver config permissions: %w", err)
	}
	if _, err := temporaryFile.WriteString(strings.Join(lines, "\n") + "\n"); err != nil {
		temporaryFile.Close()
		return fmt.Errorf("write temporary resolver config: %w", err)
	}
	if err := temporaryFile.Close(); err != nil {
		return fmt.Errorf("close temporary resolver config: %w", err)
	}
	if err := os.Remove(target); err != nil && !errors.Is(err, os.ErrNotExist) {
		return fmt.Errorf("replace resolver config: %w", err)
	}
	if err := os.Rename(temporary, target); err != nil {
		return fmt.Errorf("install resolver config: %w", err)
	}
	return nil
}

func mountExtraDrives(cmdline string) error {
	mounts, err := parseDriveMounts(cmdline)
	if err != nil {
		return err
	}
	for _, drive := range mounts {
		if err := mountExtraDrive(drive); err != nil {
			return fmt.Errorf("mount /dev/%s at %s: %w", drive.device, drive.mountPath, err)
		}
	}
	return nil
}

func mountExtraDrive(drive driveMount) error {
	if err := os.MkdirAll(drive.mountPath, 0o755); err != nil {
		return err
	}
	device := filepath.Join("/dev", drive.device)
	if drive.subPath == "" {
		return mountExt4(device, drive.mountPath)
	}

	stage := filepath.Join("/run/agentenv", drive.device)
	if err := os.Mkdir(stage, 0o700); err != nil && !errors.Is(err, os.ErrExist) {
		return err
	}
	if err := mountExt4(device, stage); err != nil {
		return err
	}
	defer func() {
		_ = unix.Unmount(stage, unix.MNT_DETACH)
		_ = os.Remove(stage)
	}()

	source := filepath.Join(stage, filepath.FromSlash(drive.subPath))
	info, err := os.Stat(source)
	if err != nil {
		return err
	}
	if !info.IsDir() {
		return fmt.Errorf("sub-path %s is not a directory", drive.subPath)
	}
	return mount(source, drive.mountPath, "", unix.MS_BIND|unix.MS_REC, "")
}

func bringLoopbackUp() error {
	fd, err := unix.Socket(unix.AF_INET, unix.SOCK_DGRAM|unix.SOCK_CLOEXEC, 0)
	if err != nil {
		return err
	}
	defer unix.Close(fd)

	request, err := unix.NewIfreq("lo")
	if err != nil {
		return err
	}
	if err := unix.IoctlIfreq(fd, unix.SIOCGIFFLAGS, request); err != nil {
		return err
	}
	request.SetUint16(request.Uint16() | unix.IFF_UP)
	return unix.IoctlIfreq(fd, unix.SIOCSIFFLAGS, request)
}
