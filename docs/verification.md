# 検証

## 一括検証

Repository root から主要検証をまとめて実行します。

```bash
bash scripts/verify-all.sh
```

この script は Rust workspace、GUI、Tauri、Plugin SDK、`git diff --check` を順に確認します。
`dotnet` が利用できる環境では C# SDK build も実行します。`dotnet` が無い環境では C# SDK build のみ明示的に skip します。

CI と近い Ubuntu 環境で検証する場合は Docker Compose を使います。

```bash
docker compose run --rm dev bash scripts/verify-all.sh
```

詳細は [Docker 開発・テスト環境](docker-development.md) を参照してください。

## Rust

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## CLI

```bash
cargo run -p mm-cli -- validate --project examples/basic/mm.toml
cargo run -p mm-cli -- validate --project examples/shorts-rich/mm.toml
cargo run -p mm-cli -- doctor
cargo run -p mm-cli -- package --project-root examples/basic --output examples/basic/output/basic.tar.gz
```

FFmpeg が利用可能な環境では build も確認します。

```bash
cargo run -p mm-cli -- build --project examples/basic/mm.toml --output examples/basic/output.mp4
```

## GUI

```bash
corepack pnpm --dir apps/mm-gui lint
corepack pnpm --dir apps/mm-gui test
corepack pnpm --dir apps/mm-gui e2e
corepack pnpm --dir apps/mm-gui e2e:tauri
corepack pnpm --dir apps/mm-gui build
```

`e2e` はブラウザ上の Playwright E2E、`e2e:tauri` は
`e2e-testing` feature で Tauri 実アプリを起動して Playwright から検証します。
`e2e:tauri` は WebKitGTK と GUI session が利用できる Linux 環境で実行します。
CI では Ubuntu 上で `xvfb-run -a pnpm --dir apps/mm-gui e2e:tauri` を実行し、
headless 環境でも Tauri 実アプリの Playwright E2E を検証します。

## Tauri

```bash
corepack pnpm --dir apps/mm-gui exec tauri --version
TAURI_TARGET_TRIPLE=x86_64-unknown-linux-gnu bash scripts/prepare-tauri-sidecars.sh
cargo test --manifest-path apps/mm-gui/src-tauri/Cargo.toml
cargo test --manifest-path apps/mm-gui/src-tauri/Cargo.toml --features e2e-testing
cargo clippy --manifest-path apps/mm-gui/src-tauri/Cargo.toml --all-targets -- -D warnings
```

GUI bundle を作成する場合は、事前に `apps/mm-gui/src-tauri/binaries/` へ
Tauri target triple 付きの `ffmpeg` / `ffprobe` sidecar を配置します。
通常は次の script で現在の Rust host triple 向けに準備できます。

```bash
bash scripts/prepare-tauri-sidecars.sh
```

例:

```bash
mkdir -p apps/mm-gui/src-tauri/binaries
cp "$(command -v ffmpeg)" apps/mm-gui/src-tauri/binaries/ffmpeg-x86_64-unknown-linux-gnu
cp "$(command -v ffprobe)" apps/mm-gui/src-tauri/binaries/ffprobe-x86_64-unknown-linux-gnu
```

Windows target では `.exe` 付きのファイル名を使います。

## Plugin SDK

```bash
bash plugin-api/sdk/generate.sh
bash plugin-api/sdk/verify.sh
cargo test --manifest-path plugin-api/sdk/rust/Cargo.toml
(cd plugin-api/sdk/go && go test ./...)
corepack pnpm --dir plugin-api/sdk/ts test
dotnet build plugin-api/sdk/csharp/mm-sdk-csharp.csproj
```
