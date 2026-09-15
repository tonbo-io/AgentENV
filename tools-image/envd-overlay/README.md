# Recoverable process I/O

This overlay extends the existing envd process service at the exact upstream commit in `../envd-source.env`. `../apply-envd-overlay.sh` verifies that commit, checks/applies `upstream.patch`, copies the new implementation and regenerates Go protobuf messages. Rust's `envd-protocol` crate consumes the same `spec/process/process.proto`; the existing envd client reexports those generated types and Cloud can depend on the protocol without the HTTP transport. Do not keep a second process schema or manually copied Rust message types. Rust generation uses a locked vendored protoc, so consumers do not require a host compiler installation. Go generation uses pinned `protoc-gen-go` v1.28.1. Publish a new immutable tools drive before any consumer enables this mode; the base upstream commit alone does not identify an overlaid binary.

## Protocol

Start a process with `recoverable_io=true`. The Start event and active List entry expose an opaque random process incarnation. Persist it with the outer sandbox/runtime generation before sending input. Every subsequent operation selects that incarnation, never a PID or tag. Incarnation is a lifecycle fence, not a substitute for the existing guest transport authorization or the outer platform's account, runtime and placement checks.

SendInput uses consecutive sequence numbers beginning at one. One in-guest journal serializes lookup and the actual stdin/PTY write. A repeated sequence with the same channel and SHA-256 payload returns its retained receipt without another pipe write; different bytes/channel conflict. A receipt confirms bytes written into a process pipe, not a completed application command or exactly-once external effects. A partial write, error or panic leaves input uncertain and prevents further input. Losing the HTTP response must retry the same sequence and bytes, not allocate a new sequence. Closing stdin is serialized with writes and replays its close error; retained successful write receipts remain available after closure.

Connect requires `after_sequence`, initially zero, meaning the last fully consumed output frame. Stdout/stderr/PTY and the terminal event share one ordered journal. Start/keepalive events have sequence zero and are not retained frames. Consumers durably advance their cursor only after consuming a frame. The journal captures its notification channel and read position under one lock, so reconnect cannot lose a wakeup. Output producers do not wait for subscribers; a slow subscriber gets explicit expiration instead of silently missing output or blocking the application. Multiple readers can independently replay retained frames.

The journal constants in `internal/processio/journal.go` bound each write, retained receipts and output history. `internal/services/process/recovery.go` bounds active plus retained exited processes and their exit retention. At the current limits this is 1 MiB per write, 1,024 receipts, 8 MiB/4,096 output frames, 64 processes and ten minutes after exit. Old input receipts and output cursors expire explicitly. The process cap is reserved before starting any process, including PTY creation. A reused PID cannot evict another incarnation's registry entry.

| Condition | Connect error code |
| --- | --- |
| Unknown or expired process incarnation | NotFound |
| Invalid sequence or missing output cursor | InvalidArgument |
| Same input sequence, conflicting payload or channel | AlreadyExists |
| Expired output cursor or input receipt | OutOfRange |
| Retention/process/write limit | ResourceExhausted |
| Wrong selector, uncertain/closed input, legacy bypass | FailedPrecondition |

Start itself is not idempotent. A lost Start response does not authorize another Start; an outer operation must discover its exact process or report uncertainty. Active List entries aid discovery but a user tag is not a unique creation receipt. VM/supervisor loss ends the authority. Snapshot rollback/fork can restore or duplicate journal memory; the outer runtime-generation and single-writer fences must reject stale or concurrent copies before using any receipt. This protocol does not claim region recovery or transparently repeat application prompts after authority loss.

## Existing consumers and transition

The current `src/sandbox/process.rs`, `crates/aenv` exec/connect commands and existing envd lifecycle tests use best-effort process mode. They remain explicit consumers of PID/tag and streaming input. Recoverable processes reject those interfaces rather than allowing a legacy caller to bypass deduplication. Remove the legacy branch only when these actual consumers have migrated and published clients no longer require it; it is not a second platform execution path. New Cloud PI recovery must opt into the same runtime execution path after the tools release, using the shared generated wire types.

Process RPC interceptors omit request/response payloads, and recoverable handlers omit raw command/output payload logging. Operation identifiers, procedure names, lifecycle and error codes remain observable. This does not claim that arbitrary application logs or all upstream legacy logging are content-free.

## Validation

`go -C tools-image/envd-overlay test -race ./internal/processio` exercises the journal without a guest. The tools-image build applies the overlay to a fresh pinned checkout, runs Linux race-enabled process/handler/logging unit tests, then compiles envd. Local Linux cross-compilation is a build check, not execution of those Linux tests. The final runtime qualification must use reviewed isolated EKS jobs: lost input acknowledgment, reconnect during output, expired cursor, controller replacement, incarnation end/reuse, unchanged runtime authority and cleanup. No serving consumer or recovery guarantee is enabled by this source change alone.
