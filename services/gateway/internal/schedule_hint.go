package gateway

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"

	schedulerv1 "agentenv/services/api/proto"
)

// buildScheduleHint inspects the incoming request and produces a structured
// scheduling hint. Requests that do not map to a known hint type return a nil
// hint. Advisory cold-start fields are restored unchanged. Hard warm-start
// placement fields are consumed by the gateway before the request is proxied
// to the selected runtime node.
func buildScheduleHint(r *http.Request) (*schedulerv1.ScheduleRequestHint, error) {
	if r.Method != http.MethodPost {
		return nil, nil
	}
	switch strings.TrimRight(r.URL.Path, "/") {
	case "/sandboxes-cold":
		body, _, err := captureRequestBody(r)
		if err != nil {
			return nil, err
		}
		return &schedulerv1.ScheduleRequestHint{
			Kind: &schedulerv1.ScheduleRequestHint_NewColdSandbox{
				NewColdSandbox: parseNewColdSandboxHint(body),
			},
		}, nil
	case "/sandboxes":
		body, inspected, err := captureRequestBodyUpTo(r, maxNewSandboxBodyBytes)
		if err != nil {
			return nil, err
		}
		if !inspected {
			return nil, &requestBodyLimitError{limit: maxNewSandboxBodyBytes}
		}
		newSandbox, err := parseNewSandboxHint(body)
		if err != nil {
			return nil, err
		}
		if newSandbox.GetPlacement() != nil {
			if err := consumeSandboxPlacement(r, body); err != nil {
				return nil, err
			}
		}
		return &schedulerv1.ScheduleRequestHint{
			Kind: &schedulerv1.ScheduleRequestHint_NewSandbox{
				NewSandbox: newSandbox,
			},
		}, nil
	default:
		return nil, nil
	}
}

// maxHintBodyBytes bounds how much of a request body the gateway buffers in
// memory while extracting a scheduling hint. Cold-sandbox creation bodies are
// small, so anything larger is assumed not worth inspecting. Keeping a bound
// here matters because hint extraction runs before upstream authentication;
// without it an unauthenticated client could force the gateway to buffer an
// arbitrarily large body.
const maxHintBodyBytes = 64 * 1024

// maxNewSandboxBodyBytes matches the generated Axum JSON extractor's default
// request limit. Hard placement constraints require complete inspection.
const maxNewSandboxBodyBytes = 2 * 1024 * 1024

type requestBodyLimitError struct {
	limit int64
}

func (e *requestBodyLimitError) Error() string {
	return fmt.Sprintf("sandbox request body exceeds the %d-byte API limit", e.limit)
}

// captureRequestBody buffers up to maxHintBodyBytes of the request body so a
// scheduling hint can be extracted, then restores r.Body so the full body
// remains available for the upstream request. If the body exceeds the budget,
// the buffered prefix is stitched back in front of the unread remainder and
// the caller receives inspected=false.
func captureRequestBody(r *http.Request) ([]byte, bool, error) {
	return captureRequestBodyUpTo(r, maxHintBodyBytes)
}

func captureRequestBodyUpTo(r *http.Request, maxBytes int64) ([]byte, bool, error) {
	if r.Body == nil || r.Body == http.NoBody {
		return nil, true, nil
	}
	orig := r.Body
	buf, err := io.ReadAll(io.LimitReader(orig, maxBytes+1))
	if err != nil {
		return nil, false, err
	}
	if int64(len(buf)) > maxBytes {
		// Too large to inspect: restore the full stream without buffering the
		// remainder and skip hint extraction.
		r.Body = &prefixedBody{Reader: io.MultiReader(bytes.NewReader(buf), orig), closer: orig}
		return nil, false, nil
	}
	_ = orig.Close()
	r.Body = io.NopCloser(bytes.NewReader(buf))
	r.ContentLength = int64(len(buf))
	return buf, true, nil
}

// consumeSandboxPlacement removes scheduler-only constraints before proxying
// the request to a runtime node. A direct runtime request therefore cannot
// silently bypass a hard cluster-level constraint.
func consumeSandboxPlacement(r *http.Request, body []byte) error {
	var request map[string]json.RawMessage
	if err := json.Unmarshal(body, &request); err != nil {
		return fmt.Errorf("invalid sandbox request body: %w", err)
	}
	delete(request, "placement")
	forwardBody, err := json.Marshal(request)
	if err != nil {
		return fmt.Errorf("encode sandbox request for runtime: %w", err)
	}
	r.Body = io.NopCloser(bytes.NewReader(forwardBody))
	r.ContentLength = int64(len(forwardBody))
	return nil
}

// prefixedBody re-presents an already-partially-read body as a single
// ReadCloser: the buffered prefix followed by the unread remainder, while
// closing the underlying body.
type prefixedBody struct {
	io.Reader
	closer io.Closer
}

func (b *prefixedBody) Close() error { return b.closer.Close() }

// newColdSandboxBody mirrors the subset of NewColdSandbox
// (src/api/openapi.yml) that is relevant for scheduling.
type newColdSandboxBody struct {
	Image          string            `json:"image"`
	CPUCount       uint32            `json:"cpuCount"`
	MemoryMB       uint64            `json:"memoryMB"`
	Metadata       map[string]string `json:"metadata"`
	AttachedDrives []struct {
		Source struct {
			Image string `json:"image"`
		} `json:"source"`
	} `json:"attachedDrives"`
}

// parseNewColdSandboxHint extracts the structured cold-sandbox hint from the
// request body. Malformed or partial bodies yield a best-effort hint rather
// than an error, since scheduling hints are advisory.
func parseNewColdSandboxHint(body []byte) *schedulerv1.NewColdSandboxHint {
	hint := &schedulerv1.NewColdSandboxHint{}
	if len(body) == 0 {
		return hint
	}
	var parsed newColdSandboxBody
	if err := json.Unmarshal(body, &parsed); err != nil {
		return hint
	}
	hint.CpuCount = parsed.CPUCount
	hint.MemoryMb = parsed.MemoryMB
	hint.Metadata = parsed.Metadata
	if parsed.Image != "" {
		hint.Images = append(hint.Images, parsed.Image)
	}
	for _, drive := range parsed.AttachedDrives {
		if drive.Source.Image != "" {
			hint.Images = append(hint.Images, drive.Source.Image)
		}
	}
	return hint
}

// newSandboxBody mirrors the subset of NewSandbox (src/api/openapi.yml) that is
// relevant for scheduling.
type newSandboxBody struct {
	Metadata  map[string]string `json:"metadata"`
	Placement json.RawMessage   `json:"placement"`
}

// parseNewSandboxHint extracts the structured sandbox hint from the request
// body. Placement is a hard contract, so malformed JSON or an unknown
// placement constraint fails closed instead of silently changing topology.
func parseNewSandboxHint(body []byte) (*schedulerv1.NewSandboxHint, error) {
	hint := &schedulerv1.NewSandboxHint{}
	if len(body) == 0 {
		return hint, nil
	}
	var parsed newSandboxBody
	if err := json.Unmarshal(body, &parsed); err != nil {
		return nil, fmt.Errorf("invalid sandbox request body: %w", err)
	}
	hint.Metadata = parsed.Metadata
	if len(parsed.Placement) > 0 && !bytes.Equal(bytes.TrimSpace(parsed.Placement), []byte("null")) {
		var placement struct {
			DifferentNodeFrom      []string `json:"differentNodeFrom"`
			SnapshotCompatibleWith []string `json:"snapshotCompatibleWith"`
		}
		decoder := json.NewDecoder(bytes.NewReader(parsed.Placement))
		decoder.DisallowUnknownFields()
		if err := decoder.Decode(&placement); err != nil {
			return nil, fmt.Errorf("invalid sandbox placement: %w", err)
		}
		hint.Placement = &schedulerv1.SandboxPlacement{
			DifferentNodeFrom:      placement.DifferentNodeFrom,
			SnapshotCompatibleWith: placement.SnapshotCompatibleWith,
		}
	}
	return hint, nil
}
