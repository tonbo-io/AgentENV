#!/usr/bin/env python3
"""Apply the reviewed Firecracker backport to an exact dependency checkout."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parent
source = Path(sys.argv[1]).resolve()
manifest = json.loads((root / "source.json").read_text())
actual = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
if actual != manifest["commit"]:
    raise SystemExit(f"Unexpected Firecracker source: {actual}")
patch = root / "host-clidr.patch"
subprocess.run(["git", "-C", str(source), "apply", "--check", str(patch)], check=True)
subprocess.run(["git", "-C", str(source), "apply", str(patch)], check=True)
old = f'version = "{manifest["base_version"]}"'
new = f'version = "{manifest["version"]}"'
changed = 0
for path in [source / "Cargo.lock", *source.rglob("Cargo.toml")]:
    original = path.read_text()
    if old in original:
        path.write_text(original.replace(old, new))
        changed += 1
if changed < 2:
    raise SystemExit("Expected Firecracker package and lockfile versions were not found")
manifest["patch_sha256"] = hashlib.sha256(patch.read_bytes()).hexdigest()
(source / "dependency-provenance.json").write_text(json.dumps(manifest, indent=2) + "\n")
