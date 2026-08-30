package main

import (
	"fmt"
	"path/filepath"
	"strconv"
	"strings"
)

func isNiceInvocation(executable string) bool {
	return filepath.Base(executable) == "nice"
}

func parseNiceArguments(arguments []string) (int, []string, error) {
	adjustment := 10
	for len(arguments) > 0 {
		argument := arguments[0]
		switch {
		case argument == "--":
			arguments = arguments[1:]
			goto parsed
		case argument == "-n" || argument == "--adjustment":
			if len(arguments) < 2 {
				return 0, nil, fmt.Errorf("%s requires a value", argument)
			}
			value, err := strconv.Atoi(arguments[1])
			if err != nil {
				return 0, nil, fmt.Errorf("invalid adjustment %q", arguments[1])
			}
			adjustment = value
			arguments = arguments[2:]
		case strings.HasPrefix(argument, "--adjustment="):
			value, err := strconv.Atoi(strings.TrimPrefix(argument, "--adjustment="))
			if err != nil {
				return 0, nil, fmt.Errorf("invalid adjustment %q", argument)
			}
			adjustment = value
			arguments = arguments[1:]
		case len(argument) > 1 && argument[0] == '-' && isDecimal(argument[1:]):
			value, err := strconv.Atoi(argument[1:])
			if err != nil {
				return 0, nil, fmt.Errorf("invalid adjustment %q", argument)
			}
			adjustment = value
			arguments = arguments[1:]
		default:
			goto parsed
		}
	}

parsed:
	if len(arguments) == 0 {
		return 0, nil, fmt.Errorf("missing command")
	}
	return adjustment, arguments, nil
}

func isDecimal(value string) bool {
	if value == "" {
		return false
	}
	if value[0] == '+' || value[0] == '-' {
		value = value[1:]
	}
	if value == "" {
		return false
	}
	for _, char := range value {
		if char < '0' || char > '9' {
			return false
		}
	}
	return true
}
