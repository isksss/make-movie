use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginReference {
    pub name: String,
}

impl PluginReference {
    pub fn named(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginLockEntry {
    pub name: String,
    pub version: String,
    pub checksum: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct PluginManager;

impl PluginManager {
    pub fn install(&self, plugin: PluginReference) -> Result<()> {
        println!("plugin install: {}", plugin.name);
        Ok(())
    }

    pub fn update(&self, plugin: PluginReference) -> Result<()> {
        println!("plugin update: {}", plugin.name);
        Ok(())
    }

    pub fn remove(&self, plugin: PluginReference) -> Result<()> {
        println!("plugin remove: {}", plugin.name);
        Ok(())
    }
}
