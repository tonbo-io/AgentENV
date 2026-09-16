#!/usr/bin/env bash
# Apply the reviewed overlay only to its immutable upstream input. Go and Rust
# generate their wire types from envd-overlay/spec/process/process.proto.
set -euo pipefail
source_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$source_dir/envd-source.env"
upstream_dir=${1:?upstream checkout path is required}
[[ $(git -C "$upstream_dir" rev-parse HEAD) == "$ENVD_REF" ]] || {
  echo "envd overlay requires the reviewed upstream commit" >&2
  exit 1
}
git -C "$upstream_dir" apply --check "$source_dir/envd-overlay/upstream.patch"
git -C "$upstream_dir" apply "$source_dir/envd-overlay/upstream.patch"
cp -R "$source_dir/envd-overlay/internal/." "$upstream_dir/packages/envd/internal/"
cp "$source_dir/envd-overlay/spec/process/process.proto" "$upstream_dir/packages/envd/spec/process/process.proto"
cd "$upstream_dir/packages/envd"
protoc --proto_path=spec --go_out=internal/services/spec \
  --go_opt=paths=source_relative \
  --go_opt=Mprocess/process.proto=github.com/e2b-dev/infra/packages/envd/internal/services/spec/process \
  spec/process/process.proto
protoc --proto_path=spec --connect-go_out=internal/services/spec \
  --connect-go_opt=paths=source_relative \
  --connect-go_opt=Mprocess/process.proto=github.com/e2b-dev/infra/packages/envd/internal/services/spec/process \
  spec/process/process.proto
