# AgentENV Tools Drive

This directory builds the small ext4 tools drive attached to every Firecracker guest as `/dev/vda`. Normal server startup consumes the immutable image configured in `config/deps_manifest.toml`.

The drive contains two static platform binaries:

- `agentenv-init` runs as guest PID 1, mounts the user root and attached drives, pivots into the user filesystem, configures the minimal runtime mounts and network files, reaps orphaned children, and starts `envd`.
- `envd` exposes the guest control API and launches user processes.

`agentenv-init` is the sole guest init. A running sandbox contains `agentenv-init`, `envd`, and the user processes requested through `envd`. PID 1 writes envd stdout and stderr to two rotating 512 KiB segments at `/run/agentenv/envd.log` and `/run/agentenv/envd.log.1`, which keeps verbose process-event logging bounded in the guest tmpfs and off the Firecracker serial path without adding a logging daemon. PID 1 fails bootstrap and powers off before starting `envd` when a declared drive or subpath cannot be mounted, loopback cannot be enabled, DNS cannot be installed, or the required devpts and shared-memory filesystems cannot be mounted. Applications therefore cannot silently write durable data into the temporary rootfs or report ready with missing guest facilities. If `envd` exits, PID 1 also powers off the guest and the host observes that runtime identity as unavailable.

The reserved `agentenv_bootstrap_failpoint=<step>` kernel argument is used only by the KVM integration suite to prove that each critical bootstrap failure prevents `envd` from starting. Guest callers cannot alter the Firecracker kernel command line.

## Build

Requirements:

- Docker with Buildx.
- Go only for direct `agentenv-init` checks; Docker supplies the pinned build toolchain for the drive.

From this directory:

```bash
make
```

From the repository root:

```bash
make -C tools-image
```

The output is `tools-image/out/tools-<TOOLS_VERSION>-<ARCH>.ext4`.

Run the static init checks without building a container:

```bash
make check
```

## Versioning

`TOOLS_VERSION` identifies the complete drive, including `agentenv-init` and `envd`. Published versions are immutable, so every content change requires a new version. `envd-source.env` is the authoritative immutable upstream repository and commit used to compile `envd`; the commit may differ from the version reported by the binary.

Official releases use versions such as `0.1.0`. Custom distributions use unique prerelease versions such as `0.1.0-tonbo.2`.

AgentENV records the expected in-guest envd version under `[envd].version`. For a native Docker build, the build log prints both `/out/envd -version` and `/out/envd -commit`. For a cross build, inspect those values on a matching Linux host.

## Configuration

| Variable | Default | Description |
|---|---|---|
| `TOOLS_VERSION` | `0.1.0` | Immutable SemVer for the complete tools drive |
| `ARCH` | Host architecture normalized to `amd64` or `arm64` | Build architecture |
| `PUBLISH_PLATFORMS` | `linux/amd64,linux/arm64` | Platforms in a published OCI manifest |
| `OUTPUT_DIR` | `out` | Export directory |
| `OUTPUT_NAME` | `tools-${TOOLS_VERSION}-${ARCH}.ext4` | Export filename |
| `IMAGE` | `agentenv-tools:${TOOLS_VERSION}` | Local or remote image tag |
| `DOCKER` | `docker` | Docker CLI |

Example:

```bash
make TOOLS_VERSION=0.1.0-tonbo.2 ARCH=amd64
```

## Publish

The reviewed `Publish Tools Image` workflow builds native `amd64` and `arm64` artifacts and creates the multi-platform manifest. The local target uses the same immutable-tag contract:

```bash
make publish TOOLS_VERSION=0.1.0-tonbo.2 IMAGE=ghcr.io/tonbo-io/agentenv-tools:0.1.0-tonbo.2
```

After publication and runtime validation, update `[tools].version` in `config/deps_manifest.toml`. Snapshots persist `TOOLS_VERSION`; Git revisions and OCI digests provide release provenance.

## Runtime Validation

Point `[tools].drive_path` at a locally built ext4 only on a Linux development host with the required KVM setup. Product validation uses the reviewed isolated workflow and verifies cold boot, command execution, PTYs, DNS, attached drives and subpaths, pause/resume, snapshot restore, the guest process tree, fail-closed behavior when `envd` exits, and fail-closed boot for missing attached-drive devices or subpaths.

Host-side root filesystem resizing remains owned by the OverlayBD toolchain and is outside this guest drive.
