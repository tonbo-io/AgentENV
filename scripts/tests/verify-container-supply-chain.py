#!/usr/bin/env python3

import re
from datetime import date
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DOCKERFILES = (
    ROOT / "tools-image/Dockerfile",
    ROOT / "deploy/docker/Dockerfile.agentenv",
    ROOT / "deploy/docker/Dockerfile.gateway",
    ROOT / "deploy/docker/Dockerfile.scheduler",
)
DIGEST_REFERENCE = re.compile(r"^.+@sha256:[0-9a-f]{64}$")


def verify_dockerfile(path: Path) -> None:
    arguments: dict[str, str] = {}
    stages: set[str] = set()
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        argument = re.fullmatch(r"ARG ([A-Za-z_][A-Za-z0-9_]*)=(\S+)", line)
        if argument:
            arguments[argument.group(1)] = argument.group(2)
            continue
        if not line.startswith("FROM "):
            continue
        tokens = line.split()
        source_index = next(index for index, token in enumerate(tokens[1:], 1) if not token.startswith("--"))
        source = tokens[source_index]
        variable = re.fullmatch(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}", source)
        if variable:
            source = arguments.get(variable.group(1), "")
        if source != "scratch" and source not in stages and not DIGEST_REFERENCE.fullmatch(source):
            raise SystemExit(f"{path.relative_to(ROOT)}:{line_number}: unpinned external base image {source!r}")
        if len(tokens) > source_index + 2 and tokens[source_index + 1].upper() == "AS":
            stages.add(tokens[source_index + 2])


def verify_grpc_health_probe_checksums() -> None:
    path = ROOT / "deploy/docker/grpc-health-probe-checksums.txt"
    entries = path.read_text(encoding="utf-8").splitlines()
    expected_names = {"grpc_health_probe-linux-amd64", "grpc_health_probe-linux-arm64"}
    actual_names = set()
    for line_number, line in enumerate(entries, 1):
        match = re.fullmatch(r"([0-9a-f]{64})  (grpc_health_probe-linux-(?:amd64|arm64))", line)
        if not match:
            raise SystemExit(f"{path.relative_to(ROOT)}:{line_number}: malformed checksum entry")
        actual_names.add(match.group(2))
    if actual_names != expected_names:
        raise SystemExit(f"{path.relative_to(ROOT)}: expected checksums for {sorted(expected_names)}, got {sorted(actual_names)}")


def verify_envd_source_lock() -> None:
    path = ROOT / "tools-image/envd-source.env"
    values = dict(line.split("=", 1) for line in path.read_text(encoding="utf-8").splitlines())
    if values.get("ENVD_UPSTREAM_REPO") != "https://github.com/e2b-dev/infra.git":
        raise SystemExit(f"{path.relative_to(ROOT)}: unexpected envd source repository")
    if not re.fullmatch(r"[0-9a-f]{40}", values.get("ENVD_REF", "")):
        raise SystemExit(f"{path.relative_to(ROOT)}: ENVD_REF must be a full immutable commit")


def verify_runtime_apt_security_epoch() -> None:
    path = ROOT / "deploy/docker/Dockerfile.agentenv"
    source = path.read_text(encoding="utf-8")
    epoch = re.search(
        r"^ARG UBUNTU_APT_SECURITY_EPOCH=(\d{4}-\d{2}-\d{2})$", source, re.MULTILINE
    )
    if not epoch:
        raise SystemExit(f"{path.relative_to(ROOT)}: runtime apt security epoch must be an ISO date")
    try:
        date.fromisoformat(epoch.group(1))
    except ValueError as error:
        raise SystemExit(
            f"{path.relative_to(ROOT)}: invalid runtime apt security epoch {epoch.group(1)!r}"
        ) from error
    if source.count("${UBUNTU_APT_SECURITY_EPOCH}") != 1:
        raise SystemExit(
            f"{path.relative_to(ROOT)}: apt transaction must consume the security epoch exactly once"
        )


for dockerfile in DOCKERFILES:
    verify_dockerfile(dockerfile)
verify_grpc_health_probe_checksums()
verify_envd_source_lock()
verify_runtime_apt_security_epoch()
print("PASS container base images and downloaded probes are digest verified")
