#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export CI="${CI:-true}"

run() {
  printf '\n==> %s\n' "$*"
  "$@"
}

run_in() {
  local dir="$1"
  shift
  printf '\n==> (%s) %s\n' "$dir" "$*"
  (cd "$dir" && "$@")
}

run cargo test --manifest-path "$root/plugin-api/sdk/rust/Cargo.toml" --all-targets
run cargo package --manifest-path "$root/plugin-api/sdk/rust/Cargo.toml" --allow-dirty --no-verify --list
run cargo package --manifest-path "$root/plugin-api/sdk/rust/Cargo.toml" --allow-dirty --no-verify

run corepack pnpm --dir "$root/plugin-api/sdk/ts" install --frozen-lockfile
run corepack pnpm --dir "$root/plugin-api/sdk/ts" test
run corepack pnpm --dir "$root/plugin-api/sdk/ts" build
run_in "$root/plugin-api/sdk/ts" npm pack --dry-run

run_in "$root/plugin-api/sdk/go" go test ./...
run_in "$root/plugin-api/sdk/go" go vet ./...

echo "plugin SDK release verification passed"
