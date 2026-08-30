# syntax=docker/dockerfile:1.7

# AgentENV envd tools drive.
#
# This image is a reproducible build recipe for the small ext4 tools drive that
# AgentENV attaches to each Firecracker guest as /dev/vda. The guest boots from
# this drive, pivots into the user rootfs on /dev/vdb, then keeps /agentenv
# bind-mounted so the platform init and compiled envd binary remain available.
#
# The final artifact stage is intentionally a scratch image containing only:
#
#   /tools.ext4
#
# AgentENV's setup path consumes prebuilt images with this shape by extracting
# /tools.ext4 from the OCI image. Normal server startup does not rebuild this
# Dockerfile; this source exists so the runtime artifact can be audited and
# reproduced.
#
# Build example:
#
#   make -C tools-image TOOLS_VERSION=0.1.0
#
# Important build args:
#   TOOLS_VERSION        Immutable AgentENV tools drive release version.
#   ENVD_REF             Commit to build from e2b-dev/infra.
#   ENVD_UPSTREAM_REPO   envd source repository. Override for forks.
#   GO_IMAGE             Digest-pinned Go toolchain image.
#   DEBIAN_IMAGE         Digest-pinned Debian build image.
ARG GO_IMAGE=golang:1.25.9-bookworm@sha256:298734aec230b5f3e8cee450ce6d7eccc39f1797ba548ee90d57e9803030c6c3
ARG DEBIAN_IMAGE=debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171
ARG TOOLS_VERSION=0.1.0

# ---------------------------------------------------------------------------
# Stage 1: compile the static AgentENV guest init.
# ---------------------------------------------------------------------------
FROM --platform=$BUILDPLATFORM ${GO_IMAGE} AS init-builder

ARG TARGETARCH

WORKDIR /src

COPY agentenv-init/go.mod agentenv-init/go.sum ./

RUN --mount=type=cache,target=/go/pkg/mod go mod download

COPY agentenv-init/ ./

RUN --mount=type=cache,target=/root/.cache/go-build \
    --mount=type=cache,target=/go/pkg/mod \
    set -eux; \
    case "$TARGETARCH" in \
      amd64|arm64) ;; \
      *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    CGO_ENABLED=0 GOOS=linux GOARCH="$TARGETARCH" \
      go build -trimpath -ldflags "-s -w" -o /out/agentenv-init .; \
    chmod 0755 /out/agentenv-init

# ---------------------------------------------------------------------------
# Stage 2: compile envd from upstream source.
#
# The tools drive build pins an upstream ref, compiles a static envd binary,
# and injects the resolved git commit into the binary for `envd -commit`.
# ---------------------------------------------------------------------------
FROM --platform=$BUILDPLATFORM ${GO_IMAGE} AS envd-builder

ARG ENVD_REF
ARG ENVD_UPSTREAM_REPO
ARG TARGETARCH

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src

RUN set -eux; \
    test -n "$ENVD_REF"; \
    test -n "$ENVD_UPSTREAM_REPO"; \
    git init infra; \
    cd infra; \
    git remote add origin "$ENVD_UPSTREAM_REPO"; \
    git fetch --depth 1 origin "$ENVD_REF"; \
    git checkout --detach FETCH_HEAD

WORKDIR /src/infra/packages/envd

RUN --mount=type=cache,target=/root/.cache/go-build \
    --mount=type=cache,target=/go/pkg/mod \
    set -eux; \
    case "$TARGETARCH" in \
      amd64|arm64) ;; \
      *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    commit_sha="$(git -C /src/infra rev-parse --short HEAD)"; \
    CGO_ENABLED=0 GOOS=linux GOARCH="$TARGETARCH" \
      go build \
        -ldflags "-X=main.commitSHA=${commit_sha} -s -w" \
        -o /out/envd \
        .; \
    chmod 0755 /out/envd; \
    build_arch="$(go env GOARCH)"; \
    if [ "$TARGETARCH" = "$build_arch" ]; then \
      /out/envd -version; \
      /out/envd -commit; \
    else \
      echo "Skipping envd execution smoke test for cross-built ${TARGETARCH} binary on ${build_arch} builder"; \
    fi

# ---------------------------------------------------------------------------
# Stage 3: assemble the guest-visible tools rootfs.
#
# The Debian stage only supplies build-time file utilities. The exported rootfs
# contains the static init, envd, version metadata, mount points, and symlinks.
# ---------------------------------------------------------------------------
FROM --platform=$BUILDPLATFORM ${DEBIAN_IMAGE} AS rootfs

ARG TOOLS_VERSION

COPY --chmod=0755 --from=init-builder /out/agentenv-init /tools-rootfs/agentenv/agentenv-init
COPY --chmod=0755 --from=envd-builder /out/envd /tools-rootfs/agentenv/envd

RUN set -eux; \
    mkdir -p \
      /tools-rootfs/dev \
      /tools-rootfs/etc \
      /tools-rootfs/mnt/user \
      /tools-rootfs/proc \
      /tools-rootfs/sys/fs/cgroup; \
    printf '%s\n' "$TOOLS_VERSION" > /tools-rootfs/agentenv/tools-drive-version; \
    ln -sf /proc/self/mounts /tools-rootfs/etc/mtab; \
    ln -sf agentenv/agentenv-init /tools-rootfs/init

# ---------------------------------------------------------------------------
# Stage 4: materialize the rootfs as an ext4 disk image.
#
# We do this inside Docker so contributors do not need host e2fsprogs. The
# image gets 50% content headroom and a 16 MiB floor to leave room for ext4
# metadata, journal, and inode tables.
# ---------------------------------------------------------------------------
FROM --platform=$BUILDPLATFORM ${DEBIAN_IMAGE} AS ext4-builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends e2fsprogs findutils \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rootfs /tools-rootfs /rootfs

RUN set -eux; \
    mkdir -p /out; \
    content_bytes="$(find /rootfs -type f -printf '%s\n' | awk '{sum += $1} END {print sum + 0}')"; \
    image_bytes="$((content_bytes * 3 / 2))"; \
    image_size_mb="$(((image_bytes + 1048575) / 1048576))"; \
    if [ "$image_size_mb" -lt 16 ]; then image_size_mb=16; fi; \
    dd if=/dev/zero of=/out/tools.ext4 bs=1M count="$image_size_mb" status=none; \
    mkfs.ext4 -q -d /rootfs -L tools /out/tools.ext4; \
    chmod 0644 /out/tools.ext4

# ---------------------------------------------------------------------------
# Stage 5: export contract.
#
# Keep the final stage tiny and stable: consumers should only rely on
# /tools.ext4 being present at the root of this image.
# ---------------------------------------------------------------------------
FROM scratch AS artifact

ARG TOOLS_VERSION
LABEL org.opencontainers.image.title="AgentENV tools drive" \
      org.opencontainers.image.version="${TOOLS_VERSION}"

COPY --from=ext4-builder /out/tools.ext4 /tools.ext4
# This artifact image is never meant to run. Keep an invalid command so an
# accidental `docker run` fails immediately instead of appearing useful.
CMD ["/nonexistent"]
