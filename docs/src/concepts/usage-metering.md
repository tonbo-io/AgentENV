# Usage Metering

Every sandbox runtime on a node is metered for the host resources it actually consumes: CPU time, resident memory, and allocated disk. The counters answer "what did this sandbox use", independent of the `cpuCount` / `memoryMB` / `diskSizeMB` it was created with, so a control plane can bill or schedule by measured use even when sandboxes are provisioned larger than they use and the node is oversubscribed.

---

## What is measured

| Counter | Source | Meaning |
|---------|--------|---------|
| `cpuUsageMicros` | cgroup `cpu.stat` `usage_usec` | Host CPU time consumed by every thread of the Firecracker process, including its vCPU threads. Divide by 1,000,000 for vCPU-seconds. |
| `memoryCurrentBytes` | cgroup `memory.current` | Host memory charged to the Firecracker process right now: anonymous guest memory it has touched plus page cache it owns. Guest memory that was never touched, or was released by the balloon, is not resident and not counted. |
| `memoryByteSeconds` | integral of the above | Resident memory integrated over the runtime instance's lifetime. |
| `diskAllocatedBytes` | `st_blocks` of the runtime work directory | Bytes the sandbox's writable layers occupy on local disk: the overlaybd upper layer of the rootfs, one runtime directory per extra drive, plus logs. Upper layers are sparse, so allocated blocks are counted, not file length. |
| `diskByteSeconds` | integral of the above | Allocated disk integrated over the runtime instance's lifetime. |

Disk is measured from the work directory rather than from the cgroup because the ublk daemon performs the writes on the sandbox's behalf, so the Firecracker cgroup's `io.stat` never sees them.

---

## Runtime instances

A **runtime instance** is one Firecracker process. It starts when the sandbox boots or resumes on this node and ends when that process stops (pause, delete, timeout, or crash). Each instance has its own `runtimeInstanceID` and its counters start from zero; they never decrease within an instance.

After the instance stops, `running` becomes `false` and the counters keep their final values for `metering.finished_retention_secs` (one hour by default), so a control plane that did not itself stop the sandbox can still read the final figures. A sandbox that resumes on another node starts a new instance there; the previous node keeps the old instance's final counters until retention ends.

---

## Reading usage

```
GET /sandboxes/{sandboxID}/usage
```

```json
{
  "sandboxID": "0192c5d4-…",
  "runtimeInstanceID": "0192c5d4-…",
  "running": true,
  "startedAt": "2026-09-03T08:00:00Z",
  "sampledAt": "2026-09-03T08:10:05Z",
  "sampleCount": 122,
  "cgroupAccounting": true,
  "cpuUsageMicros": 184203114,
  "memoryCurrentBytes": 1610612736,
  "memoryByteSeconds": 903471263744,
  "diskAllocatedBytes": 268435456,
  "diskByteSeconds": 141733920768
}
```

The endpoint returns `404` when the sandbox is unknown to this node, or when no runtime instance has been metered here (for example a paused sandbox restored after a server restart that has not resumed yet).

Counters are updated every `metering.sample_interval_secs` (five seconds by default). Byte-seconds charge the gauge that was current at the start of each interval over that interval's length; a gauge that changed between two samples is charged at its earlier value until the later sample observes it. The final sample is taken when the process stops, so the last partial interval is included.

---

## How the cgroup tree is laid out

At startup the server takes over the cgroup v2 directory it was started in, moves itself and every process already there into `agentenv/`, enables the `cpu` and `memory` controllers for its children, and keeps one leaf per sandbox under `sandboxes/`:

```text
<own cgroup>/
├── agentenv/            the server, the ublk daemon, warm-pool Firecrackers
└── sandboxes/
    └── <sandbox id>/    one Firecracker process
```

Only accounting is used; no limit is written to a sandbox leaf. A warm-pool Firecracker starts in `agentenv/` and moves into its sandbox leaf when it is claimed, so a leaf charges the process from the moment it belongs to a sandbox. The leaf is removed when the process exits.

This needs a writable cgroup v2 tree. The privileged runtime container and a root-run server have one; the systemd unit written by `scripts/install.sh` gets one through `Delegate=yes`, which hands the unit's subtree to the service user. Without one, the server logs a warning at startup, `cgroupAccounting` is `false`, the CPU and memory counters are absent, and disk is still measured.

---

## Configuration

```toml
[metering]
enabled = true
sample_interval_secs = 5
cgroup_root = "/sys/fs/cgroup"
finished_retention_secs = 3600
```

`AENV_METERING_ENABLED` and `AENV_METERING_SAMPLE_INTERVAL_SECS` override the first two.
