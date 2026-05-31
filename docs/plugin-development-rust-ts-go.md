# Rust / TypeScript / Go Plugin 開発手順

## 目的

このドキュメントは、Rust、TypeScript、Go で make-movie Plugin を開発する手順を定義します。

Plugin は追加機能専用です。Core の必須編集機能は Plugin で置き換えません。

SDK 開発は Rust、TypeScript、Go を重点対象にします。Rust SDK は native / WASM 実装の主 SDK、TypeScript SDK は script / AI / template 系 Plugin の主 SDK、Go SDK は lightweight utility / backend integration 系 Plugin の主 SDK として優先的に拡張します。

C#、Zig、C、C++ などの SDK は、当面 `plugin.wit` 追従と最小 lifecycle の互換性維持を優先します。機能追加は Rust / TypeScript / Go SDK で固めてから横展開します。

## 前提

唯一の ABI 契約は `plugin-api/plugin.wit` です。

```wit
package mm:plugin;

interface plugin {
  metadata: func() -> string;
  initialize: func();
  shutdown: func();
}
```

Plugin Runtime は WASM Component Model を優先して読み込みます。互換 fallback として core wasm も読み込みますが、Component Model Plugin を標準形とします。

## 共通ディレクトリ構成

Plugin repository は次の構成を推奨します。

```text
my-plugin/
├── plugin.toml
├── src/
├── wit/
│   └── plugin.wit
├── dist/
│   └── my-plugin.wasm
└── README.md
```

`wit/plugin.wit` は make-movie 本体の `plugin-api/plugin.wit` と同じ内容にします。ABI を変更する場合は、本体側の `plugin-api/plugin.wit` を先に変更し、SDK と Runtime を同じ PR で更新します。

## Templates

SDK利用者向けの最小テンプレートは `plugin-api/templates/` に配置します。

- `plugin-api/templates/rust-basic`
- `plugin-api/templates/ts-basic`
- `plugin-api/templates/go-basic`

テンプレートは repository 内検証のため local SDK 参照を使います。外部 repository にコピーする場合は、各テンプレートの README に従って crates.io / npm / GitHub module 参照へ切り替えます。

## Manifest

local 開発では `plugin.toml` を使います。

```toml
name = "my-plugin"
version = "0.1.0"
component = "dist/my-plugin.wasm"

[metadata]
display_name = "My Plugin"
category = "utility"
description = "開発中の local plugin"

[source]
repository = "local"
path = "./dist/my-plugin.wasm"
```

Project から利用する場合は `mm.toml` に追加します。

```toml
[[plugin]]
repository = "local"
name = "my-plugin"
path = "./plugins/my-plugin/dist/my-plugin.wasm"
version = "0.1.0"
component = "my-plugin.wasm"
```

`mm plugin install --project mm.toml` は Global Config と Project Config を merge し、Project Config を優先します。install 後は `mm.lock` に name、version、checksum、install path が保存されます。

## SDK 更新

`plugin-api/plugin.wit` を変更した場合は SDK を再生成します。

```bash
bash plugin-api/sdk/generate.sh
bash plugin-api/sdk/verify.sh
```

生成物の drift を防ぐため、SDK 更新 PR では `plugin-api/sdk/` の差分を必ず確認します。

SDK配布時の詳細なリリース手順は `docs/plugin-sdk-release.md` を参照します。

## Rust Plugin

### 1. プロジェクト作成

```bash
cargo new --lib my-plugin
cd my-plugin
```

`Cargo.toml` は Rust 2024 edition を使います。

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2024"
license = "MIT"

[lib]
crate-type = ["cdylib"]

[dependencies]
mm-sdk-rust = "0.1"
```

monorepo 内で SDK 本体を同時開発する場合だけ path 参照を使います。

```toml
[dependencies]
mm-sdk-rust = { path = "../make-movie/plugin-api/sdk/rust" }
```

### 2. lifecycle 実装

`mm-sdk-rust` は crates.io から取得します。

```bash
cargo add mm-sdk-rust
```

`MmPlugin` を実装し、Plugin 固有の状態は struct に保持します。metadata は `PluginMetadata` で生成し、`export_plugin!` で `metadata` / `initialize` / `shutdown` を export します。

```rust
use mm_sdk_rust::{MmPlugin, PluginCategory, PluginMetadata, export_plugin};

#[derive(Default)]
struct MyPlugin {
    initialized: bool,
}

impl MmPlugin for MyPlugin {
    fn metadata(&self) -> String {
        PluginMetadata::new("my-plugin", "0.1.0", PluginCategory::Utility)
            .display_name("My Plugin")
            .description("Rust plugin")
            .to_json()
            .expect("metadata must be valid")
    }

    fn initialize(&mut self) {
        self.initialized = true;
    }

    fn shutdown(&mut self) {
        self.initialized = false;
    }
}

export_plugin!(MyPlugin);
```

Component Model Plugin として本番利用する場合は、`plugin-api/plugin.wit` の `metadata` / `initialize` / `shutdown` を export する component wasm を生成します。core wasm fallback では Runtime が `initialize` / `shutdown` を optional export として扱い、`metadata` は Component Model Plugin で取得されます。

### 3. build

最小検証では core wasm を build します。

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
mkdir -p dist
cp target/wasm32-unknown-unknown/release/my_plugin.wasm dist/my-plugin.wasm
```

Component Model で配布する場合は、WASI Preview2 / Component Model 用 target と adapter を使って `plugin-api/plugin.wit` に一致する component wasm を生成します。生成手順を更新した場合は、この docs と CI の検証手順も同時に更新します。

### 4. local install

```bash
mm plugin install --manifest plugin.toml
```

Project config から install する場合:

```bash
mm plugin install --project mm.toml
```

### 5. 検証

Rust Plugin 側で最低限実行します。

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --target wasm32-unknown-unknown
```

SDK を crates.io へ公開する前は SDK directory で dry-run します。

```bash
cd plugin-api/sdk/rust
cargo publish --dry-run
```

make-movie 本体へ SDK や Runtime の変更を入れる場合は、repository root で実行します。

```bash
bash scripts/verify-all.sh
```

## TypeScript Plugin

### 1. プロジェクト作成

```bash
mkdir my-plugin
cd my-plugin
corepack pnpm init
corepack pnpm add -D typescript
corepack pnpm add mm-sdk-ts
```

`package.json` は ESM と strict TypeScript を前提にします。

```json
{
  "name": "my-plugin",
  "version": "0.1.0",
  "license": "MIT",
  "type": "module",
  "scripts": {
    "build": "tsc",
    "test": "tsc --noEmit"
  },
  "dependencies": {
    "mm-sdk-ts": "^0.1.0"
  },
  "devDependencies": {
    "typescript": "^6.0.3"
  }
}
```

`tsconfig.json`:

```json
{
  "compilerOptions": {
    "module": "ES2022",
    "moduleResolution": "Bundler",
    "strict": true,
    "target": "ES2022"
  },
  "include": ["src/**/*.ts"]
}
```

### 2. lifecycle 実装

`mm-sdk-ts` は npm から取得します。

```bash
npm install mm-sdk-ts
```

pnpm を使う場合:

```bash
corepack pnpm add mm-sdk-ts
```

`definePlugin` で lifecycle を型付けし、`metadataToJson` で make-movie が読む metadata JSON を生成します。

```ts
import { definePlugin, metadataToJson } from "mm-sdk-ts";

export default definePlugin({
  metadata: () =>
    metadataToJson({
      name: "my-plugin",
      version: "0.1.0",
      category: "utility",
      displayName: "My Plugin",
      description: "TypeScript plugin",
    }),
  initialize: () => {
    // Plugin 初期化
  },
  shutdown: () => {
    // Plugin 終了処理
  },
});
```

TypeScript 実装を Runtime で直接読み込むには、最終成果物を `plugin-api/plugin.wit` に一致する component wasm に変換する必要があります。TypeScript SDK は現在、型と実装パターンの正本です。WASM component への変換 toolchain を追加する場合は、SDK、docs、CI を同じチケットで更新します。

### 3. build / test

```bash
corepack pnpm install
corepack pnpm test
corepack pnpm build
```

component wasm を生成する toolchain を追加した場合は、出力を `dist/my-plugin.wasm` に揃えます。

```text
dist/my-plugin.wasm
```

### 4. local install

生成済み component wasm を `plugin.toml` から参照します。

```toml
name = "my-plugin"
version = "0.1.0"
component = "dist/my-plugin.wasm"

[metadata]
display_name = "My TS Plugin"
category = "utility"
description = "TypeScript plugin"

[source]
repository = "local"
path = "./dist/my-plugin.wasm"
```

install:

```bash
mm plugin install --manifest plugin.toml
```

Project config から install:

```bash
mm plugin install --project mm.toml
```

### 5. 検証

TypeScript Plugin 側で最低限実行します。

```bash
corepack pnpm test
corepack pnpm build
```

make-movie 本体の TypeScript SDK を変更した場合は、repository root で実行します。

```bash
bash plugin-api/sdk/generate.sh
bash plugin-api/sdk/verify.sh
bash scripts/verify-all.sh
```

SDK を npm へ公開する前は SDK directory で package 内容を確認します。

```bash
cd plugin-api/sdk/ts
pnpm build
npm pack --dry-run
npm publish --dry-run
```

## Go Plugin

### 1. プロジェクト作成

```bash
mkdir my-plugin
cd my-plugin
go mod init example.com/my-plugin
```

`go.mod` は Go 1.22 以上を基準にします。

```go
module example.com/my-plugin

go 1.22

require github.com/isksss/make-movie/plugin-api/sdk/go v0.1.0
```

SDK は GitHub module path から取得します。

```bash
go get github.com/isksss/make-movie/plugin-api/sdk/go
```

monorepo 内で SDK 本体を同時開発する場合だけ `replace` を使います。

```go
replace github.com/isksss/make-movie/plugin-api/sdk/go => ../make-movie/plugin-api/sdk/go
```

Go SDK を GitHub module として release する場合、tag は submodule path を prefix にします。

```bash
git tag plugin-api/sdk/go/v0.1.0
git push origin plugin-api/sdk/go/v0.1.0
```

### 2. lifecycle 実装

`mm-sdk-go` は `plugin.wit` の lifecycle を Go interface として表現し、metadata JSON の生成と検証を提供します。

```go
package main

import mmsdk "github.com/isksss/make-movie/plugin-api/sdk/go"

type MyPlugin struct {
	initialized bool
}

var _ mmsdk.Plugin = (*MyPlugin)(nil)

func (plugin *MyPlugin) Metadata() string {
	return mmsdk.MustMetadataJSON(mmsdk.Metadata{
		Name:        "my-plugin",
		Version:     "0.1.0",
		Category:    mmsdk.CategoryUtility,
		DisplayName: "My Plugin",
		Description: "Go plugin",
	})
}

func (plugin *MyPlugin) Initialize() error {
	plugin.initialized = true
	return nil
}

func (plugin *MyPlugin) Shutdown() error {
	plugin.initialized = false
	return nil
}
```

Go 実装を Runtime で読み込むには、最終成果物を `plugin-api/plugin.wit` に一致する component wasm に変換します。TinyGo / WASI Preview2 / Component Model の toolchain を採用する場合は、SDK、docs、CI を同じチケットで更新します。

### 3. build / test

Go Plugin 側で最低限実行します。

```bash
go test ./...
go vet ./...
```

component wasm を生成する toolchain を追加した場合は、出力を `dist/my-plugin.wasm` に揃えます。

```text
dist/my-plugin.wasm
```

### 4. local install

生成済み component wasm を `plugin.toml` から参照します。

```toml
name = "my-plugin"
version = "0.1.0"
component = "dist/my-plugin.wasm"

[metadata]
display_name = "My Go Plugin"
category = "utility"
description = "Go plugin"

[source]
repository = "local"
path = "./dist/my-plugin.wasm"
```

install:

```bash
mm plugin install --manifest plugin.toml
```

Project config から install:

```bash
mm plugin install --project mm.toml
```

### 5. 検証

make-movie 本体の Go SDK を変更した場合は、repository root で実行します。

```bash
bash plugin-api/sdk/generate.sh
bash plugin-api/sdk/verify.sh
bash scripts/verify-all.sh
```

## 配布前チェック

Plugin を配布する前に次を確認します。

- `plugin.toml` の `name` / `version` / `component` が成果物と一致している。
- `metadata.category` が `ai` / `subtitle` / `tts` / `template` / `export` / `utility` のいずれかである。
- component wasm が `plugin-api/plugin.wit` と互換である。
- `initialize` が複数回呼ばれても壊れない。
- `shutdown` が未初期化状態でも安全に戻れる。
- Core 機能を置き換える挙動を持たない。
- local install 後に `mm.lock` の checksum が更新される。

## ABI 変更時の手順

1. `plugin-api/plugin.wit` を変更する。
2. `mm-plugin-runtime` の load / lifecycle 呼び出しを更新する。
3. `plugin-api/sdk/generate.sh` を更新する。
4. `bash plugin-api/sdk/generate.sh` を実行する。
5. Rust / TypeScript / Go SDK の利用例を更新する。
6. `plugin-api/docs/abi.md` とこのドキュメントを更新する。
7. `bash scripts/verify-all.sh` を実行する。

ABI 変更は Plugin 作者に影響するため、PR 本文に破壊的変更かどうかを明記します。
