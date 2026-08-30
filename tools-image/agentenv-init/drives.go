package main

import (
	"fmt"
	"path"
	"strings"
)

const drivesArgumentPrefix = "agentenv_drives="

type driveMount struct {
	device    string
	mountPath string
	subPath   string
}

func parseDriveMounts(cmdline string) ([]driveMount, error) {
	for _, arg := range strings.Fields(cmdline) {
		if !strings.HasPrefix(arg, drivesArgumentPrefix) {
			continue
		}

		value := strings.TrimPrefix(arg, drivesArgumentPrefix)
		if value == "" {
			return nil, fmt.Errorf("%s is empty", drivesArgumentPrefix)
		}

		entries := strings.Split(value, ",")
		mounts := make([]driveMount, 0, len(entries))
		for _, entry := range entries {
			mount, err := parseDriveMount(entry)
			if err != nil {
				return nil, err
			}
			mounts = append(mounts, mount)
		}
		return mounts, nil
	}
	return nil, nil
}

func parseDriveMount(entry string) (driveMount, error) {
	parts := strings.Split(entry, ":")
	if len(parts) < 2 || len(parts) > 3 {
		return driveMount{}, fmt.Errorf("invalid drive mount %q", entry)
	}
	if !validVirtioBlockDevice(parts[0]) {
		return driveMount{}, fmt.Errorf("invalid drive device %q", parts[0])
	}
	mountPath, ok := normalizeAbsoluteGuestPath(parts[1])
	if !ok {
		return driveMount{}, fmt.Errorf("invalid drive mount path %q", parts[1])
	}
	var subPath string
	if len(parts) == 3 {
		var valid bool
		subPath, valid = normalizeRelativeGuestPath(parts[2])
		if !valid {
			return driveMount{}, fmt.Errorf("invalid drive sub-path %q", parts[2])
		}
	}
	return driveMount{device: parts[0], mountPath: mountPath, subPath: subPath}, nil
}

func validVirtioBlockDevice(device string) bool {
	if len(device) != 3 || !strings.HasPrefix(device, "vd") {
		return false
	}
	return device[2] >= 'c' && device[2] <= 'z'
}

func normalizeAbsoluteGuestPath(value string) (string, bool) {
	cleaned := path.Clean(value)
	return cleaned, strings.HasPrefix(value, "/") && cleaned != "/" && !containsParentComponent(value)
}

func normalizeRelativeGuestPath(value string) (string, bool) {
	cleaned := path.Clean(value)
	return cleaned, value != "" && !strings.HasPrefix(value, "/") && !containsParentComponent(value)
}

func containsParentComponent(value string) bool {
	for _, component := range strings.Split(value, "/") {
		if component == ".." {
			return true
		}
	}
	return false
}
