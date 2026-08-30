package main

import (
	"errors"
	"io"
	"os"
	"sync"
)

const envdLogSegmentBytes int64 = 512 * 1024

type rotatingLog struct {
	mu      sync.Mutex
	path    string
	file    *os.File
	size    int64
	maxSize int64
}

func openRotatingLog(path string, maxSize int64) (*rotatingLog, error) {
	if maxSize < 1 {
		return nil, errors.New("rotating log maximum size must be positive")
	}
	if info, err := os.Stat(path); err == nil && info.Size() > maxSize {
		if err := os.Remove(path); err != nil {
			return nil, err
		}
	} else if err != nil && !errors.Is(err, os.ErrNotExist) {
		return nil, err
	}
	backup := path + ".1"
	if info, err := os.Stat(backup); err == nil && info.Size() > maxSize {
		if err := os.Remove(backup); err != nil {
			return nil, err
		}
	} else if err != nil && !errors.Is(err, os.ErrNotExist) {
		return nil, err
	}
	file, err := os.OpenFile(path, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o600)
	if err != nil {
		return nil, err
	}
	info, err := file.Stat()
	if err != nil {
		_ = file.Close()
		return nil, err
	}
	return &rotatingLog{path: path, file: file, size: info.Size(), maxSize: maxSize}, nil
}

func (log *rotatingLog) Write(input []byte) (int, error) {
	log.mu.Lock()
	defer log.mu.Unlock()

	originalLength := len(input)
	if int64(len(input)) > log.maxSize {
		input = input[len(input)-int(log.maxSize):]
	}
	if log.size+int64(len(input)) > log.maxSize {
		if err := log.rotate(); err != nil {
			return 0, err
		}
	}
	written, err := log.file.Write(input)
	log.size += int64(written)
	if err != nil {
		return written, err
	}
	if written != len(input) {
		return written, io.ErrShortWrite
	}
	return originalLength, nil
}

func (log *rotatingLog) rotate() error {
	if err := log.file.Close(); err != nil {
		return err
	}
	backup := log.path + ".1"
	if err := os.Remove(backup); err != nil && !errors.Is(err, os.ErrNotExist) {
		return err
	}
	if err := os.Rename(log.path, backup); err != nil && !errors.Is(err, os.ErrNotExist) {
		return err
	}
	file, err := os.OpenFile(log.path, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0o600)
	if err != nil {
		return err
	}
	log.file = file
	log.size = 0
	return nil
}

func (log *rotatingLog) Close() error {
	log.mu.Lock()
	defer log.mu.Unlock()
	return log.file.Close()
}
