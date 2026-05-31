# Plugin System

## 方針

Plugin は追加機能専用です。Core 機能は置き換えません。

## ABI

`plugin-api/plugin.wit` を唯一の契約とします。

詳細は `plugin-api/docs/abi.md` を参照します。

## SDK

`plugin-api/sdk/` に `plugin.wit` を正本として利用する最小 SDK 雛形を配置します。

- `plugin-api/sdk/rust`: `mm-sdk-rust`
- `plugin-api/sdk/go`: `mm-sdk-go`
- `plugin-api/sdk/ts`: `mm-sdk-ts`
- `plugin-api/sdk/csharp`: `mm-sdk-csharp`

SDK 開発は Rust、TypeScript、Go を重点対象として進めます。Rust は native / WASM 実装の主 SDK、TypeScript は script / AI / template 系 Plugin の主 SDK、Go は lightweight utility / backend integration 系 Plugin の主 SDK として機能を厚くします。C#、その他言語 SDK は `plugin.wit` 追従と最小 lifecycle の互換性維持を優先します。

配布後の取得方法:

```bash
cargo add mm-sdk-rust
npm install mm-sdk-ts
go get github.com/isksss/make-movie/plugin-api/sdk/go
```

SDK 構成は次のコマンドで検証します。

```bash
bash plugin-api/sdk/verify.sh
```

Rust / TypeScript / Go で Plugin を開発する詳細手順は `docs/plugin-development-rust-ts-go.md` を参照します。

Plugin作成の最小テンプレートは `plugin-api/templates/` を参照します。

## Manifest / Lock

Plugin manifest と lock の schema は `plugin-api/schema/` に配置します。

- `plugin-api/schema/plugin-manifest.schema.json`
- `plugin-api/schema/plugin-lock.schema.json`

manifest の記述例は `plugin-api/docs/manifest.md` を参照します。

## 対応 repository

- GitHub
- GitLab
- URL
- Local

## 解決順

```text
Global Config
↓
Project Config
↓
Merge
↓
Resolve
↓
Download
↓
Load
```

Project config を優先します。
