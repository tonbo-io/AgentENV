# Snapshot Layer GC

The OSS snapshot repository stores rootfs, attached-drive, memory and tools-drive layers once, content-addressed, under `managed-layers/sha256:<digest>`, and shares them across snapshots. Deleting a snapshot removes its alias, catalog record, disk publications and `artifacts/{id}/` prefix, never its managed layers. Managed-layer GC removes the layers nothing needs any more. These layers hold guest memory images and disk deltas, so retaining them after their snapshots are deleted keeps customer data.

Code: `src/snapshot/repository/backends/oss/{layer_refs,layer_store,layer_leases,layer_gc}.rs`.

## What keeps a layer

A pass recomputes holders from scratch; there are no persistent counters to drift.

- **Catalog records** (`catalog/records/*.json`). The digest set is the union of a structured extraction of the committed payload (rootfs layers whether managed or external, attached-drive layers, memory layers, tools drive) and a raw scan of the record bytes for `sha256:<64 hex>`. The raw scan keeps forward compatibility: fields added by a newer release, and records that fail to parse, still protect their layers. A committed record that yields no digest at all aborts the pass.
- **Aliases hold nothing.** Publication binds the alias before it writes the record, inside its leased pre-commit window.
- **Node leases** (`layer-gc/leases/{node}-{instance}.json`). Every OSS-backed node process writes one, whatever its GC mode. It lists every digest the process may read:
  - the runtime set: every lower `digest`/`targetDigest` in the overlaybd image configs (rootfs, attached drives and memory) of running sandboxes and of paused sandboxes in the store, refreshed every 60 s by the orchestrator;
  - active guards: a restore's guard lives in the `RunnableSnapshot` until launch completes (template builds hold their base snapshot through publication); a publication's pre-commit guard covers its record write;
  - a 2-hour tail of every digest that left the runtime set or lost its last guard, which bridges hand-offs (guard dropped before the next runtime refresh, pause and detach, a record written after a GC's catalog read).

  The resolver records which digests each materialized runtime image config names, so an evicted memory config in the local artifact cache still contributes its digests. A lease is live while its object-store `LastModified` is within 24 h of the GC's reference time. The owner rewrites it on growth (immediately, coalesced), on shrink (after 5 minutes), and at least hourly; after 12 hours without a successful write it rotates to a new key, so a key a GC runner may have removed is never written again.

## Deletion rule

A pass P deletes `managed-layers/{d}` only if all of these hold:

| Rule | Condition |
| --- | --- |
| G1 | Its `LastModified` is older than `T0 - grace`, where `T0` is the `LastModified` of the runner's own lease, rewritten and stat'ed at pass start. |
| G2 | No catalog record read by P names `d`. |
| G3 | P's deletion intent `layer-gc/intents/{runner}-{pass}.json` naming `d` was written and acknowledged before P started its final lease listing, and no live lease returned by that listing names `d`. |
| G4 | The DELETE is issued within 45 s of the intent write returning, with a 15 s request timeout and no retry after the deadline. |
| G5 | The span from P's catalog listing to the end of its final lease read is at most 30 minutes; otherwise P aborts without deleting. |

Node rules:

- **N1.** Before a node trusts that `managed-layers/{d}` exists (restore, writing a record that names `d`, resuming persisted paused state), `d` is durable in its lease, the node has listed intents, and when a live intent names `d` it has waited 120 s. Only then does it check `d`, failing cleanly (restore: `ArtifactNotFound`; publication: a retryable backend error before any record exists) when it is missing. Digests checked earlier and continuously leased since skip the lease write, the intent listing and the check.
- **N2.** `d` stays in the lease while anything on the node can read it, and for 2 hours after.

### Why this is safe

Suppose P deletes `d` at time `x`.

1. A lease that held `d` durably before P's final lease listing began is returned by that listing, so P would have skipped `d`. A node that trusts `d` after `x` therefore made `d` durable after the listing began, which is after P's intent was acknowledged. The node's intent listing then returns P's intent (P removes it only after all its DELETEs completed, and keeps it when one failed or timed out), so the node waits out the deletion window and its check sees `d` missing.
2. A record that names `d` and exists at `x` either existed when P listed the catalog (P read `d`) or was written later by a publication whose pre-commit lease held `d` until the record write plus 2 hours, longer than P's 30-minute span; P's final lease listing saw it unless the lease became durable after that listing began, which case 1 covers.
3. Running guests are covered by case 1 and N2.

This is a Dekker pair over a store with strong read-after-write and list-after-write consistency (AWS S3 since December 2020, Alibaba Cloud OSS): nodes write their lease and then read intents; GC runners write their intent and then read leases. A weaker S3-compatible store invalidates the argument. Correctness never compares two clocks: object ages and lease liveness compare object-store `LastModified` values with the object-store time of the runner's own lease, and every other bound is a monotonic duration within one process. The grace period is not needed for the argument; it protects writers outside the protocol (older releases during a rollout, the `aenv-snapshot-image` export tool) and multipart uploads, whose `LastModified` is their initiation time.

A crash leaves at most a stale intent: nodes take the 120 s slow path for the digests it names until any runner removes intents older than 10 minutes. Concurrent runners are safe: intents are per pass, runners never write leases or records, and DELETE is idempotent. A runner skips its pass when another runner of at least its mode reported success within half an interval (best effort, not needed for safety).

## Modes and configuration

`[snapshot.layer_gc]` (see the [configuration reference](../configuration/reference.md#snapshotlayer_gc)):

| Key | Default | Notes |
| --- | --- | --- |
| `mode` | `off` | `off`, `report` or `delete`. Env `AENV_SNAPSHOT_LAYER_GC_MODE`. |
| `interval_secs` | `3600` | At least 300; ±10% jitter; the first pass runs 10 minutes after start. |
| `grace_secs` | `86400` | At least 3600. |
| `max_deletes_per_pass` | `1000` | 1 to 10000; oldest candidates first. |

`report` runs the full classification, including the lease read, and writes the run report and metrics, but writes no intent and deletes nothing. Protocol constants (lease TTL 24 h, liveness rewrite 1 h, shrink debounce 5 min, tail 2 h, key rotation 12 h, intent wait 120 s, issue window 45 s, request timeout 15 s, pass span 30 min, stale intents 10 min, stale leases 7 days) live in code.

The GC runs inside the node server process; there is no separate worker. It starts after the orchestrator has leased the layers of persisted paused sandboxes. On shutdown it stops issuing deletes and the server waits up to 20 s for in-flight requests.

## Rollout and rollback

1. Ship the lease-writing release with `mode = "report"` everywhere. Report mode never deletes, so mixing old and new nodes is safe.
2. Observe at least two report passes per cell: no aborts, lease age under an hour, `leased` non-zero only while restored guests run.
3. Switch to `mode = "delete"` only after every node writing to the same bucket and prefix (including other cluster generations) runs the lease-writing release. A pre-lease node's restored guests are unleased.
4. To roll back, first set `mode` to `off` or `report` and let it reach every node; only then revert the release.

Deletes use `DeleteObject` without a version id. In a versioned bucket the bytes stay as a noncurrent version under the bucket's lifecycle rule; recovering a wrongly deleted layer means copying that version back (or removing the delete marker) with a principal that holds `GetObjectVersion`, which the node role does not. The run report and the per-layer audit log lines list what each pass deleted.

## Observability

Every pass logs `snapshot layer gc pass complete` (warn on abort) with mode, runner, pass id, outcome, duration and every count below; delete mode also logs `snapshot layer gc deleted managed layer` per object (digest, size, `LastModified`). The run report `layer-gc/runs/{runner}.json` carries the same counts, up to 20 garbage digests and the full deleted list.

Metrics on the node's `/metrics`:

- `agentenv_snapshot_layer_gc_passes_total{mode,outcome}`
- `agentenv_snapshot_layer_gc_objects{class}` and `agentenv_snapshot_layer_gc_bytes{class}` for `referenced`, `leased`, `young`, `unrecognized` and `garbage`
- `agentenv_snapshot_layer_gc_deleted_objects_total`, `agentenv_snapshot_layer_gc_deleted_bytes_total`, `agentenv_snapshot_layer_gc_delete_failures_total`
- `agentenv_snapshot_layer_gc_last_success_timestamp_seconds`
- `agentenv_snapshot_layer_lease_writes_total{reason,outcome}`, `agentenv_snapshot_layer_lease_age_seconds`, `agentenv_snapshot_layer_lease_digests`
- `agentenv_snapshot_layer_intent_waits_total{path}` (`restore`, `publish`, `startup`)
- `agentenv_snapshot_layer_lease_unreadable_image_configs_total`

The one liveness assumption: a live node that cannot write its lease for 24 hours loses protection for guests restored from deleted snapshots. Alert on lease age.

## Rules for new code

Any code path that reads managed layers must lease them first (`LayerLeases::protect` or `protect_and_check`), or be covered by the runtime set. A path that skips it is protected only by the 2-hour tail of whoever leased the layers last.
