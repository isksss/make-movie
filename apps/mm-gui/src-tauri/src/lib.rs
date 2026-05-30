use mm_core::{
    import_asset as import_core_asset, import_asset_into_project as import_core_asset_into_project,
    load_project as load_core_project, save_project as save_core_project,
};
use mm_plugin_runtime::{PluginManager, PluginReference};
use mm_render::{render_project, RenderOptions};
use std::env;
use std::path::PathBuf;

#[tauri::command]
fn load_project(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let project = load_core_project(&path).map_err(|error| error.to_string())?;
    toml::to_string_pretty(&project).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_project(path: String, toml: String) -> Result<(), String> {
    let project: mm_core::Project = toml::from_str(&toml).map_err(|error| error.to_string())?;
    save_core_project(&project, path).map_err(|error| error.to_string())
}

#[tauri::command]
fn build_project(path: String) -> Result<(), String> {
    let project = load_core_project(&path).map_err(|error| error.to_string())?;
    let project_path = PathBuf::from(&path);
    let project_root = project_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let options = RenderOptions::new(project_root, project.settings.output.clone());
    render_project(&project, &options).map_err(|error| error.to_string())
}

#[tauri::command]
fn import_asset(
    project_root: String,
    source_path: String,
    kind: mm_core::AssetKind,
) -> Result<String, String> {
    let asset =
        import_core_asset(project_root, source_path, kind).map_err(|error| error.to_string())?;
    toml::to_string_pretty(&asset).map_err(|error| error.to_string())
}

#[tauri::command]
fn import_asset_into_project(
    project_path: String,
    source_path: String,
    kind: mm_core::AssetKind,
) -> Result<String, String> {
    let project = import_core_asset_into_project(project_path, source_path, kind)
        .map_err(|error| error.to_string())?;
    toml::to_string_pretty(&project).map_err(|error| error.to_string())
}

#[tauri::command]
fn install_plugin(name: String) -> Result<(), String> {
    plugin_manager()
        .install(PluginReference::named(name))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_plugin(name: String) -> Result<(), String> {
    plugin_manager()
        .update(PluginReference::named(name))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn remove_plugin(name: String) -> Result<(), String> {
    plugin_manager()
        .remove(PluginReference::named(name))
        .map_err(|error| error.to_string())
}

fn plugin_manager() -> PluginManager {
    match (env::var("MM_PLUGIN_DIR"), env::var("MM_PLUGIN_LOCK")) {
        (Ok(plugin_dir), Ok(lock_path)) => PluginManager::new(plugin_dir, lock_path),
        _ => PluginManager::default(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_project,
            save_project,
            build_project,
            import_asset,
            import_asset_into_project,
            install_plugin,
            update_plugin,
            remove_plugin
        ])
        .run(tauri::generate_context!())
        .expect("tauri application error");
}

#[cfg(test)]
mod tests {
    use super::*;
    use mm_core::AssetKind;

    #[test]
    fn project_commands_load_save_and_import_asset() {
        let dir = tempfile::tempdir().expect("temp dir を作成できる");
        let project_path = dir.path().join("mm.toml");
        let source_path = dir.path().join("source.png");
        std::fs::write(&source_path, [0u8]).expect("import元ファイルを書き込める");

        save_project(
            project_path.display().to_string(),
            sample_project_toml("Tauri E2E"),
        )
        .expect("Tauri command で project を保存できる");

        let loaded =
            load_project(project_path.display().to_string()).expect("project を読み込める");
        assert!(loaded.contains("Tauri E2E"));

        let asset = import_asset(
            dir.path().display().to_string(),
            source_path.display().to_string(),
            AssetKind::Image,
        )
        .expect("asset を取り込める");
        assert!(asset.contains("kind = \"image\""));
        assert!(dir.path().join("media/image/source.png").exists());

        let project = import_asset_into_project(
            project_path.display().to_string(),
            source_path.display().to_string(),
            AssetKind::Image,
        )
        .expect("asset を project へ登録して配置できる");
        assert!(project.contains("[[assets]]"));
        assert!(project.contains("[[tracks.layers]]"));
        let saved = std::fs::read_to_string(&project_path).expect("project を読める");
        assert!(saved.contains("[[assets]]"));
    }

    #[test]
    fn plugin_commands_install_update_and_remove() {
        let dir = tempfile::tempdir().expect("temp dir を作成できる");
        env::set_var("MM_PLUGIN_DIR", dir.path().join("plugins"));
        env::set_var("MM_PLUGIN_LOCK", dir.path().join("mm.lock"));

        install_plugin("theme".to_string()).expect("plugin を install できる");
        update_plugin("theme".to_string()).expect("plugin を update できる");
        remove_plugin("theme".to_string()).expect("plugin を remove できる");

        env::remove_var("MM_PLUGIN_DIR");
        env::remove_var("MM_PLUGIN_LOCK");
        let lock = std::fs::read_to_string(dir.path().join("mm.lock")).expect("lock を読める");
        assert!(!lock.contains("theme"));
    }

    fn sample_project_toml(title: &str) -> String {
        format!(
            r#"[settings]
title = "{title}"
width = 1080
height = 1920
fps = 30
sample_rate = 48000
duration = 1.0
output = "output/movie.mp4"
asset_mode = "copy"
"#
        )
    }
}
