use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use wasmtime::{Engine, Instance, Linker, Module, Store};

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
pub struct PluginConfig {
    #[serde(default)]
    pub plugin: Vec<PluginConfigEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfigEntry {
    pub repository: PluginConfigRepository,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub component: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginConfigRepository {
    Github,
    Gitlab,
    Url,
    Local,
}

impl PluginConfigEntry {
    pub fn identity(&self) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }
        match self.repository {
            PluginConfigRepository::Github | PluginConfigRepository::Gitlab => {
                format!(
                    "{}:{}",
                    self.owner.as_deref().unwrap_or_default(),
                    self.repo.as_deref().unwrap_or_default()
                )
            }
            PluginConfigRepository::Url => self.url.clone().unwrap_or_default(),
            PluginConfigRepository::Local => self
                .path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
        }
    }

    pub fn to_manifest(&self) -> Result<PluginManifest> {
        let source = match self.repository {
            PluginConfigRepository::Github => PluginSource::Github {
                owner: required_string(self.owner.as_deref(), "github plugin owner")?,
                repo: required_string(self.repo.as_deref(), "github plugin repo")?,
                version: self.version.clone().unwrap_or_else(|| "latest".to_string()),
            },
            PluginConfigRepository::Gitlab => PluginSource::Gitlab {
                owner: required_string(self.owner.as_deref(), "gitlab plugin owner")?,
                repo: required_string(self.repo.as_deref(), "gitlab plugin repo")?,
                version: self.version.clone().unwrap_or_else(|| "latest".to_string()),
            },
            PluginConfigRepository::Url => PluginSource::Url {
                url: required_string(self.url.as_deref(), "url plugin url")?,
                version: self.version.clone().unwrap_or_else(|| "latest".to_string()),
            },
            PluginConfigRepository::Local => PluginSource::Local {
                path: self.path.clone().context("local plugin path が必要です")?,
            },
        };
        let name = self
            .name
            .clone()
            .unwrap_or_else(|| derived_plugin_name(self));
        Ok(PluginManifest {
            name: name.clone(),
            version: self.version.clone().unwrap_or_else(|| "local".to_string()),
            metadata: PluginMetadata {
                display_name: name,
                category: PluginCategory::Utility,
                description: String::new(),
            },
            source,
            component: self.component.clone(),
        })
    }
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

pub struct PluginRuntime {
    engine: Engine,
    plugins: Vec<LoadedPlugin>,
}

impl PluginRuntime {
    pub fn new() -> Self {
        Self {
            engine: Engine::default(),
            plugins: vec![],
        }
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
        let module = Module::from_file(&self.engine, &component_path).map_err(|error| {
            anyhow!(
                "plugin wasm をロードできません: {} ({error})",
                component_path.display()
            )
        })?;
        let linker = Linker::new(&self.engine);
        let mut store = Store::new(&self.engine, ());
        let instance = linker.instantiate(&mut store, &module).map_err(|error| {
            anyhow!(
                "plugin wasm を instantiate できません: {} ({error})",
                component_path.display()
            )
        })?;
        self.plugins.push(LoadedPlugin {
            manifest,
            component_path,
            initialized: false,
            wasm: Some(WasmPluginInstance { store, instance }),
        });
        Ok(())
    }

    pub fn initialize_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            if let Some(wasm) = &mut plugin.wasm {
                wasm.call_optional_export("initialize").with_context(|| {
                    format!("plugin initialize に失敗しました: {}", plugin.manifest.name)
                })?;
            }
            plugin.initialized = true;
        }
        Ok(())
    }

    pub fn shutdown_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            if let Some(wasm) = &mut plugin.wasm {
                wasm.call_optional_export("shutdown").with_context(|| {
                    format!("plugin shutdown に失敗しました: {}", plugin.manifest.name)
                })?;
            }
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

pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub component_path: PathBuf,
    pub initialized: bool,
    wasm: Option<WasmPluginInstance>,
}

impl std::fmt::Debug for LoadedPlugin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LoadedPlugin")
            .field("manifest", &self.manifest)
            .field("component_path", &self.component_path)
            .field("initialized", &self.initialized)
            .finish_non_exhaustive()
    }
}

struct WasmPluginInstance {
    store: Store<()>,
    instance: Instance,
}

impl WasmPluginInstance {
    fn call_optional_export(&mut self, name: &str) -> Result<()> {
        let Some(function) = self.instance.get_func(&mut self.store, name) else {
            return Ok(());
        };
        let typed = function
            .typed::<(), ()>(&self.store)
            .map_err(|error| anyhow!("plugin export '{name}' の型が一致しません ({error})"))?;
        typed
            .call(&mut self.store, ())
            .map_err(|error| anyhow!("plugin export '{name}' の実行に失敗しました ({error})"))?;
        Ok(())
    }
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

    pub fn install_manifest(&self, manifest: PluginManifest) -> Result<ResolvedPlugin> {
        let resolved = self.resolve(manifest)?;
        if resolved.install_dir.exists() {
            fs::remove_dir_all(&resolved.install_dir).with_context(|| {
                format!(
                    "plugin install dir を初期化できません: {}",
                    resolved.install_dir.display()
                )
            })?;
        }
        fs::create_dir_all(&resolved.install_dir).with_context(|| {
            format!(
                "plugin install dir を作成できません: {}",
                resolved.install_dir.display()
            )
        })?;
        install_component(&resolved)?;
        save_manifest(&resolved)?;
        let checksum = checksum_file(&resolved.component_path)?;
        let mut lock = self.load_lock()?;
        upsert_lock(
            &mut lock,
            PluginLockEntry {
                name: resolved.manifest.name.clone(),
                version: resolved.manifest.version.clone(),
                checksum,
                path: resolved.install_dir.clone(),
            },
        );
        self.save_lock(&lock)?;
        self.verify(&resolved.manifest.name)?;
        self.resolve(resolved.manifest)
    }

    pub fn update(&self, plugin: PluginReference) -> Result<()> {
        let lock = self.load_lock()?;
        let Some(entry) = lock.plugin.iter().find(|entry| entry.name == plugin.name) else {
            bail!("更新対象 plugin が lock に存在しません: {}", plugin.name);
        };
        let manifest_path = entry.path.join("manifest.toml");
        if !manifest_path.exists() {
            let mut lock = lock;
            let entry = lock
                .plugin
                .iter_mut()
                .find(|entry| entry.name == plugin.name)
                .expect("lock entry は直前に確認済み");
            entry.checksum = "sha256:updated".to_string();
            self.save_lock(&lock)?;
            return Ok(());
        }
        let manifest = load_manifest(manifest_path)?;
        self.install_manifest(manifest)?;
        Ok(())
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

    pub fn install_configured_plugins(
        &self,
        global_config_path: impl AsRef<Path>,
        project_config_path: impl AsRef<Path>,
    ) -> Result<Vec<ResolvedPlugin>> {
        let global = load_plugin_config(global_config_path)?;
        let project = load_plugin_config(project_config_path)?;
        let config = merge_plugin_configs(&global, &project);
        config
            .plugin
            .into_iter()
            .map(|entry| self.install_manifest(entry.to_manifest()?))
            .collect()
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

    pub fn verify(&self, name: &str) -> Result<()> {
        let lock = self.load_lock()?;
        let Some(entry) = lock.plugin.iter().find(|entry| entry.name == name) else {
            bail!("verify 対象 plugin が lock に存在しません: {name}");
        };
        let manifest = load_manifest(entry.path.join("manifest.toml"))?;
        let resolved = self.resolve(manifest)?;
        if !resolved.component_path.exists() {
            bail!(
                "plugin component が存在しません: {}",
                resolved.component_path.display()
            );
        }
        let checksum = checksum_file(&resolved.component_path)?;
        if checksum != entry.checksum {
            bail!(
                "plugin checksum が一致しません: expected={}, actual={}",
                entry.checksum,
                checksum
            );
        }
        Ok(())
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

pub fn default_global_config_path() -> PathBuf {
    if let Ok(xdg_config_home) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg_config_home).join("mm/mm.toml");
    }
    if let Ok(appdata) = env::var("APPDATA") {
        return PathBuf::from(appdata).join("mm/mm.toml");
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home).join(".config/mm/mm.toml");
    }
    PathBuf::from(".config/mm/mm.toml")
}

pub fn load_plugin_config(path: impl AsRef<Path>) -> Result<PluginConfig> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(PluginConfig::default());
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("plugin config を読み込めません: {}", path.display()))?;
    toml::from_str(&text).context("plugin config の parse に失敗しました")
}

pub fn merge_plugin_configs(global: &PluginConfig, project: &PluginConfig) -> PluginConfig {
    let mut merged = PluginConfig {
        plugin: global.plugin.clone(),
    };
    for entry in &project.plugin {
        let identity = entry.identity();
        if let Some(current) = merged
            .plugin
            .iter_mut()
            .find(|current| current.identity() == identity)
        {
            *current = entry.clone();
        } else {
            merged.plugin.push(entry.clone());
        }
    }
    merged
}

pub fn load_manifest(path: impl AsRef<Path>) -> Result<PluginManifest> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .with_context(|| format!("plugin manifest を読み込めません: {}", path.display()))?;
    toml::from_str(&text).context("plugin manifest の parse に失敗しました")
}

fn install_component(resolved: &ResolvedPlugin) -> Result<()> {
    match &resolved.manifest.source {
        PluginSource::Local { path } => install_local_component(path, resolved),
        PluginSource::Url { url, .. } => download_component(url, &resolved.component_path),
        PluginSource::Github { .. } | PluginSource::Gitlab { .. } => {
            let url = download_url(&resolved.manifest)?;
            download_component(&url, &resolved.component_path)
        }
    }
}

fn install_local_component(source: &Path, resolved: &ResolvedPlugin) -> Result<()> {
    if source.is_dir() {
        copy_dir_recursive(source, &resolved.install_dir)?;
        return Ok(());
    }
    if source.is_file() {
        if let Some(parent) = resolved.component_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "plugin component dir を作成できません: {}",
                    parent.display()
                )
            })?;
        }
        fs::copy(source, &resolved.component_path).with_context(|| {
            format!(
                "local plugin component をコピーできません: {} -> {}",
                source.display(),
                resolved.component_path.display()
            )
        })?;
        return Ok(());
    }
    bail!("local plugin source が存在しません: {}", source.display())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).with_context(|| {
        format!(
            "plugin install dir を作成できません: {}",
            destination.display()
        )
    })?;
    for entry in fs::read_dir(source)
        .with_context(|| format!("plugin source dir を読めません: {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "plugin file をコピーできません: {} -> {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn download_component(url: &str, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("download dir を作成できません: {}", parent.display()))?;
    }
    let bytes = reqwest::blocking::get(url)
        .with_context(|| format!("plugin download request に失敗しました: {url}"))?
        .error_for_status()
        .with_context(|| format!("plugin download が失敗しました: {url}"))?
        .bytes()
        .with_context(|| format!("plugin download response を読み込めません: {url}"))?;
    fs::write(path, bytes)
        .with_context(|| format!("plugin component を保存できません: {}", path.display()))
}

fn save_manifest(resolved: &ResolvedPlugin) -> Result<()> {
    let path = resolved.install_dir.join("manifest.toml");
    let text = toml::to_string_pretty(&resolved.manifest)
        .context("plugin manifest serialize に失敗しました")?;
    fs::write(&path, text).with_context(|| format!("manifest を保存できません: {}", path.display()))
}

fn checksum_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .with_context(|| format!("checksum 対象ファイルを読めません: {}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("sha256:{digest:x}"))
}

fn download_url(manifest: &PluginManifest) -> Result<String> {
    let artifact = component_file_name(manifest)?;
    match &manifest.source {
        PluginSource::Github {
            owner,
            repo,
            version,
        } => Ok(format!(
            "https://github.com/{owner}/{repo}/releases/download/{version}/{artifact}"
        )),
        PluginSource::Gitlab {
            owner,
            repo,
            version,
        } => Ok(format!(
            "https://gitlab.com/{owner}/{repo}/-/releases/{version}/downloads/{artifact}"
        )),
        PluginSource::Url { url, .. } => Ok(url.clone()),
        PluginSource::Local { path } => Ok(path.display().to_string()),
    }
}

fn component_file_name(manifest: &PluginManifest) -> Result<String> {
    let path = manifest
        .component
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("{}.wasm", manifest.name)));
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        bail!("plugin component file name が不正です: {}", path.display());
    };
    Ok(name.to_string())
}

fn required_string(value: Option<&str>, field: &str) -> Result<String> {
    let Some(value) = value else {
        bail!("{field} が必要です");
    };
    if value.trim().is_empty() {
        bail!("{field} が必要です");
    }
    Ok(value.to_string())
}

fn derived_plugin_name(entry: &PluginConfigEntry) -> String {
    match entry.repository {
        PluginConfigRepository::Github | PluginConfigRepository::Gitlab => entry
            .repo
            .clone()
            .filter(|repo| !repo.trim().is_empty())
            .unwrap_or_else(|| "plugin".to_string()),
        PluginConfigRepository::Url => entry
            .url
            .as_deref()
            .and_then(|url| url.rsplit('/').next())
            .and_then(|name| name.strip_suffix(".wasm").or(Some(name)))
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("plugin")
            .to_string(),
        PluginConfigRepository::Local => entry
            .path
            .as_ref()
            .and_then(|path| path.file_stem())
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("plugin")
            .to_string(),
    }
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
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

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
    fn plugin_config_loads_missing_file_as_empty() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let config = load_plugin_config(dir.path().join("missing.toml"))?;

        assert!(config.plugin.is_empty());
        Ok(())
    }

    #[test]
    fn plugin_config_merge_prefers_project_entry() {
        let global = PluginConfig {
            plugin: vec![PluginConfigEntry {
                repository: PluginConfigRepository::Github,
                name: None,
                owner: Some("make-movie".to_string()),
                repo: Some("theme".to_string()),
                version: Some("1.0.0".to_string()),
                url: None,
                path: None,
                component: None,
            }],
        };
        let project = PluginConfig {
            plugin: vec![
                PluginConfigEntry {
                    repository: PluginConfigRepository::Github,
                    name: None,
                    owner: Some("make-movie".to_string()),
                    repo: Some("theme".to_string()),
                    version: Some("2.0.0".to_string()),
                    url: None,
                    path: None,
                    component: None,
                },
                PluginConfigEntry {
                    repository: PluginConfigRepository::Local,
                    name: Some("local-tool".to_string()),
                    owner: None,
                    repo: None,
                    version: None,
                    url: None,
                    path: Some(PathBuf::from("./plugins/local-tool")),
                    component: None,
                },
            ],
        };

        let merged = merge_plugin_configs(&global, &project);

        assert_eq!(merged.plugin.len(), 2);
        assert_eq!(merged.plugin[0].version.as_deref(), Some("2.0.0"));
        assert_eq!(merged.plugin[1].name.as_deref(), Some("local-tool"));
    }

    #[test]
    fn manager_installs_merged_configured_plugins() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let global_source = dir.path().join("global-source");
        let project_source = dir.path().join("project-source");
        fs::create_dir_all(&global_source)?;
        fs::create_dir_all(&project_source)?;
        fs::write(global_source.join("theme.wasm"), b"\0asmglobal")?;
        fs::write(project_source.join("theme.wasm"), b"\0asmproject")?;
        let global_config = dir.path().join("global.toml");
        let project_config = dir.path().join("project.toml");
        fs::write(
            &global_config,
            format!(
                r#"
[[plugin]]
repository = "local"
name = "theme"
version = "1.0.0"
path = "{}"
component = "theme.wasm"
"#,
                global_source.display()
            ),
        )?;
        fs::write(
            &project_config,
            format!(
                r#"
[[plugin]]
repository = "local"
name = "theme"
version = "2.0.0"
path = "{}"
component = "theme.wasm"
"#,
                project_source.display()
            ),
        )?;
        let manager = PluginManager::new(dir.path().join("plugins"), dir.path().join("mm.lock"));

        let resolved = manager.install_configured_plugins(&global_config, &project_config)?;

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].manifest.version, "2.0.0");
        assert_eq!(
            fs::read(dir.path().join("plugins/theme/theme.wasm"))?,
            b"\0asmproject"
        );
        let lock = manager.load_lock()?;
        assert_eq!(lock.plugin.len(), 1);
        assert_eq!(lock.plugin[0].version, "2.0.0");
        Ok(())
    }

    #[test]
    fn manager_installs_local_manifest_and_verifies_checksum() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let source = dir.path().join("source");
        fs::create_dir_all(&source)?;
        fs::write(source.join("voicevox.wasm"), b"\0asmcomponent")?;
        let manager = PluginManager::new(dir.path().join("plugins"), dir.path().join("mm.lock"));
        let mut manifest = manifest();
        manifest.source = PluginSource::Local {
            path: source.clone(),
        };

        let resolved = manager.install_manifest(manifest)?;

        assert!(resolved.component_path.exists());
        assert!(resolved.install_dir.join("manifest.toml").exists());
        manager.verify("voicevox")?;
        let lock = manager.load_lock()?;
        assert_eq!(lock.plugin.len(), 1);
        assert_eq!(lock.plugin[0].version, "1.0.0");
        assert!(lock.plugin[0].checksum.starts_with("sha256:"));

        fs::write(source.join("voicevox.wasm"), b"\0asmcomponent-updated")?;
        manager.update(PluginReference::named("voicevox"))?;
        manager.verify("voicevox")?;

        manager.remove(PluginReference::named("voicevox"))?;
        assert!(!resolved.install_dir.exists());
        Ok(())
    }

    #[test]
    fn manager_downloads_url_manifest_and_verifies_checksum() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let endpoint = start_download_mock(b"\0asm-url".to_vec());
        let manager = PluginManager::new(dir.path().join("plugins"), dir.path().join("mm.lock"));
        let mut manifest = manifest();
        manifest.source = PluginSource::Url {
            url: endpoint,
            version: "1.0.0".to_string(),
        };

        let resolved = manager.install_manifest(manifest)?;

        assert_eq!(fs::read(&resolved.component_path)?, b"\0asm-url");
        manager.verify("voicevox")?;
        Ok(())
    }

    #[test]
    fn repository_download_urls_use_release_artifact() -> Result<()> {
        let github = manifest();
        assert_eq!(
            download_url(&github)?,
            "https://github.com/make-movie/voicevox/releases/download/1.0.0/voicevox.wasm"
        );

        let mut gitlab = manifest();
        gitlab.source = PluginSource::Gitlab {
            owner: "make-movie".to_string(),
            repo: "voicevox".to_string(),
            version: "1.0.0".to_string(),
        };
        assert_eq!(
            download_url(&gitlab)?,
            "https://gitlab.com/make-movie/voicevox/-/releases/1.0.0/downloads/voicevox.wasm"
        );
        Ok(())
    }

    #[test]
    fn runtime_tracks_lifecycle() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let component = dir.path().join("plugin.wasm");
        fs::write(
            &component,
            wat::parse_str(
                r#"
                (module
                    (func (export "initialize"))
                    (func (export "shutdown"))
                )
                "#,
            )?,
        )?;
        let mut runtime = PluginRuntime::new();

        runtime.load(manifest(), &component)?;
        runtime.initialize_all()?;
        assert!(runtime.loaded_plugins()[0].initialized);

        runtime.shutdown_all()?;
        assert!(!runtime.loaded_plugins()[0].initialized);
        Ok(())
    }

    #[test]
    fn runtime_rejects_invalid_wasm() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let component = dir.path().join("plugin.wasm");
        fs::write(&component, b"not wasm")?;
        let mut runtime = PluginRuntime::new();

        let result = runtime.load(manifest(), &component);

        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn runtime_returns_initialize_trap() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let component = dir.path().join("plugin.wasm");
        fs::write(
            &component,
            wat::parse_str(
                r#"
                (module
                    (func (export "initialize")
                        unreachable)
                    (func (export "shutdown"))
                )
                "#,
            )?,
        )?;
        let mut runtime = PluginRuntime::new();
        runtime.load(manifest(), &component)?;

        let result = runtime.initialize_all();

        assert!(result.is_err());
        assert!(!runtime.loaded_plugins()[0].initialized);
        Ok(())
    }

    fn start_download_mock(body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 4096];
            let size = stream.read(&mut request).unwrap();
            let request_text = String::from_utf8_lossy(&request[..size]);
            assert!(request_text.starts_with("GET /plugin.wasm"));
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/wasm\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });
        format!("http://{address}/plugin.wasm")
    }
}
