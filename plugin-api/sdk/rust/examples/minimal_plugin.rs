use mm_sdk_rust::{MmPlugin, PluginCategory, PluginMetadata, export_plugin};

#[derive(Default)]
struct MinimalPlugin;

impl MmPlugin for MinimalPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("minimal-plugin", "0.1.0", PluginCategory::Utility)
            .display_name("Minimal Plugin")
            .description("Minimal make-movie plugin example")
    }
}

export_plugin!(MinimalPlugin);

fn main() {}
