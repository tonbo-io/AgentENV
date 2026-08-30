package main

import (
	"reflect"
	"testing"
)

func TestParseDriveMounts(t *testing.T) {
	want := []driveMount{
		{device: "vdc", mountPath: "/mnt/data"},
		{device: "vdd", mountPath: "/workspace/cache", subPath: "tenant/one"},
	}
	got, err := parseDriveMounts("console=ttyS0 agentenv_drives=vdc:/mnt/data,vdd:/workspace/cache:tenant/one panic=1")
	if err != nil {
		t.Fatalf("parseDriveMounts() error = %v", err)
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("parseDriveMounts() = %#v, want %#v", got, want)
	}
}

func TestParseDriveMountsAbsent(t *testing.T) {
	got, err := parseDriveMounts("console=ttyS0 panic=1")
	if err != nil {
		t.Fatalf("parseDriveMounts() error = %v", err)
	}
	if got != nil {
		t.Fatalf("parseDriveMounts() = %#v, want nil", got)
	}
}

func TestParseDriveMountRejectsInvalidValues(t *testing.T) {
	for _, input := range []string{
		"vdb:/mnt/data",
		"vdc:/",
		"vdc:relative",
		"vdc:/mnt/../data",
		"vdc:/mnt/data:/absolute",
		"vdc:/mnt/data:../escape",
		"vdc:/mnt/data:",
		"vdc:/mnt/data:valid:extra",
	} {
		t.Run(input, func(t *testing.T) {
			if _, err := parseDriveMount(input); err == nil {
				t.Fatalf("parseDriveMount(%q) succeeded, want error", input)
			}
		})
	}
}

func TestParseDriveMountNormalizesEquivalentPaths(t *testing.T) {
	got, err := parseDriveMount("vdc:/mnt//data/:./tenant//one/")
	if err != nil {
		t.Fatalf("parseDriveMount() error = %v", err)
	}
	want := driveMount{device: "vdc", mountPath: "/mnt/data", subPath: "tenant/one"}
	if got != want {
		t.Fatalf("parseDriveMount() = %#v, want %#v", got, want)
	}
}
