# 検証

## Rust

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## CLI

```bash
cargo run -p mm-cli -- validate --project examples/basic/mm.toml
cargo run -p mm-cli -- doctor
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
corepack pnpm --dir apps/mm-gui build
```

## Tauri

```bash
cargo test --manifest-path apps/mm-gui/src-tauri/Cargo.toml
cargo check --manifest-path apps/mm-gui/src-tauri/Cargo.toml
```
