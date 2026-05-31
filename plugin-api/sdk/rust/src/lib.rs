//! make-movie Plugin SDK for Rust.
//!
//! Generated from `plugin-api/plugin.wit`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginCategory {
    Ai,
    Subtitle,
    Tts,
    Template,
    Export,
    Utility,
}

impl PluginCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ai => "ai",
            Self::Subtitle => "subtitle",
            Self::Tts => "tts",
            Self::Template => "template",
            Self::Export => "export",
            Self::Utility => "utility",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub category: PluginCategory,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

impl PluginMetadata {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        category: PluginCategory,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            category,
            display_name: None,
            description: None,
        }
    }

    pub fn display_name(mut self, value: impl Into<String>) -> Self {
        self.display_name = Some(value.into());
        self
    }

    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }

    pub fn validate(&self) -> Result<(), MetadataError> {
        validate_required("name", &self.name)?;
        validate_required("version", &self.version)?;
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, MetadataError> {
        self.validate()?;
        let mut fields = vec![
            json_field("name", &self.name),
            json_field("version", &self.version),
            json_field("category", self.category.as_str()),
        ];
        if let Some(value) = &self.display_name {
            fields.push(json_field("display_name", value));
        }
        if let Some(value) = &self.description {
            fields.push(json_field("description", value));
        }
        Ok(format!("{{{}}}", fields.join(",")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataError {
    message: String,
}

impl MetadataError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for MetadataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MetadataError {}

pub trait MmPlugin {
    fn metadata(&self) -> PluginMetadata;

    fn initialize(&mut self) {}

    fn shutdown(&mut self) {}
}

pub fn validate_metadata_json(json: &str) -> Result<(), MetadataError> {
    validate_required("metadata", json)?;
    for field in ["name", "version", "category"] {
        let pattern = format!("\"{field}\"");
        if !json.contains(&pattern) {
            return Err(MetadataError::new(format!("metadata missing `{field}`")));
        }
    }
    Ok(())
}

#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        static PLUGIN: std::sync::Mutex<Option<$plugin>> = std::sync::Mutex::new(None);

        #[unsafe(no_mangle)]
        pub extern "C" fn metadata() -> *mut std::ffi::c_char {
            let mut plugin = PLUGIN.lock().expect("plugin lock poisoned");
            let instance = plugin.get_or_insert_with(<$plugin as Default>::default);
            let metadata = <$plugin as $crate::MmPlugin>::metadata(instance)
                .to_json()
                .expect("plugin metadata must be valid");
            std::ffi::CString::new(metadata)
                .expect("plugin metadata must not contain NUL bytes")
                .into_raw()
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn metadata_free(ptr: *mut std::ffi::c_char) {
            if !ptr.is_null() {
                unsafe {
                    let _ = std::ffi::CString::from_raw(ptr);
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn initialize() {
            let mut plugin = PLUGIN.lock().expect("plugin lock poisoned");
            let instance = plugin.get_or_insert_with(<$plugin as Default>::default);
            <$plugin as $crate::MmPlugin>::initialize(instance);
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn shutdown() {
            let mut plugin = PLUGIN.lock().expect("plugin lock poisoned");
            if let Some(instance) = plugin.as_mut() {
                <$plugin as $crate::MmPlugin>::shutdown(instance);
            }
        }
    };
}

fn validate_required(field: &str, value: &str) -> Result<(), MetadataError> {
    if value.trim().is_empty() {
        Err(MetadataError::new(format!(
            "metadata `{field}` is required"
        )))
    } else {
        Ok(())
    }
}

fn json_field(key: &str, value: &str) -> String {
    format!("\"{}\":\"{}\"", escape_json(key), escape_json(value))
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            value if value.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", value as u32));
            }
            value => escaped.push(value),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::{MmPlugin, PluginCategory, PluginMetadata, validate_metadata_json};

    #[derive(Default)]
    struct TestPlugin;

    impl MmPlugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("test", "0.1.0", PluginCategory::Utility)
        }
    }

    #[test]
    fn plugin_returns_metadata() {
        let metadata = TestPlugin.metadata().to_json().unwrap();
        assert!(metadata.contains("\"name\":\"test\""));
        validate_metadata_json(&metadata).unwrap();
    }

    #[test]
    fn metadata_builder_escapes_json() {
        let metadata = PluginMetadata::new("quoted", "0.1.0", PluginCategory::Template)
            .display_name("quote \" plugin")
            .description("line\nbreak")
            .to_json()
            .unwrap();

        assert!(metadata.contains(r#""display_name":"quote \" plugin""#));
        assert!(metadata.contains(r#""description":"line\nbreak""#));
    }

    #[test]
    fn metadata_validation_rejects_empty_name() {
        let error = PluginMetadata::new("", "0.1.0", PluginCategory::Utility)
            .to_json()
            .unwrap_err();

        assert!(error.to_string().contains("name"));
    }
}
