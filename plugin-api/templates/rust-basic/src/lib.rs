use mm_sdk_rust::{MmPlugin, PluginCategory, PluginMetadata, export_plugin};

#[derive(Default)]
struct RustBasicPlugin {
    initialized: bool,
}

impl MmPlugin for RustBasicPlugin {
    fn metadata(&self) -> String {
        PluginMetadata::new("rust-basic", "0.1.0", PluginCategory::Utility)
            .display_name("Rust Basic")
            .description("Rust plugin template")
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

export_plugin!(RustBasicPlugin);

#[cfg(test)]
mod tests {
    use super::{MmPlugin, RustBasicPlugin};
    use mm_sdk_rust::validate_metadata_json;

    #[test]
    fn metadata_is_valid() {
        let plugin = RustBasicPlugin::default();

        validate_metadata_json(&plugin.metadata()).unwrap();
    }

    #[test]
    fn lifecycle_updates_state() {
        let mut plugin = RustBasicPlugin::default();

        plugin.initialize();
        assert!(plugin.initialized);

        plugin.shutdown();
        assert!(!plugin.initialized);
    }
}
