package main

import (
	"fmt"
	"strings"
)

const bootstrapFailpointArgumentPrefix = "agentenv_bootstrap_failpoint="

func bootstrapFailpoint(cmdline, step string) error {
	for _, argument := range strings.Fields(cmdline) {
		if strings.HasPrefix(argument, bootstrapFailpointArgumentPrefix) && strings.TrimPrefix(argument, bootstrapFailpointArgumentPrefix) == step {
			return fmt.Errorf("injected %s failure", step)
		}
	}
	return nil
}
