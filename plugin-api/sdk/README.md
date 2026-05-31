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
