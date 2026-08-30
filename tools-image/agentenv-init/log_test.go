package main

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"
)

func TestRotatingLogBoundsCurrentAndPreviousSegments(t *testing.T) {
	path := filepath.Join(t.TempDir(), "envd.log")
	log, err := openRotatingLog(path, 16)
	if err != nil {
		t.Fatal(err)
	}
	for _, input := range [][]byte{[]byte("first-1234"), []byte("second-5678"), []byte("latest")} {
		if _, err := log.Write(input); err != nil {
			t.Fatal(err)
		}
	}
	if err := log.Close(); err != nil {
		t.Fatal(err)
	}
	for _, segment := range []string{path, path + ".1"} {
		info, err := os.Stat(segment)
		if err != nil {
			t.Fatal(err)
		}
		if info.Size() > 16 {
			t.Fatalf("%s grew to %d bytes", segment, info.Size())
		}
	}
	current, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Contains(current, []byte("latest")) {
		t.Fatalf("current log omitted latest bytes: %q", current)
	}
}

func TestRotatingLogRetainsTailOfOversizedWrite(t *testing.T) {
	path := filepath.Join(t.TempDir(), "envd.log")
	log, err := openRotatingLog(path, 8)
	if err != nil {
		t.Fatal(err)
	}
	input := []byte("0123456789abcdef")
	written, err := log.Write(input)
	if err != nil {
		t.Fatal(err)
	}
	if written != len(input) {
		t.Fatalf("Write() = %d, want %d", written, len(input))
	}
	if err := log.Close(); err != nil {
		t.Fatal(err)
	}
	got, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	if string(got) != "89abcdef" {
		t.Fatalf("log tail = %q, want %q", got, "89abcdef")
	}
}
