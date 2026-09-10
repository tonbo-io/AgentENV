#!/usr/bin/env bash
# Put /tmp on a filesystem whose block device reports 512-byte logical sectors.
#
# The OverlayBD local backend issues O_DIRECT I/O aligned to 512 bytes, which a
# 4096-byte-sector device (Depot runners' root disk) rejects with EINVAL. When
# the device behind /tmp uses larger sectors, mount a loop-backed ext4 with
# 512-byte sectors over /tmp. Mounting over /tmp rather than exporting TMPDIR
# matters: the capability runner re-executes test binaries through sudo, whose
# env_reset drops TMPDIR even with -E, so an environment variable never reaches
# the tests. Production hosts and GitHub-hosted runners report 512-byte sectors
# and are left untouched.
set -euo pipefail

device=$(findmnt -no SOURCE --target /tmp)
sector=$(sudo blockdev --getss "$device" 2>/dev/null || echo 512)
if [[ "$sector" == "512" ]]; then
  echo "/tmp is on $device with 512-byte sectors; nothing to do"
  exit 0
fi
image="${RUNNER_TEMP:?RUNNER_TEMP is required}/tmp-512.img"
truncate -s 24G "$image"
loop=$(sudo losetup --find --show --sector-size 512 "$image")
sudo mkfs.ext4 -q -F "$loop"
sudo mount "$loop" /tmp
sudo chmod 1777 /tmp
echo "/tmp was on $device with $sector-byte sectors; now on $loop (512-byte sectors)"
