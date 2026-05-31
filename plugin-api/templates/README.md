# Plugin Templates

このディレクトリは make-movie Plugin 開発を始めるための最小テンプレートを管理します。

## Templates

- `rust-basic`: Rust / `mm-sdk-rust`
- `ts-basic`: TypeScript / `mm-sdk-ts`
- `go-basic`: Go / `github.com/isksss/make-movie/plugin-api/sdk/go`

各テンプレートは repository 内検証のため local SDK 参照を使います。配布後に外部 repository へコピーする場合は、各テンプレートの README に従って crates.io / npm / GitHub module 参照へ切り替えます。

## Verification

```bash
cargo test --manifest-path plugin-api/templates/rust-basic/Cargo.toml
corepack pnpm --dir plugin-api/templates/ts-basic install --no-frozen-lockfile
corepack pnpm --dir plugin-api/templates/ts-basic test
(cd plugin-api/templates/go-basic && go test ./...)
```
