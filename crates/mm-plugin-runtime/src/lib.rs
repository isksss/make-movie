use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub metadata: PluginMetadata,
    pub source: PluginSource,
    #[serde(default)]
    pub component: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub display_name: String,
    pub category: PluginCategory,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    Ai,
    Subtitle,
    Tts,
    Template,
    Export,
    Utility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "repository", rename_all = "snake_case")]
pub enum PluginSource {
    Github {
        owner: String,
        repo: String,
        version: String,
    },
    Gitlab {
        owner: String,
        repo: String,
        version: String,
    },
    Url {
        url: String,
        version: String,
    },
    Local {
        path: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PluginLock {
    #[serde(default)]
    pub plugin: Vec<PluginLockEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginLockEntry {
    pub name: String,
    pub version: String,
    pub checksum: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ResolvedPlugin {
    pub manifest: PluginManifest,
    pub install_dir: PathBuf,
    pub component_path: PathBuf,
    pub checksum: String,
}

#[derive(Debug, Clone)]
pub struct PluginRuntime {
    plugins: Vec<LoadedPlugin>,
}

impl PluginRuntime {
    pub fn new() -> Self {
        Self { plugins: vec![] }
    }

    pub fn load(
        &mut self,
        manifest: PluginManifest,
        component_path: impl Into<PathBuf>,
    ) -> Result<()> {
        let component_path = component_path.into();
        if !component_path.exists() {
            bail!(
                "plugin component が存在しません: {}",
                component_path.display()
            );
        }
        self.plugins.push(LoadedPlugin {
            manifest,
            component_path,
            initialized: false,
        });
        Ok(())
    }

    pub fn initialize_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.initialized = true;
        }
        Ok(())
    }

    pub fn shutdown_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.initialized = false;
        }
        Ok(())
    }

    pub fn loaded_plugins(&self) -> &[LoadedPlugin] {
        &self.plugins
    }
}

impl Default for PluginRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub component_path: PathBuf,
    pub initialized: bool,
}

#[derive(Debug, Clone)]
pub struct PluginManager {
    pub plugin_dir: PathBuf,
    pub lock_path: PathBuf,
}

impl PluginManager {
    pub fn new(plugin_dir: impl Into<PathBuf>, lock_path: impl Into<PathBuf>) -> Self {
        Self {
            plugin_dir: plugin_dir.into(),
            lock_path: lock_path.into(),
        }
    }

    pub fn install(&self, plugin: PluginReference) -> Result<()> {
        let install_dir = self.plugin_dir.join(&plugin.name);
        fs::create_dir_all(&install_dir).with_context(|| {
            format!(
                "plugin install dir を作成できません: {}",
                install_dir.display()
            )
        })?;
        let mut lock = self.load_lock()?;
        upsert_lock(
            &mut lock,
            PluginLockEntry {
                name: plugin.name,
                version: "local".to_string(),
                checksum: "sha256:unverified".to_string(),
                path: install_dir,
            },
        );
        self.save_lock(&lock)
    }

    pub fn update(&self, plugin: PluginReference) -> Result<()> {
        let mut lock = self.load_lock()?;
        let Some(entry) = lock
            .plugin
            .iter_mut()
            .find(|entry| entry.name == plugin.name)
        else {
            bail!("更新対象 plugin が lock に存在しません: {}", plugin.name);
        };
        entry.checksum = "sha256:updated".to_string();
        self.save_lock(&lock)
    }

    pub fn remove(&self, plugin: PluginReference) -> Result<()> {
        let mut lock = self.load_lock()?;
        lock.plugin.retain(|entry| entry.name != plugin.name);
        let install_dir = self.plugin_dir.join(&plugin.name);
        if install_dir.exists() {
            fs::remove_dir_all(&install_dir).with_context(|| {
                format!(
                    "plugin install dir を削除できません: {}",
                    install_dir.display()
                )
            })?;
        }
        self.save_lock(&lock)
    }

    pub fn resolve(&self, manifest: PluginManifest) -> Result<ResolvedPlugin> {
        let install_dir = self.plugin_dir.join(&manifest.name);
        let component_path = manifest
            .component
            .clone()
            .map(|path| install_dir.join(path))
            .unwrap_or_else(|| install_dir.join(format!("{}.wasm", manifest.name)));
        Ok(ResolvedPlugin {
            checksum: checksum_hint(&manifest),
            manifest,
            install_dir,
            component_path,
        })
    }

    pub fn load_lock(&self) -> Result<PluginLock> {
        if !self.lock_path.exists() {
            return Ok(PluginLock::default());
        }
        let text = fs::read_to_string(&self.lock_path)
            .with_context(|| format!("lock を読み込めません: {}", self.lock_path.display()))?;
        toml::from_str(&text).context("mm.lock の parse に失敗しました")
    }

    pub fn save_lock(&self, lock: &PluginLock) -> Result<()> {
        if let Some(parent) = self.lock_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("lock dir を作成できません: {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(lock).context("mm.lock の serialize に失敗しました")?;
        fs::write(&self.lock_path, text)
            .with_context(|| format!("lock を保存できません: {}", self.lock_path.display()))
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        let base = default_plugin_dir();
        Self::new(base, PathBuf::from("mm.lock"))
    }
}

pub fn default_plugin_dir() -> PathBuf {
    if let Ok(xdg_data_home) = env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg_data_home).join("mm/plugins");
    }
    if let Ok(appdata) = env::var("APPDATA") {
        return PathBuf::from(appdata).join("mm/plugins");
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home).join(".local/share/mm/plugins");
    }
    PathBuf::from(".mm/plugins")
}

pub fn load_manifest(path: impl AsRef<Path>) -> Result<PluginManifest> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .with_context(|| format!("plugin manifest を読み込めません: {}", path.display()))?;
    toml::from_str(&text).context("plugin manifest の parse に失敗しました")
}

fn upsert_lock(lock: &mut PluginLock, entry: PluginLockEntry) {
    if let Some(current) = lock
        .plugin
        .iter_mut()
        .find(|current| current.name == entry.name)
    {
        *current = entry;
    } else {
        lock.plugin.push(entry);
    }
}

fn checksum_hint(manifest: &PluginManifest) -> String {
    let source = match &manifest.source {
        PluginSource::Github {
            owner,
            repo,
            version,
        } => format!("github:{owner}/{repo}@{version}"),
        PluginSource::Gitlab {
            owner,
            repo,
            version,
        } => format!("gitlab:{owner}/{repo}@{version}"),
        PluginSource::Url { url, version } => format!("url:{url}@{version}"),
        PluginSource::Local { path } => format!("local:{}", path.display()),
    };
    format!("sha256:{}", stable_hex(source.as_bytes()))
}

fn stable_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PluginManifest {
        PluginManifest {
            name: "voicevox".to_string(),
            version: "1.0.0".to_string(),
            metadata: PluginMetadata {
                display_name: "VOICEVOX".to_string(),
                category: PluginCategory::Tts,
                description: "TTS".to_string(),
            },
            source: PluginSource::Github {
                owner: "make-movie".to_string(),
                repo: "voicevox".to_string(),
                version: "1.0.0".to_string(),
            },
            component: Some(PathBuf::from("voicevox.wasm")),
        }
    }

    #[test]
    fn manifest_round_trip_toml() -> Result<()> {
        let text = toml::to_string(&manifest())?;
        let loaded: PluginManifest = toml::from_str(&text)?;

        assert_eq!(loaded.name, "voicevox");
        assert_eq!(loaded.metadata.category, PluginCategory::Tts);
        Ok(())
    }

    #[test]
    fn manager_installs_updates_and_removes_lock_entry() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let manager = PluginManager::new(dir.path().join("plugins"), dir.path().join("mm.lock"));

        manager.install(PluginReference::named("theme"))?;
        let lock = manager.load_lock()?;
        assert_eq!(lock.plugin.len(), 1);

        manager.update(PluginReference::named("theme"))?;
        let lock = manager.load_lock()?;
        assert_eq!(lock.plugin[0].checksum, "sha256:updated");

        manager.remove(PluginReference::named("theme"))?;
        let lock = manager.load_lock()?;
        assert!(lock.plugin.is_empty());
        Ok(())
    }

    #[test]
    fn runtime_tracks_lifecycle() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let component = dir.path().join("plugin.wasm");
        fs::write(&component, [])?;
        let mut runtime = PluginRuntime::new();

        runtime.load(manifest(), &component)?;
        runtime.initialize_all()?;
        assert!(runtime.loaded_plugins()[0].initialized);

        runtime.shutdown_all()?;
        assert!(!runtime.loaded_plugins()[0].initialized);
        Ok(())
    }
}
