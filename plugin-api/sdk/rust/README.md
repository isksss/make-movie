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
    fn metadata(&self) -> String {
        PluginMetadata::new("my-plugin", "0.1.0", PluginCategory::Utility)
            .to_json()
            .expect("metadata must be valid")
    }
}

export_plugin!(MyPlugin);
```

## Publish

```bash
cargo publish --dry-run
cargo publish
```
