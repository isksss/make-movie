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

mkdir -p \
  "$sdk_root/rust/src" \
  "$sdk_root/rust/examples" \
  "$sdk_root/go" \
  "$sdk_root/ts/src" \
  "$sdk_root/ts/examples" \
  "$sdk_root/csharp"

cat >"$sdk_root/README.md" <<'EOF'
# make-movie Plugin SDK

このディレクトリは `plugin-api/plugin.wit` を正本として生成する SDK 雛形を管理します。

## 契約

唯一の ABI 契約は `../plugin.wit` です。SDK は Plugin 実装者が各言語で同じ lifecycle と metadata を実装しやすくするための補助層です。

必須 export:

- `metadata() -> plugin-metadata`
- `initialize()`
- `shutdown()`

## SDK

- `rust/`: `mm-sdk-rust`
- `go/`: `mm-sdk-go`
- `ts/`: `mm-sdk-ts`
- `csharp/`: `mm-sdk-csharp`

Rust / TypeScript / Go は重点開発対象です。各 SDK は次の補助を提供します。

- plugin metadata の構造化
- metadata validation
- lifecycle interface / trait
- 最小 Plugin example

## Install

配布後の利用方法:

```bash
cargo add mm-sdk-rust
npm install mm-sdk-ts
go get github.com/isksss/make-movie/plugin-api/sdk/go
```

monorepo 内で SDK 開発する場合は path / workspace 参照を使います。

## 生成

```bash
bash plugin-api/sdk/generate.sh
```

生成元は `plugin-api/plugin.wit` です。`plugin.wit` を変更した場合は必ず SDK を再生成します。

## 検証

```bash
bash plugin-api/sdk/verify.sh
```

この検証は `plugin.wit` の必須 export、各 SDK の最小ファイル、生成物の drift を確認します。Rust / TypeScript / Go SDK には追加で各言語の型検証・単体テストがあります。
EOF

cat >"$sdk_root/rust/Cargo.toml" <<'EOF'
[package]
name = "mm-sdk-rust"
version = "0.1.0"
edition = "2024"
license = "MIT"
description = "Rust SDK for make-movie WASM plugins"
repository = "https://github.com/isksss/make-movie"
homepage = "https://github.com/isksss/make-movie"
readme = "README.md"
keywords = ["make-movie", "plugin", "wasm", "wit"]
categories = ["api-bindings", "wasm"]
include = ["Cargo.toml", "README.md", "src/**/*.rs", "examples/**/*.rs"]

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]

[workspace]
EOF

cat >"$sdk_root/rust/README.md" <<'EOF'
# mm-sdk-rust

Rust SDK for make-movie plugins.

## Install

```bash
cargo add mm-sdk-rust
```

monorepo 内で開発する場合:

```toml
[dependencies]
mm-sdk-rust = { path = "../make-movie/plugin-api/sdk/rust" }
```

## Example

```rust
use mm_sdk_rust::{MmPlugin, PluginCategory, PluginMetadata, export_plugin};

#[derive(Default)]
struct MyPlugin;

impl MmPlugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("my-plugin", "0.1.0", PluginCategory::Utility)
    }
}

export_plugin!(MyPlugin);
```

## Publish

```bash
cargo publish --dry-run
cargo publish
```
EOF

cat >"$sdk_root/rust/src/lib.rs" <<'EOF'
//! make-movie Plugin SDK for Rust.
//!
//! Generated from `plugin-api/plugin.wit`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginCategory {
    Ai,
    Subtitle,
    Tts,
    Template,
    Export,
    Utility,
}

impl PluginCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ai => "ai",
            Self::Subtitle => "subtitle",
            Self::Tts => "tts",
            Self::Template => "template",
            Self::Export => "export",
            Self::Utility => "utility",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub category: PluginCategory,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

impl PluginMetadata {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        category: PluginCategory,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            category,
            display_name: None,
            description: None,
        }
    }

    pub fn display_name(mut self, value: impl Into<String>) -> Self {
        self.display_name = Some(value.into());
        self
    }

    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }

    pub fn validate(&self) -> Result<(), MetadataError> {
        validate_required("name", &self.name)?;
        validate_required("version", &self.version)?;
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, MetadataError> {
        self.validate()?;
        let mut fields = vec![
            json_field("name", &self.name),
            json_field("version", &self.version),
            json_field("category", self.category.as_str()),
        ];
        if let Some(value) = &self.display_name {
            fields.push(json_field("display_name", value));
        }
        if let Some(value) = &self.description {
            fields.push(json_field("description", value));
        }
        Ok(format!("{{{}}}", fields.join(",")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataError {
    message: String,
}

impl MetadataError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for MetadataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MetadataError {}

pub trait MmPlugin {
    fn metadata(&self) -> PluginMetadata;

    fn initialize(&mut self) {}

    fn shutdown(&mut self) {}
}

pub fn validate_metadata_json(json: &str) -> Result<(), MetadataError> {
    validate_required("metadata", json)?;
    for field in ["name", "version", "category"] {
        let pattern = format!("\"{field}\"");
        if !json.contains(&pattern) {
            return Err(MetadataError::new(format!("metadata missing `{field}`")));
        }
    }
    Ok(())
}

#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        static PLUGIN: std::sync::Mutex<Option<$plugin>> = std::sync::Mutex::new(None);

        #[unsafe(no_mangle)]
        pub extern "C" fn metadata() -> *mut std::ffi::c_char {
            let mut plugin = PLUGIN.lock().expect("plugin lock poisoned");
            let instance = plugin.get_or_insert_with(<$plugin as Default>::default);
            let metadata = <$plugin as $crate::MmPlugin>::metadata(instance)
                .to_json()
                .expect("plugin metadata must be valid");
            std::ffi::CString::new(metadata)
                .expect("plugin metadata must not contain NUL bytes")
                .into_raw()
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn metadata_free(ptr: *mut std::ffi::c_char) {
            if !ptr.is_null() {
                unsafe {
                    let _ = std::ffi::CString::from_raw(ptr);
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn initialize() {
            let mut plugin = PLUGIN.lock().expect("plugin lock poisoned");
            let instance = plugin.get_or_insert_with(<$plugin as Default>::default);
            <$plugin as $crate::MmPlugin>::initialize(instance);
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn shutdown() {
            let mut plugin = PLUGIN.lock().expect("plugin lock poisoned");
            if let Some(instance) = plugin.as_mut() {
                <$plugin as $crate::MmPlugin>::shutdown(instance);
            }
        }
    };
}

fn validate_required(field: &str, value: &str) -> Result<(), MetadataError> {
    if value.trim().is_empty() {
        Err(MetadataError::new(format!(
            "metadata `{field}` is required"
        )))
    } else {
        Ok(())
    }
}

fn json_field(key: &str, value: &str) -> String {
    format!("\"{}\":\"{}\"", escape_json(key), escape_json(value))
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            value if value.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", value as u32));
            }
            value => escaped.push(value),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::{MmPlugin, PluginCategory, PluginMetadata, validate_metadata_json};

    #[derive(Default)]
    struct TestPlugin;

    impl MmPlugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("test", "0.1.0", PluginCategory::Utility)
        }
    }

    #[test]
    fn plugin_returns_metadata() {
        let metadata = TestPlugin.metadata().to_json().unwrap();
        assert!(metadata.contains("\"name\":\"test\""));
        validate_metadata_json(&metadata).unwrap();
    }

    #[test]
    fn metadata_builder_escapes_json() {
        let metadata = PluginMetadata::new("quoted", "0.1.0", PluginCategory::Template)
            .display_name("quote \" plugin")
            .description("line\nbreak")
            .to_json()
            .unwrap();

        assert!(metadata.contains(r#""display_name":"quote \" plugin""#));
        assert!(metadata.contains(r#""description":"line\nbreak""#));
    }

    #[test]
    fn metadata_validation_rejects_empty_name() {
        let error = PluginMetadata::new("", "0.1.0", PluginCategory::Utility)
            .to_json()
            .unwrap_err();

        assert!(error.to_string().contains("name"));
    }
}
EOF

cat >"$sdk_root/rust/examples/minimal_plugin.rs" <<'EOF'
use mm_sdk_rust::{MmPlugin, PluginCategory, PluginMetadata, export_plugin};

#[derive(Default)]
struct MinimalPlugin;

impl MmPlugin for MinimalPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("minimal-plugin", "0.1.0", PluginCategory::Utility)
            .display_name("Minimal Plugin")
            .description("Minimal make-movie plugin example")
    }
}

export_plugin!(MinimalPlugin);

fn main() {}
EOF

cat >"$sdk_root/go/go.mod" <<'EOF'
module github.com/isksss/make-movie/plugin-api/sdk/go

go 1.22
EOF

cat >"$sdk_root/go/README.md" <<'EOF'
# mm-sdk-go

Go SDK for make-movie plugins.

## Install

```bash
go get github.com/isksss/make-movie/plugin-api/sdk/go
```

## Example

```go
package main

import mmsdk "github.com/isksss/make-movie/plugin-api/sdk/go"

type MyPlugin struct{}

func (plugin MyPlugin) Metadata() mmsdk.Metadata {
	return mmsdk.Metadata{
		Name:     "my-plugin",
		Version:  "0.1.0",
		Category: mmsdk.CategoryUtility,
	}
}

func (plugin MyPlugin) Initialize() error {
	return nil
}

func (plugin MyPlugin) Shutdown() error {
	return nil
}

var _ mmsdk.Plugin = MyPlugin{}
```

## Release Tag

Go は GitHub module path から取得します。tag は submodule path を prefix にします。

```bash
git tag plugin-api/sdk/go/v0.1.0
git push origin plugin-api/sdk/go/v0.1.0
```
EOF

cat >"$sdk_root/go/plugin.go" <<'EOF'
package mmsdk

// Generated from plugin-api/plugin.wit.

import (
	"encoding/json"
	"errors"
	"strings"
)

// Category is the plugin category used by make-movie.
type Category string

const (
	CategoryAI       Category = "ai"
	CategorySubtitle Category = "subtitle"
	CategoryTTS      Category = "tts"
	CategoryTemplate Category = "template"
	CategoryExport   Category = "export"
	CategoryUtility  Category = "utility"
)

// Metadata is the structured metadata returned by Plugin.Metadata.
type Metadata struct {
	Name        string   `json:"name"`
	Version     string   `json:"version"`
	Category    Category `json:"category"`
	DisplayName string   `json:"display_name,omitempty"`
	Description string   `json:"description,omitempty"`
}

func (metadata Metadata) Validate() error {
	if strings.TrimSpace(metadata.Name) == "" {
		return errors.New("metadata name is required")
	}
	if strings.TrimSpace(metadata.Version) == "" {
		return errors.New("metadata version is required")
	}
	switch metadata.Category {
	case CategoryAI, CategorySubtitle, CategoryTTS, CategoryTemplate, CategoryExport, CategoryUtility:
		return nil
	default:
		return errors.New("metadata category is invalid")
	}
}

func (metadata Metadata) JSON() (string, error) {
	if err := metadata.Validate(); err != nil {
		return "", err
	}
	value, err := json.Marshal(metadata)
	if err != nil {
		return "", err
	}
	return string(value), nil
}

func MustMetadataJSON(metadata Metadata) string {
	value, err := metadata.JSON()
	if err != nil {
		panic(err)
	}
	return value
}

// Plugin mirrors the lifecycle defined in plugin-api/plugin.wit.
type Plugin interface {
	Metadata() Metadata
	Initialize() error
	Shutdown() error
}

// NoopPlugin is useful for examples and tests.
type NoopPlugin struct {
	Value Metadata
}

func (plugin NoopPlugin) Metadata() Metadata {
	if strings.TrimSpace(plugin.Value.Name) == "" {
		return Metadata{
			Name:     "noop-plugin",
			Version:  "0.1.0",
			Category: CategoryUtility,
		}
	}
	return plugin.Value
}

func (plugin NoopPlugin) Initialize() error {
	return nil
}

func (plugin NoopPlugin) Shutdown() error {
	return nil
}
EOF

cat >"$sdk_root/go/plugin_test.go" <<'EOF'
package mmsdk

import (
	"strings"
	"testing"
)

func TestMetadataJSON(t *testing.T) {
	metadata := Metadata{
		Name:        "go-plugin",
		Version:     "0.1.0",
		Category:    CategoryUtility,
		DisplayName: "Go Plugin",
	}

	value, err := metadata.JSON()
	if err != nil {
		t.Fatalf("metadata JSON should be valid: %v", err)
	}

	if !strings.Contains(value, `"name":"go-plugin"`) {
		t.Fatalf("metadata JSON does not contain name: %s", value)
	}
}

func TestMetadataValidationRejectsInvalidCategory(t *testing.T) {
	metadata := Metadata{Name: "go-plugin", Version: "0.1.0", Category: "invalid"}

	if err := metadata.Validate(); err == nil {
		t.Fatal("invalid category should fail")
	}
}

func TestNoopPluginMetadataIsValid(t *testing.T) {
	metadata := NoopPlugin{}.Metadata()

	if err := metadata.Validate(); err != nil {
		t.Fatalf("metadata should be valid: %v", err)
	}

	if metadata.Category != CategoryUtility {
		t.Fatalf("noop metadata should contain category")
	}
}
EOF

cat >"$sdk_root/go/example_test.go" <<'EOF'
package mmsdk_test

import mmsdk "github.com/isksss/make-movie/plugin-api/sdk/go"

type MinimalPlugin struct{}

func (plugin MinimalPlugin) Metadata() mmsdk.Metadata {
	return mmsdk.Metadata{
		Name:     "minimal-plugin",
		Version:  "0.1.0",
		Category: mmsdk.CategoryUtility,
	}
}

func (plugin MinimalPlugin) Initialize() error {
	return nil
}

func (plugin MinimalPlugin) Shutdown() error {
	return nil
}

var _ mmsdk.Plugin = MinimalPlugin{}
EOF

cat >"$sdk_root/ts/package.json" <<'EOF'
{
  "name": "mm-sdk-ts",
  "version": "0.1.0",
  "description": "TypeScript SDK for make-movie plugins",
  "license": "MIT",
  "type": "module",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "import": "./dist/index.js"
    }
  },
  "files": [
    "dist",
    "README.md",
    "package.json"
  ],
  "repository": {
    "type": "git",
    "url": "git+https://github.com/isksss/make-movie.git",
    "directory": "plugin-api/sdk/ts"
  },
  "publishConfig": {
    "access": "public"
  },
  "scripts": {
    "build": "tsc -p tsconfig.json",
    "prepack": "npm run build",
    "prepublishOnly": "npm test && npm run build",
    "test": "tsc --noEmit -p tsconfig.test.json"
  },
  "devDependencies": {
    "typescript": "^6.0.3"
  }
}
EOF

cat >"$sdk_root/ts/README.md" <<'EOF'
# mm-sdk-ts

TypeScript SDK for make-movie plugins.

## Install

```bash
npm install mm-sdk-ts
```

pnpm:

```bash
pnpm add mm-sdk-ts
```

## Example

```ts
import { definePlugin } from "mm-sdk-ts";

export default definePlugin({
  metadata: () => ({
    name: "my-plugin",
    version: "0.1.0",
    category: "utility",
  }),
  initialize: () => undefined,
  shutdown: () => undefined,
});
```

## Publish

```bash
pnpm build
npm publish --dry-run
npm publish --access public
```
EOF

cat >"$sdk_root/ts/tsconfig.json" <<'EOF'
{
  "compilerOptions": {
    "declaration": true,
    "module": "ES2022",
    "moduleResolution": "Bundler",
    "outDir": "dist",
    "rootDir": "src",
    "strict": true,
    "target": "ES2022"
  },
  "include": ["src/index.ts"]
}
EOF

cat >"$sdk_root/ts/tsconfig.test.json" <<'EOF'
{
  "extends": "./tsconfig.json",
  "compilerOptions": {
    "noEmit": true,
    "rootDir": "."
  },
  "include": ["src/**/*.ts", "examples/**/*.ts"]
}
EOF

cat >"$sdk_root/ts/src/index.ts" <<'EOF'
// Generated from plugin-api/plugin.wit.

export type PluginCategory =
  | "ai"
  | "subtitle"
  | "tts"
  | "template"
  | "export"
  | "utility";

export interface PluginMetadata {
  name: string;
  version: string;
  category: PluginCategory;
  displayName?: string;
  description?: string;
}

export interface MmPlugin {
  metadata(): PluginMetadata;
  initialize(): void | Promise<void>;
  shutdown(): void | Promise<void>;
}

const categories = new Set<PluginCategory>([
  "ai",
  "subtitle",
  "tts",
  "template",
  "export",
  "utility",
]);

export function definePlugin(plugin: MmPlugin): MmPlugin {
  return plugin;
}

export function metadataToJson(metadata: PluginMetadata): string {
  validateMetadata(metadata);
  return JSON.stringify({
    name: metadata.name,
    version: metadata.version,
    category: metadata.category,
    ...(metadata.displayName ? { display_name: metadata.displayName } : {}),
    ...(metadata.description ? { description: metadata.description } : {}),
  });
}

export function validateMetadata(metadata: PluginMetadata): void {
  if (metadata.name.trim() === "") {
    throw new Error("metadata name is required");
  }
  if (metadata.version.trim() === "") {
    throw new Error("metadata version is required");
  }
  if (!categories.has(metadata.category)) {
    throw new Error(`metadata category is invalid: ${metadata.category}`);
  }
}

export const noopPlugin: MmPlugin = definePlugin({
  metadata: () => ({
    name: "noop-plugin",
    version: "0.1.0",
    category: "utility",
  }),
  initialize: () => undefined,
  shutdown: () => undefined,
});
EOF

cat >"$sdk_root/ts/src/index.test.ts" <<'EOF'
import {
  definePlugin,
  metadataToJson,
  noopPlugin,
  type MmPlugin,
  type PluginMetadata,
} from "./index.js";

const metadata = {
  name: "ts-plugin",
  version: "0.1.0",
  category: "utility",
  displayName: "TS Plugin",
} satisfies PluginMetadata;

const metadataJson: string = metadataToJson(metadata);

const plugin: MmPlugin = definePlugin({
  metadata: () => metadata,
  initialize: async () => undefined,
  shutdown: () => undefined,
});

const noopMetadata: PluginMetadata = noopPlugin.metadata();

void plugin;
void noopMetadata;
EOF

cat >"$sdk_root/ts/examples/minimal-plugin.ts" <<'EOF'
import { definePlugin } from "../src/index.js";

export default definePlugin({
  metadata: () => ({
    name: "minimal-plugin",
    version: "0.1.0",
    category: "utility",
    displayName: "Minimal Plugin",
    description: "Minimal make-movie plugin example",
  }),
  initialize: () => undefined,
  shutdown: () => undefined,
});
EOF

cat >"$sdk_root/csharp/mm-sdk-csharp.csproj" <<'EOF'
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <AssemblyName>Mm.Sdk</AssemblyName>
    <PackageId>mm-sdk-csharp</PackageId>
    <Version>0.1.0</Version>
  </PropertyGroup>
</Project>
EOF

cat >"$sdk_root/csharp/Plugin.cs" <<'EOF'
namespace Mm.Sdk;

/// <summary>
/// Minimal lifecycle contract generated from plugin-api/plugin.wit.
/// </summary>
public interface IMmPlugin
{
    PluginMetadata Metadata();

    void Initialize()
    {
    }

    void Shutdown()
    {
    }
}

public enum PluginCategory
{
    Ai,
    Subtitle,
    Tts,
    Template,
    Export,
    Utility,
}

public sealed record PluginMetadata(
    string Name,
    string Version,
    PluginCategory Category,
    string? DisplayName = null,
    string? Description = null);

public sealed class NoopPlugin : IMmPlugin
{
    public PluginMetadata Metadata() => new("noop-plugin", "0.1.0", PluginCategory.Utility);
}
EOF

echo "plugin SDK scaffolds generated from plugin.wit"
