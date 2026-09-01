package scheduler

import (
	"encoding/json"
	"fmt"
	"sort"
	"strconv"
	"strings"
)

// x86CPUConfig mirrors the x86_64 Firecracker cpu_config_json structure.
// ARM configs are deliberately handled separately: serializing this type on
// aarch64 introduces x86-only fields that Firecracker rejects.
type x86CPUConfig struct {
	KvmCapabilities []string        `json:"kvm_capabilities"`
	CpuidModifiers  []cpuidModifier `json:"cpuid_modifiers"`
	MsrModifiers    []msrModifier   `json:"msr_modifiers"`
}

// armCPUConfig contains only fields accepted by the aarch64 Firecracker API.
// The Scheduler currently emits an empty native template after proving every
// node has the same fingerprint; a heterogeneous ARM template must be built
// and verified explicitly rather than replaying a fingerprint dump.
type armCPUConfig struct {
	KvmCapabilities []string          `json:"kvm_capabilities"`
	RegModifiers    []json.RawMessage `json:"reg_modifiers"`
	VcpuFeatures    []json.RawMessage `json:"vcpu_features"`
}

type cpuConfigArchitecture int

const (
	cpuConfigArchitectureNeutral cpuConfigArchitecture = iota
	cpuConfigArchitectureX86
	cpuConfigArchitectureARM
)

type cpuidModifier struct {
	Leaf      string        `json:"leaf"`
	Subleaf   string        `json:"subleaf"`
	Flags     int           `json:"flags"`
	Modifiers []registerMod `json:"modifiers"`
}

type registerMod struct {
	Register string `json:"register"`
	Bitmap   string `json:"bitmap"`
}

type msrModifier struct {
	Addr   string `json:"addr"`
	Bitmap string `json:"bitmap"`
}

// cpuidLeafKey is the composite key identifying a CPUID entry.
type cpuidLeafKey struct {
	leaf, subleaf uint32
	flags         int
}

// IntersectCpuConfigs computes a conservative common Firecracker CPU config.
// x86_64 entries present in every input are retained and their shared bitmap
// fields are ANDed. Identical aarch64 fingerprints select an empty native ARM
// template; synthesizing a heterogeneous ARM template requires explicit
// Firecracker verification.
// Returns an empty string when jsons is empty.
func IntersectCpuConfigs(jsons []string) (string, error) {
	if len(jsons) == 0 {
		return "", nil
	}

	canonical := make([]string, len(jsons))
	architecture := cpuConfigArchitectureNeutral
	for i, j := range jsons {
		config, configArchitecture, err := canonicalCPUConfig(j)
		if err != nil {
			return "", fmt.Errorf("parse config %d: %w", i, err)
		}
		canonical[i] = config
		if configArchitecture == cpuConfigArchitectureNeutral {
			continue
		}
		if architecture != cpuConfigArchitectureNeutral && architecture != configArchitecture {
			return "", fmt.Errorf("configs contain mixed x86_64 and aarch64 fields")
		}
		architecture = configArchitecture
	}

	if architecture == cpuConfigArchitectureARM {
		for _, config := range canonical[1:] {
			if config != canonical[0] {
				return "", fmt.Errorf("aarch64 configs differ; a verified common CPU template is required")
			}
		}
		result := armCPUConfig{
			KvmCapabilities: []string{},
			RegModifiers:    []json.RawMessage{},
			VcpuFeatures:    []json.RawMessage{},
		}
		out, err := json.Marshal(result)
		if err != nil {
			return "", fmt.Errorf("marshal native aarch64 template: %w", err)
		}
		return string(out), nil
	}

	if architecture == cpuConfigArchitectureNeutral {
		allEqual := true
		for _, config := range canonical[1:] {
			allEqual = allEqual && config == canonical[0]
		}
		if allEqual {
			return canonical[0], nil
		}
	}

	configs := make([]x86CPUConfig, len(canonical))
	for i, config := range canonical {
		if err := json.Unmarshal([]byte(config), &configs[i]); err != nil {
			return "", fmt.Errorf("parse canonical config %d: %w", i, err)
		}
	}

	cpuidMods, err := intersectCpuidModifiers(configs)
	if err != nil {
		return "", err
	}
	msrMods, err := intersectMsrModifiers(configs)
	if err != nil {
		return "", err
	}

	result := x86CPUConfig{
		KvmCapabilities: intersectKvmCapabilities(configs),
		CpuidModifiers:  cpuidMods,
		MsrModifiers:    msrMods,
	}

	out, err := json.Marshal(result)
	if err != nil {
		return "", fmt.Errorf("marshal result: %w", err)
	}
	return string(out), nil
}

func canonicalCPUConfig(input string) (string, cpuConfigArchitecture, error) {
	var fields map[string]any
	if err := json.Unmarshal([]byte(input), &fields); err != nil {
		return "", cpuConfigArchitectureNeutral, err
	}
	if fields == nil {
		return "", cpuConfigArchitectureNeutral, fmt.Errorf("CPU config must be a JSON object")
	}

	hasX86Fields := false
	hasARMFields := false
	for field := range fields {
		switch field {
		case "kvm_capabilities":
		case "cpuid_modifiers", "msr_modifiers":
			hasX86Fields = true
		case "reg_modifiers", "vcpu_features":
			hasARMFields = true
		default:
			return "", cpuConfigArchitectureNeutral, fmt.Errorf("unknown Firecracker CPU config field %q", field)
		}
	}
	if hasX86Fields && hasARMFields {
		return "", cpuConfigArchitectureNeutral, fmt.Errorf("config contains both x86_64 and aarch64 fields")
	}

	architecture := cpuConfigArchitectureNeutral
	if hasX86Fields {
		architecture = cpuConfigArchitectureX86
	} else if hasARMFields {
		architecture = cpuConfigArchitectureARM
	}
	canonical, err := json.Marshal(fields)
	if err != nil {
		return "", cpuConfigArchitectureNeutral, fmt.Errorf("marshal canonical config: %w", err)
	}
	return string(canonical), architecture, nil
}

var kvmReadOnlyLeafIDs = map[uint32]bool{
	0xb:  true,
	0x1f: true,
}

func intersectCpuidModifiers(configs []x86CPUConfig) ([]cpuidModifier, error) {
	n := len(configs)

	// Build per-config maps: leafKey -> register -> u32 bitmap value.
	perConfig := make([]map[cpuidLeafKey]map[string]uint32, n)
	for i, cfg := range configs {
		perConfig[i] = make(map[cpuidLeafKey]map[string]uint32)
		for _, mod := range cfg.CpuidModifiers {
			lk, err := makeCpuidLeafKey(mod)
			if err != nil {
				return nil, fmt.Errorf("config %d cpuid %s/%s: %w", i, mod.Leaf, mod.Subleaf, err)
			}
			if perConfig[i][lk] == nil {
				perConfig[i][lk] = make(map[string]uint32)
			}
			for _, rm := range mod.Modifiers {
				v, err := parseBitmapU32(rm.Bitmap)
				if err != nil {
					return nil, fmt.Errorf("config %d cpuid %s %s bitmap: %w", i, mod.Leaf, rm.Register, err)
				}
				perConfig[i][lk][rm.Register] = v
			}
		}
	}

	// Count how many configs contain each leaf key.
	leafCount := make(map[cpuidLeafKey]int)
	for _, m := range perConfig {
		for lk := range m {
			leafCount[lk]++
		}
	}

	// Iterate in the order of configs[0] so output is deterministic.
	var result []cpuidModifier
	seenLeaf := make(map[cpuidLeafKey]bool)
	for _, mod := range configs[0].CpuidModifiers {
		lk, _ := makeCpuidLeafKey(mod) // already validated above
		if seenLeaf[lk] {
			continue
		}
		seenLeaf[lk] = true
		if leafCount[lk] < n {
			continue // not present in every config
		}
		if kvmReadOnlyLeafIDs[lk.leaf] {
			continue // KVM does not allow overriding this leaf
		}

		// Count how many configs have each register for this leaf.
		regCount := make(map[string]int)
		for _, m := range perConfig {
			for reg := range m[lk] {
				regCount[reg]++
			}
		}

		// Note: the register candidate set is seeded from configs[0]. Registers
		// present in configs[1..n] but absent in configs[0] are not visited here,
		// even if they appear in every config. The result is therefore conservative
		// (safe for migration) but may be narrower than the true intersection.
		// regCount already tracks full cross-config presence, so the per-register
		// AND below is still correct for the registers that are visited.
		var regMods []registerMod
		seenReg := make(map[string]bool)
		for _, rm := range mod.Modifiers {
			if seenReg[rm.Register] {
				continue
			}
			seenReg[rm.Register] = true
			if regCount[rm.Register] < n {
				continue // not present in every config
			}
			// AND across all configs. Starting value is all-ones (AND identity).
			var andVal uint32 = ^uint32(0)
			for _, m := range perConfig {
				andVal &= m[lk][rm.Register]
			}
			regMods = append(regMods, registerMod{
				Register: rm.Register,
				Bitmap:   formatBitmap(uint64(andVal), 32),
			})
		}

		if len(regMods) > 0 {
			result = append(result, cpuidModifier{
				Leaf:      mod.Leaf,
				Subleaf:   mod.Subleaf,
				Flags:     mod.Flags,
				Modifiers: regMods,
			})
		}
	}

	if result == nil {
		result = []cpuidModifier{}
	}
	return result, nil
}

func intersectMsrModifiers(configs []x86CPUConfig) ([]msrModifier, error) {
	n := len(configs)

	perConfig := make([]map[uint32]uint64, n)
	for i, cfg := range configs {
		perConfig[i] = make(map[uint32]uint64)
		for _, mod := range cfg.MsrModifiers {
			addr, err := parseHexUint32(mod.Addr)
			if err != nil {
				return nil, fmt.Errorf("config %d msr addr %q: %w", i, mod.Addr, err)
			}
			v, err := parseBitmapU64(mod.Bitmap)
			if err != nil {
				return nil, fmt.Errorf("config %d msr %s bitmap: %w", i, mod.Addr, err)
			}
			perConfig[i][addr] = v
		}
	}

	addrCount := make(map[uint32]int)
	for _, m := range perConfig {
		for addr := range m {
			addrCount[addr]++
		}
	}

	var result []msrModifier
	seenAddr := make(map[uint32]bool)
	for _, mod := range configs[0].MsrModifiers {
		addr, _ := parseHexUint32(mod.Addr) // already validated above
		if seenAddr[addr] {
			continue
		}
		seenAddr[addr] = true
		if addrCount[addr] < n {
			continue
		}
		var andVal uint64 = ^uint64(0)
		for _, m := range perConfig {
			andVal &= m[addr]
		}
		result = append(result, msrModifier{
			Addr:   mod.Addr,
			Bitmap: formatBitmap(andVal, 64),
		})
	}

	if result == nil {
		result = []msrModifier{}
	}
	return result, nil
}

func intersectKvmCapabilities(configs []x86CPUConfig) []string {
	counts := make(map[string]int)
	for _, cfg := range configs {
		seen := make(map[string]bool)
		for _, cap := range cfg.KvmCapabilities {
			if !seen[cap] {
				counts[cap]++
				seen[cap] = true
			}
		}
	}
	n := len(configs)
	var result []string
	for cap, count := range counts {
		if count == n {
			result = append(result, cap)
		}
	}
	sort.Strings(result)
	if result == nil {
		result = []string{}
	}
	return result
}

func makeCpuidLeafKey(mod cpuidModifier) (cpuidLeafKey, error) {
	leaf, err := parseHexUint32(mod.Leaf)
	if err != nil {
		return cpuidLeafKey{}, fmt.Errorf("parse leaf: %w", err)
	}
	subleaf, err := parseHexUint32(mod.Subleaf)
	if err != nil {
		return cpuidLeafKey{}, fmt.Errorf("parse subleaf: %w", err)
	}
	return cpuidLeafKey{leaf, subleaf, mod.Flags}, nil
}

func parseHexUint32(s string) (uint32, error) {
	s = strings.TrimPrefix(strings.ToLower(s), "0x")
	v, err := strconv.ParseUint(s, 16, 32)
	if err != nil {
		return 0, err
	}
	return uint32(v), nil
}

func parseBitmapU32(s string) (uint32, error) {
	s = strings.TrimPrefix(s, "0b")
	v, err := strconv.ParseUint(s, 2, 32)
	if err != nil {
		return 0, err
	}
	return uint32(v), nil
}

func parseBitmapU64(s string) (uint64, error) {
	s = strings.TrimPrefix(s, "0b")
	return strconv.ParseUint(s, 2, 64)
}

// formatBitmap serializes v as a "0b" prefixed binary string padded to bits
// digits (32 for CPUID registers, 64 for MSRs).
func formatBitmap(v uint64, bits int) string {
	return fmt.Sprintf("0b%0*b", bits, v)
}
