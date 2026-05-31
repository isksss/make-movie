# Rust Plugin Template

Rust で make-movie Plugin を作る最小テンプレートです。

## Install

外部 repository で使う場合は SDK を crates.io から追加します。

```bash
cargo add mm-sdk-rust
```

この repository 内の検証では `../../sdk/rust` を path dependency として参照します。

## Build / Test

```bash
cargo test
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

成果物は `target/wasm32-unknown-unknown/release/mm_plugin_rust_basic.wasm` です。配布時は `dist/` にコピーして `plugin.toml` の `component` から参照します。
