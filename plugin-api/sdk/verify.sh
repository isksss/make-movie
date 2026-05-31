#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sdk_root="$root/sdk"
wit="$root/plugin.wit"

required_wit_patterns=(
  "package mm:plugin;"
  "record plugin-metadata {"
  "metadata: func() -> plugin-metadata;"
  "initialize: func();"
  "shutdown: func();"
)

for pattern in "${required_wit_patterns[@]}"; do
  grep -Fq "$pattern" "$wit"
done

before_diff=""
after_diff=""
if git -C "$root/.." rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  before_diff="$(mktemp)"
  after_diff="$(mktemp)"
  trap 'rm -f "$before_diff" "$after_diff"' EXIT
  git -C "$root/.." diff -- plugin-api/sdk >"$before_diff"
fi

bash "$sdk_root/generate.sh"

required_files=(
  "$sdk_root/README.md"
  "$sdk_root/generate.sh"
  "$sdk_root/rust/Cargo.toml"
  "$sdk_root/rust/README.md"
  "$sdk_root/rust/src/lib.rs"
  "$sdk_root/rust/examples/minimal_plugin.rs"
  "$sdk_root/go/go.mod"
  "$sdk_root/go/README.md"
  "$sdk_root/go/plugin.go"
  "$sdk_root/go/plugin_test.go"
  "$sdk_root/go/example_test.go"
  "$sdk_root/ts/README.md"
  "$sdk_root/ts/package.json"
  "$sdk_root/ts/tsconfig.json"
  "$sdk_root/ts/tsconfig.test.json"
  "$sdk_root/ts/src/index.ts"
  "$sdk_root/ts/src/index.test.ts"
  "$sdk_root/ts/examples/minimal-plugin.ts"
  "$sdk_root/csharp/mm-sdk-csharp.csproj"
  "$sdk_root/csharp/Plugin.cs"
)

for file in "${required_files[@]}"; do
  test -f "$file"
done

grep -Fq "PluginMetadata" "$sdk_root/rust/src/lib.rs"
grep -Fq "metadata_free" "$sdk_root/rust/src/lib.rs"
grep -Fq "Metadata" "$sdk_root/go/plugin.go"
grep -Fq "MustMetadataJSON" "$sdk_root/go/plugin.go"
grep -Fq "metadataToJson" "$sdk_root/ts/src/index.ts"
grep -Fq "PluginMetadata" "$sdk_root/ts/src/index.ts"
grep -Fq "Shutdown" "$sdk_root/csharp/Plugin.cs"

if [ -n "$before_diff" ] && [ -n "$after_diff" ]; then
  git -C "$root/.." diff -- plugin-api/sdk >"$after_diff"
  diff -u "$before_diff" "$after_diff" >/dev/null
fi

echo "plugin SDK generation verification passed"
