package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

func resolveExecutable(name, pathValue string) (string, error) {
	if strings.ContainsRune(name, '/') {
		if executableFile(name) {
			return name, nil
		}
		return "", fmt.Errorf("command %q is not executable", name)
	}

	for _, directory := range filepath.SplitList(pathValue) {
		candidate := filepath.Join(directory, name)
		if executableFile(candidate) {
			return candidate, nil
		}
	}
	return "", fmt.Errorf("command %q not found in PATH", name)
}

func executableFile(path string) bool {
	info, err := os.Stat(path)
	return err == nil && info.Mode().IsRegular() && info.Mode().Perm()&0o111 != 0
}
