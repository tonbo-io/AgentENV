# Snapshot Layer GC

The OSS snapshot repository stores rootfs, attached-drive, memory and tools-drive layers once, content-addressed, under `managed-layers/sha256:<digest>`, and shares them across snapshots. Deleting a snapshot removes its alias, catalog record, disk publications and `artifacts/{id}/` prefix, never its managed layers. Managed-layer GC removes the layers nothing needs any more. These layers hold guest memory images and disk deltas, so retaining them after their snapshots are deleted keeps customer data.

A catalog-only GC is not safe: a restored guest reads its source snapshot's memory and rootfs lowers lazily from object storage for its whole lifetime, and a caller may delete the source snapshot right after the restore commits. Node processes therefore publish leases, and deleting runners publish deletion intents first.

Code: `src/snapshot/repository/backends/oss/{layer_refs,layer_store,layer_leases,layer_gc,boot_clock}.rs`.

## Modes

`[snapshot.layer_gc].mode` decides how far a node process takes part. Only `delete` changes what serving paths do.

| | `off` (default) | `report` | `delete` |
| --- | --- | --- | --- |
| Restore | As without GC: managed rootfs, drive and memory lowers are HEADed inside the image-config cache fill on a cache miss. | As `off`, plus an in-memory hold of the restore's layers (no request, no wait, no failure). | Hold, then the gate (below) as one more branch of the restore's concurrent fetches. A missing layer fails with `ArtifactNotFound`; any other gate failure is a retryable backend error. |
| Publication | As without GC. | As `off`, plus an in-memory hold of every layer the record names until the record write. | Hold and gate every layer the record names before the alias and record writes; failures are retryable backend errors raised before the record exists. |
| Resume of a persisted paused sandbox | As without GC. | As without GC. | Gated only while the background startup check has not verified the sandbox; only its remote managed lowers are checked. A missing layer fails the resume (`SandboxOperationFailed(Resume)`) and leaves the sandbox paused. |
| Startup | As without GC. | Nothing extra; the first refresh covers persisted paused sandboxes. | Restored paused sandboxes start unverified and are verified in a background task. Nothing blocks the API listener. |
| Shutdown | As without GC. | Sends a stop signal; waits for nothing. | Sends a stop signal; waits for nothing. A pass stops issuing deletes; an interrupted pass leaves its intent for any runner's stale-intent cleanup. |
| Background | Nothing. | Lease refresher, the orchestrator's 60 s runtime refresh, report passes. | As `report`, with delete passes. |
| Writes under `layer-gc/` | None. | Its lease (declaring `report`), its run report, removal of stale intents and week-old leases. | As `report` with a lease declaring `delete`, plus deletion intents. |

The gate (rule N1) makes no request when every needed digest was verified by an earlier gate and the lease holding it was written successfully less than 12 hours ago (boot clock). Otherwise it makes the digests durable in the node's lease (at most one coalesced PUT), lists `layer-gc/intents/` once and reads each listed intent (16 concurrent), sleeps 120 s only when an intent names a needed digest, HEADs the unverified digests (16 concurrent, stopping at the first error) and marks them verified. Every request has a 15 s timeout. In a restore, nothing reads a managed layer before the resolve returns, and the resolve returns only after the gate and every fetch succeeded, so the gate runs alongside the vm_state, manifest, image-config and tools-drive fetches; materialization skips its own HEADs because the gate checks them. The tools drive is not gated on restore: restore downloads it and checks its digest, so it is never read lazily.

## What keeps a layer

A pass recomputes holders from scratch; there are no persistent counters to drift.

- **Catalog records** (`catalog/records/*.json`). The digest set is the union of a structured extraction of the committed payload (rootfs layers whether managed or external, attached-drive layers, memory layers, tools drive) and a raw scan of the record bytes for `sha256:<64 lowercase hex>`. A committed record that yields no digest aborts the pass. A record that does not parse makes a delete pass degrade (G6).
- **Aliases hold nothing.** Publication binds the alias before it writes the record, inside its held pre-commit window.
- **Node leases** (`layer-gc/leases/{node}-{instance}[-g{n}].json`, version 2), written only by `report` and `delete` processes. Each declares its writer's `mode` and lists every digest the process may read:
  - the runtime set: every lower `digest`/`targetDigest` in the overlaybd image configs (rootfs, attached drives and memory) of running sandboxes and of paused sandboxes in the store, refreshed every 60 s by the orchestrator (config files are read off the async runtime; a sandbox whose handle is locked by a long operation is skipped, and while any is skipped the set only grows);
  - active holds: a restore's hold lives in the `RunnableSnapshot` until launch completes (template builds hold their base snapshot through publication); a publication's hold covers its record write;
  - a 2-hour tail of every digest that left the runtime set or lost its last hold, which bridges hand-offs (hold dropped before the next runtime refresh, pause and detach, a record written after a GC's catalog read).

  The resolver records which digests each materialized runtime image config names, so an evicted memory config in the local artifact cache still contributes its digests. A lease is live while its object-store `LastModified` is within 24 h of the GC's reference time. The refresher rewrites it promptly on growth (coalesced), on shrink after 5 minutes, and at least hourly; after a failed write it waits a full minute before writing again. A write that fails (for example times out) may still land later with older content, so the owner abandons that key and writes the next lease to a new `-g{n}` key; the abandoned key keeps every digest that both its last successful content and the failed attempt carry until it expires, and the node keeps treating exactly those digests as durable (and verified ones as verified) while its last success is fresh. A key a GC runner may have removed is never written again.

## Deletion rule

A pass P deletes `managed-layers/{d}` only if all of these hold. `T0` is the `LastModified` of the runner's own lease, rewritten and stat'ed before the catalog listing starts.

| Rule | Condition | Clock |
| --- | --- | --- |
| G1 | `LastModified(d) < T0 - grace`. | Object store |
| G2 | No catalog record read by P names `d`. | — |
| G3 | P's deletion intent `layer-gc/intents/{runner}-{pass}.json` naming `d` was written and stat'ed (`I_lm`) before P started its final lease listing, and no live lease returned by that listing names `d`. | — |
| G4a | After the final lease read, P rewrites the intent with the same bytes and stats it (`T_end`); `T_end - I_lm <= 19 s` (a 20 s budget minus 1 s of `LastModified` precision). | Object store |
| G4b | Each DELETE is issued within 20 s of taking the boot-clock time just before that rewrite was sent; candidates past it are counted as `not_issued` and wait for the next pass. With G4a every DELETE is issued within 41 s of `I_lm`, inside the 45 s issue window. | `CLOCK_BOOTTIME` |
| G5 | `T_end - T0 <= 30 min - 1 s`, so the catalog listing and the end of the final lease read are at most 30 minutes apart. | Object store |
| G6 | Both lease reads found only live leases declaring `mode = "delete"`, and every catalog record parsed. Otherwise the pass degrades to a report pass: no intent (or its intent is removed) and no DELETE, outcome `degraded`. | — |

After its DELETEs, P removes its intent at once when every issued DELETE succeeded; when one failed or timed out it keeps the intent until 120 s after the intent write was acknowledged (boot clock), then removes it. A pass that is interrupted keeps it; any runner removes intents at least 10 minutes old (object-store time).

Node rules, in `delete` mode only:

- **N1.** Before a process trusts that `managed-layers/{d}` exists, either `d` is verified (checked by an earlier N1 run and continuously in a lease key whose last successful write is under 12 h old on the boot clock), or, in order: a lease PUT carrying `d` was acknowledged at `t_L`; the process listed intents at `L > t_L`; it slept 120 s if a listed intent names `d`; it HEADed `d` and refuses it when missing.
- **N2.** `d` stays in the live set while anything on the node can read it, plus a 2-hour tail (boot clock), and leaves the lease only after a 5-minute debounce.

## Why this is safe

Assumptions:

- **A1.** The store has strong read-after-write and list-after-write consistency (AWS S3 since December 2020, Alibaba Cloud OSS). A weaker S3-compatible store voids the argument.
- **A2 (rollout precondition).** While any process runs `delete` on a bucket and prefix, every other process that reads its managed layers runs this release in `delete`, or in `report` with a live lease (which makes every delete pass degrade). `off` processes, older releases and other tools (`aenv-snapshot-image`) are invisible to the GC and must be excluded operationally; see Rollout.
- **A3.** A DELETE lands, if at all, within 60 s after its 15 s client timeout. SigV4's 15-minute request validity bounds the worst case.
- **A4 (liveness).** A delete-mode process rewrites its lease at least every 24 h. Alerted on at 2 h and 6 h.
- **A5.** `LastModified` has whole-second precision; every object-store comparison carries 1 s of slack.

Suppose P deletes `d` at time `x`.

1. **Gated readers.** Let R trust `d`, with `d` durable in R's lease from `t_L` (A4 keeps that key live). If `t_L` is before P's final listing began, that listing read R's lease after `t_L` and found `d` (whatever late write lands under the key, it still carries `d`), so P skipped `d`. Otherwise `t_L` is after P's intent was committed, and R's intent listing at `L > t_L` either saw P's intent, so R HEADed `d` at least 120 s after `I_lm`, by which time every DELETE of P had landed (issued within 45 s of `I_lm`, timed out within 15 s more, landed within 60 s more by A3); or did not see it, because P removed it after all its DELETEs succeeded (before R's HEAD) or no earlier than 120 s after the intent write (after every landing). Either way R's HEAD observes the deletion and R refuses `d`. If R trusted `d` because it was verified, apply the same argument to the N1 run that verified it.
2. **Records.** Let a committed record naming `d` exist at `x`. If it existed when P listed the catalog, P read `d` (G2). Otherwise it was written later by a publisher that, by A2 and G6, gates (a report-mode publisher has a live report lease, and P would have degraded). The publication made `d` durable at `t_P` before the record write `w` and held it until at least `w + 2 h`; P's final lease read ended at most 30 minutes after the catalog listing (G5), which is before `w + 2 h`. If `t_P` precedes P's final listing, P saw `d`; otherwise the publication's gate ran after it and, as in step 1, observed the deletion and failed before the record was written.
3. **Formats and grace.** Unparsed records degrade the pass (G6); unknown fields in parsed records are covered by the raw scan; a committed record naming no digest aborts the pass. G1 is not needed for steps 1 and 2; it covers multipart uploads in progress (their `LastModified` is the initiation time) and writers outside the protocol.

This is a Dekker pair: delete-mode nodes write their lease and then read intents; GC runners write their intent and then read leases.

`off` and `report` never delete a managed layer: `off` constructs no GC, and a report pass returns before the delete phase. Their only deletes are intents at least 10 minutes old and leases at least 7 days old (object-store time); such intents are past every landing bound and expired leases protect nothing.

## Clocks

- Object-store times are compared only with object-store times: G1, G4a, G5, lease liveness and hygiene.
- In-process durations that bound safety use `CLOCK_BOOTTIME`, which keeps counting while the host is suspended or hibernated: G4b, the intent hold, the gate's 12-hour freshness, the tail and the shrink debounce.
- Tokio sleeps are used only for waits (the node's 120 s intent wait); a clock that stops only lengthens the real wait. The pass-level Tokio timeouts are liveness guards, not safety rules.
- Residual: no in-guest clock sees a hypervisor pause of a VM-hosted process. The object-store checks cover everything up to the pre-delete rewrite; a VM pause of a runner inside the 20 s delete phase, or of a node for more than 12 hours, is not covered. Tonbo's hosts are EC2 bare metal (`m6g.metal`), which cannot be paused or hibernated; deployments on VMs that can be paused depend on this not happening.

## Configuration

`[snapshot.layer_gc]` (see the [configuration reference](../configuration/reference.md#snapshotlayer_gc)):

| Key | Default | Notes |
| --- | --- | --- |
| `mode` | `off` | `off`, `report` or `delete`. Env `AENV_SNAPSHOT_LAYER_GC_MODE`. |
| `interval_secs` | `3600` | At least 300; ±10% jitter; the first pass runs at a random point 5 to 10 minutes after start. |
| `grace_secs` | `86400` | At least 3600. |
| `max_deletes_per_pass` | `1000` | 1 to 10000; oldest candidates first. |

Protocol constants live in code: lease TTL 24 h, liveness rewrite 1 h, shrink debounce 5 min, tail 2 h, gate freshness 12 h, intent wait 120 s, issue window 45 s (pre-delete budget 20 s, delete phase 20 s), request timeout 15 s, pass span 30 min, stale intents 10 min, stale leases 7 days.

The GC runs inside the node server process; there is no separate worker. A runner skips its pass when another runner of at least its mode reported `ok` within half an interval (a `degraded` run counts as a report run). This is best effort and not needed for safety: intents are per pass, runners never write leases or records, and DELETE is idempotent.

Each process start (for example every scale-to-zero wake) and each failed write creates a new lease key. Keys stay live for 24 h and any runner removes them after 7 days; in a versioned bucket every rewrite also leaves a noncurrent version for the bucket's lifecycle window. A delete pass reads every live lease.

## Rollout and rollback

1. Every process that reads managed layers from a bucket and prefix must run this release in `report` or `delete` before any process enables `delete` (A2). `off` processes, older releases and `aenv-snapshot-image` are invisible to the GC. Verify with a read-only listing of `layer-gc/leases/` owners against the cell's node inventory, and confirm that no other process or cluster generation reads the prefix.
2. Order: `off` to `report` everywhere, then `delete` everywhere. Never `off` straight to `delete`.
3. After switching to `delete`, passes stay `degraded` until every report-mode lease has expired, up to 24 h after the last report-mode write. The run report's `non_gating_sample` lists the blocking keys.
4. Rollback: `delete` to `report` everywhere first; only after no process runs `delete` may any process run `off` or an older release. Older releases ignore `layer-gc/` objects.

Deletes use `DeleteObject` without a version id. In a versioned bucket the bytes stay as a noncurrent version under the bucket's lifecycle rule; recovering a wrongly deleted layer means copying that version back (or removing the delete marker) with a principal that holds `GetObjectVersion`, which the node role does not. The run report and the per-layer audit log lines list what each pass deleted.

## Tonbo Cloud

Tonbo's idle reclamation publishes every paused guest as a catalog record before its host leaves, and every wake restores from those records. Layers needed after a host is gone are therefore protected by records, not by that host's lease; a lease only protects what its live process holds (running guests, persisted paused sandboxes, in-flight restores and publications). A host that is gone or stopped exports no metrics, so the lease-age alert below pairs with node-down alerting.

## The snapshot image export tool

`aenv-snapshot-image` reads managed layers without a lease, and the grace period does not protect it (it only exempts objects younger than `grace_secs`). If its snapshot is deleted while it exports, a later delete pass may remove the layers and the export fails cleanly (`ManagedLayerNotFound`, or `IntegrityMismatch` on a short read). Export snapshots that are not being deleted.

## Observability and alerts

Every pass logs `snapshot layer gc pass complete` (warn on abort) with mode, runner, pass id, outcome, the degrade reason, duration and every count below; delete passes also log `snapshot layer gc deleted managed layer` per object (digest, size, `LastModified`). The run report `layer-gc/runs/{runner}.json` carries the same counts, up to 20 garbage digests, up to 20 non-gating lease keys and the full deleted list. Restores log the phase `managed_layers_gated` (outcome `fast`, `cold`, `intent_wait` or `failed`, and elapsed time); delete-mode publications log the same phase.

Metrics on the node's `/metrics`:

- `agentenv_snapshot_layer_lease_age_seconds`: boot-clock seconds since this process's last successful lease write; normally under one hour.
- `agentenv_snapshot_layer_lease_writes_total{reason,outcome}`, `agentenv_snapshot_layer_lease_digests`
- `agentenv_snapshot_layer_gate_seconds{path,outcome}` and `agentenv_snapshot_layer_gate_failures_total{path,reason}` for `path` in `restore`, `publish`, `startup`, `resume` (delete mode only)
- `agentenv_snapshot_layer_intent_waits_total{path}`
- `agentenv_snapshot_layer_gc_passes_total{mode,outcome}` with `outcome` in `ok`, `degraded`, `skipped`, `aborted`
- `agentenv_snapshot_layer_gc_non_gating_leases`
- `agentenv_snapshot_layer_gc_objects{class}` and `agentenv_snapshot_layer_gc_bytes{class}` for `referenced`, `leased`, `young`, `unrecognized` and `garbage`
- `agentenv_snapshot_layer_gc_deleted_objects_total`, `agentenv_snapshot_layer_gc_deleted_bytes_total`, `agentenv_snapshot_layer_gc_delete_failures_total`, `agentenv_snapshot_layer_gc_not_issued_total`
- `agentenv_snapshot_layer_gc_last_success_timestamp_seconds`
- `agentenv_snapshot_layer_retention_contended_total`: runtime refreshes that could not observe every sandbox and only grew the set
- `agentenv_snapshot_layer_lease_unreadable_image_configs_total` (each such config is also logged at warn, at most hourly per path)

Recommended alerts:

| Condition | Severity |
| --- | --- |
| `agentenv_snapshot_layer_lease_age_seconds > 7200` for 15 min | Warn |
| `agentenv_snapshot_layer_lease_age_seconds > 21600` (6 h, well under the 24 h TTL) | Page |
| Restore gate p99 above the cold-restore budget, or gate failures for 15 min | Ticket |
| Three consecutive `aborted` passes | Ticket |
| `degraded` passes more than 48 h after enabling `delete` | Ticket |
| `agentenv_snapshot_layer_gc_last_success_timestamp_seconds` older than three intervals | Ticket |

## Rules for new code

A path that reads managed layers must `hold()` them for as long as it relies on them and, when the leases are gating (`is_gating()`), `gate()` them before trusting that they exist, or be covered by the runtime set. Reference extraction lives only in `layer_refs.rs`. Records must keep naming their layers as `sha256:<64 lowercase hex>` tokens; a parseable record that names layers any other way is invisible to older runners and needs a coordinated runner change first.
