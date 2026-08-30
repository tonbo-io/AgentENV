package main

import (
	"reflect"
	"testing"
)

func TestParseNiceArguments(t *testing.T) {
	for _, test := range []struct {
		name       string
		arguments  []string
		adjustment int
		command    []string
	}{
		{name: "envd form", arguments: []string{"-n", "-15", "/bin/echo", "hello"}, adjustment: -15, command: []string{"/bin/echo", "hello"}},
		{name: "default", arguments: []string{"/bin/true"}, adjustment: 10, command: []string{"/bin/true"}},
		{name: "long", arguments: []string{"--adjustment=3", "/bin/true"}, adjustment: 3, command: []string{"/bin/true"}},
		{name: "legacy", arguments: []string{"-5", "/bin/true"}, adjustment: 5, command: []string{"/bin/true"}},
		{name: "separator", arguments: []string{"--", "-command"}, adjustment: 10, command: []string{"-command"}},
	} {
		t.Run(test.name, func(t *testing.T) {
			adjustment, command, err := parseNiceArguments(test.arguments)
			if err != nil {
				t.Fatalf("parseNiceArguments() error = %v", err)
			}
			if adjustment != test.adjustment || !reflect.DeepEqual(command, test.command) {
				t.Fatalf("parseNiceArguments() = (%d, %#v), want (%d, %#v)", adjustment, command, test.adjustment, test.command)
			}
		})
	}
}

func TestParseNiceArgumentsRejectsMissingValues(t *testing.T) {
	for _, arguments := range [][]string{{}, {"-n"}, {"--adjustment=bad", "/bin/true"}} {
		if _, _, err := parseNiceArguments(arguments); err == nil {
			t.Fatalf("parseNiceArguments(%#v) succeeded, want error", arguments)
		}
	}
}
