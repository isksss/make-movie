#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

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

run cargo fmt --check
run cargo test
run cargo clippy --workspace --all-targets -- -D warnings

run corepack pnpm --dir "$root/apps/mm-gui" test
run corepack pnpm --dir "$root/apps/mm-gui" build
run corepack pnpm --dir "$root/apps/mm-gui" lint
run corepack pnpm --dir "$root/apps/mm-gui" e2e

run corepack pnpm --dir "$root/apps/mm-gui" exec tauri --version
run bash "$root/scripts/prepare-tauri-sidecars.sh"
run cargo test --manifest-path "$root/apps/mm-gui/src-tauri/Cargo.toml"
run cargo clippy --manifest-path "$root/apps/mm-gui/src-tauri/Cargo.toml" --all-targets -- -D warnings

run bash "$root/plugin-api/sdk/generate.sh"
run bash "$root/plugin-api/sdk/verify.sh"
run cargo test --manifest-path "$root/plugin-api/sdk/rust/Cargo.toml"
run_in "$root/plugin-api/sdk/go" go test ./...
run corepack pnpm --dir "$root/plugin-api/sdk/ts" install --silent
run corepack pnpm --dir "$root/plugin-api/sdk/ts" test

if command -v dotnet >/dev/null 2>&1; then
  run dotnet build "$root/plugin-api/sdk/csharp/mm-sdk-csharp.csproj"
else
  printf '\n==> dotnet が見つからないため C# SDK build を skip します\n'
fi

run git -C "$root" diff --check
