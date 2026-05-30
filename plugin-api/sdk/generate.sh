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

mkdir -p \
  "$sdk_root/rust/src" \
  "$sdk_root/go" \
  "$sdk_root/ts/src" \
  "$sdk_root/csharp"

cat >"$sdk_root/README.md" <<'EOF'
# make-movie Plugin SDK

このディレクトリは `plugin-api/plugin.wit` を正本として生成する最小 SDK 雛形を管理します。

## 契約

唯一の ABI 契約は `../plugin.wit` です。SDK は Plugin 実装者が各言語で同じ export を実装しやすくするための薄い補助層です。

必須 export:

- `metadata() -> string`
- `initialize()`
- `shutdown()`

## SDK

- `rust/`: `mm-sdk-rust`
- `go/`: `mm-sdk-go`
- `ts/`: `mm-sdk-ts`
- `csharp/`: `mm-sdk-csharp`

## 生成

```bash
bash plugin-api/sdk/generate.sh
```

生成元は `plugin-api/plugin.wit` です。`plugin.wit` を変更した場合は必ず SDK を再生成します。

## 検証

```bash
bash plugin-api/sdk/verify.sh
```

この検証は `plugin.wit` の必須 export、各 SDK の最小ファイル、生成物の drift を確認します。
EOF

cat >"$sdk_root/rust/Cargo.toml" <<'EOF'
[package]
name = "mm-sdk-rust"
version = "0.1.0"
edition = "2021"
license = "MIT"
repository = "https://github.com/isksss/make-movie"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]

[workspace]
EOF

cat >"$sdk_root/rust/src/lib.rs" <<'EOF'
//! make-movie Plugin SDK for Rust.
//!
//! Generated from `plugin-api/plugin.wit`.

pub trait MmPlugin {
    fn metadata(&self) -> String;

    fn initialize(&mut self) {}

    fn shutdown(&mut self) {}
}

#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        static PLUGIN: std::sync::Mutex<Option<$plugin>> = std::sync::Mutex::new(None);

        #[no_mangle]
        pub extern "C" fn initialize() {
            let mut plugin = PLUGIN.lock().expect("plugin lock poisoned");
            let instance = plugin.get_or_insert_with(<$plugin as Default>::default);
            <$plugin as $crate::MmPlugin>::initialize(instance);
        }

        #[no_mangle]
        pub extern "C" fn shutdown() {
            let mut plugin = PLUGIN.lock().expect("plugin lock poisoned");
            if let Some(instance) = plugin.as_mut() {
                <$plugin as $crate::MmPlugin>::shutdown(instance);
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::MmPlugin;

    #[derive(Default)]
    struct TestPlugin;

    impl MmPlugin for TestPlugin {
        fn metadata(&self) -> String {
            r#"{"name":"test"}"#.to_string()
        }
    }

    #[test]
    fn plugin_returns_metadata() {
        assert!(TestPlugin.metadata().contains("test"));
    }
}
EOF

cat >"$sdk_root/go/go.mod" <<'EOF'
module github.com/isksss/make-movie/plugin-api/sdk/go

go 1.22
EOF

cat >"$sdk_root/go/plugin.go" <<'EOF'
package mmsdk

// Generated from plugin-api/plugin.wit.

// Plugin mirrors the lifecycle defined in plugin-api/plugin.wit.
type Plugin interface {
	Metadata() string
	Initialize() error
	Shutdown() error
}

// NoopPlugin is useful for examples and tests.
type NoopPlugin struct {
	Value string
}

func (plugin NoopPlugin) Metadata() string {
	if plugin.Value == "" {
		return "{}"
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

cat >"$sdk_root/ts/package.json" <<'EOF'
{
  "name": "mm-sdk-ts",
  "version": "0.1.0",
  "license": "MIT",
  "type": "module",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "build": "tsc --declaration --emitDeclarationOnly",
    "test": "tsc --noEmit"
  },
  "devDependencies": {
    "typescript": "^5.9.3"
  }
}
EOF

cat >"$sdk_root/ts/tsconfig.json" <<'EOF'
{
  "compilerOptions": {
    "declaration": true,
    "emitDeclarationOnly": true,
    "module": "ES2022",
    "moduleResolution": "Bundler",
    "outDir": "dist",
    "strict": true,
    "target": "ES2022"
  },
  "include": ["src/**/*.ts"]
}
EOF

cat >"$sdk_root/ts/src/index.ts" <<'EOF'
// Generated from plugin-api/plugin.wit.

export interface MmPlugin {
  metadata(): string;
  initialize(): void | Promise<void>;
  shutdown(): void | Promise<void>;
}

export function definePlugin(plugin: MmPlugin): MmPlugin {
  return plugin;
}

export const noopPlugin: MmPlugin = definePlugin({
  metadata: () => "{}",
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
    string Metadata();

    void Initialize()
    {
    }

    void Shutdown()
    {
    }
}

public sealed class NoopPlugin : IMmPlugin
{
    public string Metadata() => "{}";
}
EOF

echo "plugin SDK scaffolds generated from plugin.wit"
