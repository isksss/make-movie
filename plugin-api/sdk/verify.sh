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

required_files=(
  "$sdk_root/README.md"
  "$sdk_root/rust/Cargo.toml"
  "$sdk_root/rust/src/lib.rs"
  "$sdk_root/go/go.mod"
  "$sdk_root/go/plugin.go"
  "$sdk_root/ts/package.json"
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

echo "plugin SDK scaffold verification passed"
