package main

import "testing"

func TestBootstrapFailpoint(t *testing.T) {
	if err := bootstrapFailpoint("console=ttyS0 agentenv_bootstrap_failpoint=dns panic=1", "dns"); err == nil {
		t.Fatal("bootstrapFailpoint() succeeded, want injected failure")
	}
	if err := bootstrapFailpoint("console=ttyS0 panic=1", "dns"); err != nil {
		t.Fatalf("bootstrapFailpoint() error = %v, want nil", err)
	}
}
