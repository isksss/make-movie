use mm_core::{
    ExternalAnalysisOptions, ExternalAnalysisResult, apply_external_analysis_to_layer,
    import_asset as import_core_asset, import_asset_into_project as import_core_asset_into_project,
    load_project as load_core_project, save_project as save_core_project,
};
use mm_plugin_runtime::{
    PluginManager, PluginReference, default_global_config_path, default_plugin_dir,
};
use mm_render::{
    BundledFfmpegLocator, FfmpegLocator, RenderBackend, RenderOptions,
    probe_gpu_backend as probe_render_gpu_backend, render_frame_with_backend, render_project,
    system_font_families,
};
use serde::Serialize;
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
    build_project_with_backend(path, "auto".to_string())
}

#[tauri::command]
fn build_project_with_backend(path: String, backend: String) -> Result<(), String> {
    let project = load_core_project(&path).map_err(|error| error.to_string())?;
    let project_path = PathBuf::from(&path);
    let mut options = gui_render_options(&project_path, &project, bundled_binary_dir());
    options.backend = parse_render_backend(&backend).map_err(|error| error.to_string())?;
    render_project(&project, &options).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_system_fonts() -> Vec<String> {
    system_font_families()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiGpuProbe {
    available: bool,
    adapter_name: Option<String>,
}

#[tauri::command]
fn probe_gpu_backend() -> GuiGpuProbe {
    let probe = probe_render_gpu_backend();
    GuiGpuProbe {
        available: probe.available,
        adapter_name: probe.adapter_name,
    }
}

#[tauri::command]
fn render_preview_frame(path: String, time: f64, backend: String) -> Result<String, String> {
    let project = load_core_project(&path).map_err(|error| error.to_string())?;
    let project_path = PathBuf::from(&path);
    let project_root = project_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let output = project_root.join("cache/preview.png");
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut options = RenderOptions::new(project_root, &output);
    options.backend = parse_render_backend(&backend).map_err(|error| error.to_string())?;
    let frame = render_frame_with_backend(
        &project,
        &options,
        time.clamp(0.0, project.settings.duration),
    )
    .map_err(|error| error.to_string())?;
    frame.save(&output).map_err(|error| error.to_string())?;
    Ok(output.display().to_string())
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
fn apply_external_analysis_result(
    project_toml: String,
    layer_id: String,
    analysis_json: String,
) -> Result<String, String> {
    let mut project: mm_core::Project =
        toml::from_str(&project_toml).map_err(|error| error.to_string())?;
    let analysis: ExternalAnalysisResult =
        serde_json::from_str(&analysis_json).map_err(|error| error.to_string())?;
    apply_external_analysis_to_layer(
        &mut project,
        &layer_id,
        &analysis,
        &ExternalAnalysisOptions::default(),
    )
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

#[tauri::command]
fn install_configured_plugins(
    project_path: String,
    global_config: Option<String>,
) -> Result<Vec<String>, String> {
    let project_path = PathBuf::from(project_path);
    let project_root = project_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let manager = PluginManager::new(plugin_dir(), project_root.join("mm.lock"));
    let global_config = global_config
        .map(PathBuf::from)
        .unwrap_or_else(default_global_config_path);
    let plugins = manager
        .install_configured_plugins(global_config, &project_path)
        .map_err(|error| error.to_string())?;
    Ok(plugins
        .into_iter()
        .map(|plugin| plugin.manifest.name)
        .collect())
}

fn plugin_manager() -> PluginManager {
    match (env::var("MM_PLUGIN_DIR"), env::var("MM_PLUGIN_LOCK")) {
        (Ok(plugin_dir), Ok(lock_path)) => PluginManager::new(plugin_dir, lock_path),
        _ => PluginManager::default(),
    }
}

fn plugin_dir() -> PathBuf {
    env::var("MM_PLUGIN_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_plugin_dir())
}

fn gui_render_options(
    project_path: &std::path::Path,
    project: &mm_core::Project,
    bundle_dir: Option<PathBuf>,
) -> RenderOptions {
    let project_root = project_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut options = RenderOptions::new(project_root, project.settings.output.clone());
    if let Some(ffmpeg_path) = bundle_dir.and_then(|dir| bundled_ffmpeg_path(&dir, project)) {
        options.ffmpeg_path = Some(ffmpeg_path);
    }
    options
}

fn bundled_binary_dir() -> Option<PathBuf> {
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
}

fn bundled_ffmpeg_path(
    bundle_dir: &std::path::Path,
    project: &mm_core::Project,
) -> Option<PathBuf> {
    BundledFfmpegLocator::new(bundle_dir)
        .ffmpeg_path(project)
        .ok()
}

fn parse_render_backend(value: &str) -> anyhow::Result<RenderBackend> {
    match value.to_ascii_lowercase().as_str() {
        "auto" => Ok(RenderBackend::Auto),
        "cpu" => Ok(RenderBackend::Cpu),
        "skia" => Ok(RenderBackend::Skia),
        "gpu" => Ok(RenderBackend::Gpu),
        other => anyhow::bail!("不明なレンダリングバックエンドです: {other}"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_project,
            save_project,
            build_project,
            build_project_with_backend,
            render_preview_frame,
            probe_gpu_backend,
            list_system_fonts,
            import_asset,
            import_asset_into_project,
            apply_external_analysis_result,
            install_plugin,
            install_configured_plugins,
            update_plugin,
            remove_plugin
        ]);

    #[cfg(feature = "e2e-testing")]
    let builder = builder.plugin(tauri_plugin_playwright::init());

    builder
        .run(tauri::generate_context!())
        .expect("tauri application error");
}

#[cfg(test)]
mod tests {
    use super::*;
    use mm_core::AssetKind;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock poisoned")
    }

    #[test]
    fn list_system_fonts_returns_sorted_unique_names() {
        let fonts = list_system_fonts();
        assert!(fonts.windows(2).all(|window| window[0] < window[1]));
    }

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
        let _guard = env_lock();
        let dir = tempfile::tempdir().expect("temp dir を作成できる");
        // SAFETY: This test owns MM_PLUGIN_DIR and MM_PLUGIN_LOCK while holding env_lock.
        unsafe {
            env::set_var("MM_PLUGIN_DIR", dir.path().join("plugins"));
            env::set_var("MM_PLUGIN_LOCK", dir.path().join("mm.lock"));
        }

        install_plugin("theme".to_string()).expect("plugin を install できる");
        update_plugin("theme".to_string()).expect("plugin を update できる");
        remove_plugin("theme".to_string()).expect("plugin を remove できる");

        // SAFETY: This test owns MM_PLUGIN_DIR and MM_PLUGIN_LOCK while holding env_lock.
        unsafe {
            env::remove_var("MM_PLUGIN_DIR");
            env::remove_var("MM_PLUGIN_LOCK");
        }
        let lock = std::fs::read_to_string(dir.path().join("mm.lock")).expect("lock を読める");
        assert!(!lock.contains("theme"));
    }

    #[test]
    fn plugin_command_installs_configured_plugins_to_project_lock() {
        let _guard = env_lock();
        let dir = tempfile::tempdir().expect("temp dir を作成できる");
        let plugin_dir = dir.path().join("plugins");
        let project_path = dir.path().join("mm.toml");
        let component_path = dir.path().join("theme.wasm");
        std::fs::write(&component_path, b"\0asmcomponent").expect("component を書ける");
        std::fs::write(
            &project_path,
            format!(
                r#"[[plugin]]
repository = "local"
name = "theme"
path = "{}"
"#,
                component_path.display()
            ),
        )
        .expect("project config を書ける");
        // SAFETY: This test owns MM_PLUGIN_DIR while holding env_lock.
        unsafe {
            env::set_var("MM_PLUGIN_DIR", &plugin_dir);
        }

        let installed = install_configured_plugins(project_path.display().to_string(), None)
            .expect("設定済み plugin を install できる");

        // SAFETY: This test owns MM_PLUGIN_DIR while holding env_lock.
        unsafe {
            env::remove_var("MM_PLUGIN_DIR");
        }
        assert_eq!(installed, vec!["theme"]);
        let lock =
            std::fs::read_to_string(dir.path().join("mm.lock")).expect("project lock を読める");
        assert!(lock.contains("theme"));
        assert!(plugin_dir.join("theme/theme.wasm").exists());
    }

    #[test]
    fn apply_external_analysis_result_adds_position_and_crop_animations() {
        let project = sample_video_project_toml();
        let analysis = r#"{
  "source_width": 640,
  "source_height": 360,
  "targets": [
    {
      "id": "target-1",
      "kind": "person",
      "frames": [
        { "time": 0.0, "bbox": { "x": 10.0, "y": 20.0, "width": 100.0, "height": 80.0 } },
        { "time": 0.5, "bbox": { "x": 30.0, "y": 40.0, "width": 120.0, "height": 90.0 } }
      ]
    }
  ]
}"#;

        let updated =
            apply_external_analysis_result(project, "video-main".to_string(), analysis.to_string())
                .expect("external analysis を適用できる");

        assert!(updated.contains("property = \"x\""));
        assert!(updated.contains("property = \"crop_width\""));
        assert!(updated.contains("value = 60.0"));
    }

    #[test]
    fn gui_render_options_prefers_bundled_ffmpeg() {
        let dir = tempfile::tempdir().expect("temp dir を作成できる");
        let bundle_dir = dir.path().join("bundle");
        std::fs::create_dir_all(&bundle_dir).expect("bundle dir を作成できる");
        let bundled_ffmpeg = bundle_dir.join("ffmpeg");
        std::fs::write(&bundled_ffmpeg, []).expect("同梱 ffmpeg を書ける");
        let project_path = dir.path().join("project/mm.toml");
        let mut project: mm_core::Project =
            toml::from_str(&sample_project_toml("Bundled")).expect("project TOML を parse できる");
        project.settings.ffmpeg = Some(PathBuf::from("/custom/ffmpeg"));

        let options = gui_render_options(&project_path, &project, Some(bundle_dir));

        assert_eq!(options.project_root, dir.path().join("project"));
        assert_eq!(options.ffmpeg_path, Some(bundled_ffmpeg));
    }

    #[test]
    fn gui_render_options_keeps_fallback_when_bundled_ffmpeg_is_missing() {
        let dir = tempfile::tempdir().expect("temp dir を作成できる");
        let project_path = dir.path().join("mm.toml");
        let mut project: mm_core::Project =
            toml::from_str(&sample_project_toml("Fallback")).expect("project TOML を parse できる");
        project.settings.ffmpeg = Some(PathBuf::from("/custom/ffmpeg"));

        let options = gui_render_options(&project_path, &project, Some(dir.path().join("bundle")));

        assert_eq!(options.ffmpeg_path, None);
    }

    #[test]
    fn parse_render_backend_accepts_gui_values() {
        assert_eq!(parse_render_backend("auto").unwrap(), RenderBackend::Auto);
        assert_eq!(parse_render_backend("cpu").unwrap(), RenderBackend::Cpu);
        assert_eq!(parse_render_backend("skia").unwrap(), RenderBackend::Skia);
        assert_eq!(parse_render_backend("gpu").unwrap(), RenderBackend::Gpu);
        assert!(parse_render_backend("metal").is_err());
    }

    #[test]
    fn render_preview_frame_writes_png_with_selected_backend() {
        let dir = tempfile::tempdir().expect("temp dir を作成できる");
        let project_path = dir.path().join("mm.toml");
        std::fs::write(&project_path, sample_project_toml("Preview"))
            .expect("project TOML を書ける");

        let output =
            render_preview_frame(project_path.display().to_string(), 0.25, "cpu".to_string())
                .expect("preview frame を生成できる");

        assert_eq!(PathBuf::from(&output), dir.path().join("cache/preview.png"));
        assert!(dir.path().join("cache/preview.png").exists());
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

    fn sample_video_project_toml() -> String {
        r#"[settings]
title = "Analysis"
width = 640
height = 360
fps = 30
sample_rate = 48000
duration = 1.0
output = "output/movie.mp4"
asset_mode = "copy"

[[assets]]
id = "video"
kind = "video"
path = "media/video/sample.mp4"

[[tracks]]
id = "v1"
name = "V1 Main Video"
kind = "video"

[[tracks.layers]]
id = "video-main"
start = 0.0
duration = 1.0
z_index = 0

[tracks.layers.content]
type = "video"
asset_id = "video"

[tracks.layers.transform]
x = 0.0
y = 0.0
width = 640.0
height = 360.0
scale = 1.0
rotation = 0.0
opacity = 1.0
"#
        .to_string()
    }
}
