//! make-movie Plugin SDK for Rust.
//!
//! Generated from `plugin-api/plugin.wit`.

pub trait MmPlugin {
    fn metadata(&self) -> String;

    fn initialize(&mut self) {}

    fn shutdown(&mut self) {}
}

#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        static PLUGIN: std::sync::Mutex<Option<$plugin>> = std::sync::Mutex::new(None);

        #[no_mangle]
        pub extern "C" fn initialize() {
            let mut plugin = PLUGIN.lock().expect("plugin lock poisoned");
            let instance = plugin.get_or_insert_with(<$plugin as Default>::default);
            <$plugin as $crate::MmPlugin>::initialize(instance);
        }

        #[no_mangle]
        pub extern "C" fn shutdown() {
            let mut plugin = PLUGIN.lock().expect("plugin lock poisoned");
            if let Some(instance) = plugin.as_mut() {
                <$plugin as $crate::MmPlugin>::shutdown(instance);
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::MmPlugin;

    #[derive(Default)]
    struct TestPlugin;

    impl MmPlugin for TestPlugin {
        fn metadata(&self) -> String {
            r#"{"name":"test"}"#.to_string()
        }
    }

    #[test]
    fn plugin_returns_metadata() {
        assert!(TestPlugin.metadata().contains("test"));
    }
}
