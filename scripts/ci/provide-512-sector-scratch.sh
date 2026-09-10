#!/usr/bin/env bash
# Give tests a temp directory on a filesystem whose block device reports
# 512-byte logical sectors.
#
# The OverlayBD local backend issues O_DIRECT I/O aligned to 512 bytes, which
# a 4096-byte-sector device (Depot runners' root disk) rejects with EINVAL. When
# the device behind $TMPDIR uses larger sectors, mount a loop-backed ext4 with
# 512-byte sectors under the runner temp directory and point TMPDIR at it.
set -euo pipefail

tmp=${TMPDIR:-/tmp}
device=$(findmnt -no SOURCE --target "$tmp")
sector=$(sudo blockdev --getss "$device" 2>/dev/null || echo 512)
if [[ "$sector" == "512" ]]; then
  echo "TMPDIR $tmp is on $device with 512-byte sectors; nothing to do"
  exit 0
fi
scratch="${RUNNER_TEMP:?RUNNER_TEMP is required}/scratch"
image="$scratch.img"
truncate -s 24G "$image"
loop=$(sudo losetup --find --show --sector-size 512 "$image")
sudo mkfs.ext4 -q -F "$loop"
sudo mkdir -p "$scratch"
sudo mount "$loop" "$scratch"
sudo chown "$(id -u):$(id -g)" "$scratch"
echo "TMPDIR=$scratch" >> "${GITHUB_ENV:?GITHUB_ENV is required}"
echo "TMPDIR $tmp is on $device with $sector-byte sectors; tests use $scratch on $loop (512-byte sectors)"
