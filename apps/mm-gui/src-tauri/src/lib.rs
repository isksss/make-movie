use mm_core::{
    import_asset as import_core_asset, load_project as load_core_project,
    save_project as save_core_project,
};
use mm_plugin_runtime::{PluginManager, PluginReference};
use mm_render::{render_project, RenderOptions};
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
fn install_plugin(name: String) -> Result<(), String> {
    PluginManager::default()
        .install(PluginReference::named(name))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_plugin(name: String) -> Result<(), String> {
    PluginManager::default()
        .update(PluginReference::named(name))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn remove_plugin(name: String) -> Result<(), String> {
    PluginManager::default()
        .remove(PluginReference::named(name))
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_project,
            save_project,
            build_project,
            import_asset,
            install_plugin,
            update_plugin,
            remove_plugin
        ])
        .run(tauri::generate_context!())
        .expect("tauri application error");
}
