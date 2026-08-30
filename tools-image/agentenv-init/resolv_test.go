//go:build linux

package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestWriteResolvConf(t *testing.T) {
	temporary := t.TempDir()
	source := filepath.Join(temporary, "pnp")
	target := filepath.Join(temporary, "resolv.conf")
	if err := os.WriteFile(source, []byte("#PROTO: DHCP\nnameserver 10.0.0.2\nsearch example.internal\ninvalid ignored\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := writeResolvConf(source, target); err != nil {
		t.Fatalf("writeResolvConf() error = %v", err)
	}
	got, err := os.ReadFile(target)
	if err != nil {
		t.Fatal(err)
	}
	want := "nameserver 10.0.0.2\nsearch example.internal\n"
	if string(got) != want {
		t.Fatalf("resolver config = %q, want %q", got, want)
	}
}
