# Plugin SDK Release

Rust / TypeScript / Go SDKを配布するためのリリース手順です。

## 方針

- `plugin-api/plugin.wit` を唯一のABI契約とする。
- SDK更新は `plugin-api/sdk/generate.sh` から生成し、手動編集だけで済ませない。
- 認証情報、token、passwordはdocs、Issue、PR、ログに書かない。
- publish前に必ずdry-runを実行する。
- Go SDKは `plugin-api/sdk/go` がmodule rootなので、tagにsubdirectory prefixを付ける。

## 共通チェック

```bash
bash plugin-api/sdk/generate.sh
bash plugin-api/sdk/verify.sh
bash plugin-api/sdk/verify-release.sh
git diff --check
```

ABIを変更した場合は、次も同じPRで更新します。

- `plugin-api/plugin.wit`
- `plugin-api/docs/abi.md`
- `plugin-api/sdk/generate.sh`
- `docs/plugin-development-rust-ts-go.md`
- Runtime側のload / lifecycle呼び出し

## Rust SDK

対象:

```text
plugin-api/sdk/rust
```

公開前検証:

```bash
cargo fmt --manifest-path plugin-api/sdk/rust/Cargo.toml --check
cargo test --manifest-path plugin-api/sdk/rust/Cargo.toml --all-targets
cargo package --manifest-path plugin-api/sdk/rust/Cargo.toml --allow-dirty --no-verify --list
cargo package --manifest-path plugin-api/sdk/rust/Cargo.toml --allow-dirty --no-verify
cargo publish --manifest-path plugin-api/sdk/rust/Cargo.toml --dry-run --allow-dirty
```

公開:

```bash
cargo publish --manifest-path plugin-api/sdk/rust/Cargo.toml
```

公開後確認:

```bash
cargo search mm-sdk-rust
```

## TypeScript SDK

対象:

```text
plugin-api/sdk/ts
```

公開前検証:

```bash
corepack pnpm --dir plugin-api/sdk/ts install --frozen-lockfile
corepack pnpm --dir plugin-api/sdk/ts test
corepack pnpm --dir plugin-api/sdk/ts build
(cd plugin-api/sdk/ts && npm pack --dry-run)
```

公開:

```bash
cd plugin-api/sdk/ts
npm publish --access public
```

公開後確認:

```bash
npm view mm-sdk-ts version
```

## Go SDK

対象:

```text
plugin-api/sdk/go
```

公開前検証:

```bash
cd plugin-api/sdk/go
go test ./...
go vet ./...
```

tag:

```bash
git tag plugin-api/sdk/go/v0.1.0
git push origin plugin-api/sdk/go/v0.1.0
```

公開後確認:

```bash
go list -m github.com/isksss/make-movie/plugin-api/sdk/go@v0.1.0
```

## Release PR

PR本文には次を記載します。

- SDK名とversion
- `plugin.wit` 変更の有無
- 破壊的変更の有無
- dry-run結果
- publish後確認結果

## Rollback

crates.ioとnpmは公開済みversionを上書きしません。問題が見つかった場合は修正版versionを追加で公開します。Go SDKは誤tagを削除せず、修正版tagを追加します。
