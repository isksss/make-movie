#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sdk_root="$root/sdk"
wit="$root/plugin.wit"

required_wit_patterns=(
  "package mm:plugin;"
  "metadata: func() -> string;"
  "initialize: func();"
  "shutdown: func();"
)

for pattern in "${required_wit_patterns[@]}"; do
  grep -Fq "$pattern" "$wit"
done

bash "$sdk_root/generate.sh"

required_files=(
  "$sdk_root/README.md"
  "$sdk_root/generate.sh"
  "$sdk_root/rust/Cargo.toml"
  "$sdk_root/rust/src/lib.rs"
  "$sdk_root/go/go.mod"
  "$sdk_root/go/plugin.go"
  "$sdk_root/ts/package.json"
  "$sdk_root/ts/tsconfig.json"
  "$sdk_root/ts/src/index.ts"
  "$sdk_root/csharp/mm-sdk-csharp.csproj"
  "$sdk_root/csharp/Plugin.cs"
)

for file in "${required_files[@]}"; do
  test -f "$file"
done

grep -Fq "metadata" "$sdk_root/rust/src/lib.rs"
grep -Fq "Initialize" "$sdk_root/go/plugin.go"
grep -Fq "shutdown" "$sdk_root/ts/src/index.ts"
grep -Fq "Shutdown" "$sdk_root/csharp/Plugin.cs"

if git -C "$root/.." rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git -C "$root/.." diff --exit-code -- plugin-api/sdk
fi

echo "plugin SDK generation verification passed"
