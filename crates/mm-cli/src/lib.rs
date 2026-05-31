use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use mm_core::{load_project, validate_project};
use mm_plugin_runtime::{
    default_global_config_path, default_plugin_dir, load_manifest, PluginManager, PluginReference,
};
use mm_render::{
    render_frame, render_project, FfmpegLocator, RenderBackend, RenderOptions, SystemFfmpegLocator,
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "mm")]
#[command(version)]
pub struct Cli {
    #[arg(long, value_enum)]
    lang: Option<CliLanguage>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum CliLanguage {
    Ja,
    En,
}

#[derive(Debug, Subcommand)]
enum Command {
    Build(ProjectArgs),
    Validate(ProjectArgs),
    Preview(PreviewArgs),
    Cleanup(ProjectRootArgs),
    Package(ProjectRootArgs),
    Doctor,
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
}

#[derive(Debug, Args)]
struct ProjectArgs {
    #[arg(long, default_value = "mm.toml")]
    project: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = CliRenderBackend::Auto)]
    backend: CliRenderBackend,
}

#[derive(Debug, Args)]
struct PreviewArgs {
    #[arg(long, default_value = "mm.toml")]
    project: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value_t = 0.0)]
    time: f64,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum CliRenderBackend {
    Auto,
    Cpu,
    Skia,
    Gpu,
}

impl From<CliRenderBackend> for RenderBackend {
    fn from(value: CliRenderBackend) -> Self {
        match value {
            CliRenderBackend::Auto => RenderBackend::Auto,
            CliRenderBackend::Cpu => RenderBackend::Cpu,
            CliRenderBackend::Skia => RenderBackend::Skia,
            CliRenderBackend::Gpu => RenderBackend::Gpu,
        }
    }
}

#[derive(Debug, Args)]
struct ProjectRootArgs {
    #[arg(long, default_value = ".")]
    project_root: PathBuf,
}

#[derive(Debug, Subcommand)]
enum PluginCommand {
    Install(PluginInstallArgs),
    Update(PluginRefArgs),
    Remove(PluginRefArgs),
}

#[derive(Debug, Args)]
struct PluginInstallArgs {
    name: Option<String>,
    #[arg(long)]
    manifest: Option<PathBuf>,
    #[arg(long, default_value = "mm.toml")]
    project: PathBuf,
    #[arg(long)]
    global_config: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct PluginRefArgs {
    name: String,
    #[arg(long, default_value = "mm.toml")]
    project: PathBuf,
}

pub fn run() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    if let Some(help) = localized_help(&args) {
        print!("{help}");
        return Ok(());
    }
    run_with(Cli::parse())
}

pub fn run_and_report() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let args = env::args().collect::<Vec<_>>();
            let language = resolve_language(language_from_args(&args));
            eprintln!("{}", localized_error(language, &error));
            ExitCode::FAILURE
        }
    }
}

pub fn run_with(cli: Cli) -> Result<()> {
    let messages = Messages::new(resolve_language(cli.lang));
    match cli.command {
        Command::Build(args) => build(args, messages),
        Command::Validate(args) => validate(args, messages),
        Command::Preview(args) => preview(args, messages),
        Command::Cleanup(args) => cleanup(args, messages),
        Command::Package(args) => package(args, messages),
        Command::Doctor => doctor(messages),
        Command::Plugin { command } => plugin(command, messages),
    }
}

fn localized_error(language: CliLanguage, error: &anyhow::Error) -> String {
    let mut chain = error.chain();
    let first = chain.next().expect("anyhow error chain is never empty");
    let mut lines = vec![format!(
        "{}: {}",
        error_label(language),
        translate_error_message(language, &first.to_string())
    )];
    lines.extend(chain.map(|cause| {
        format!(
            "{}: {}",
            cause_label(language),
            translate_error_message(language, &cause.to_string())
        )
    }));
    lines.join("\n")
}

fn error_label(language: CliLanguage) -> &'static str {
    match language {
        CliLanguage::Ja => "エラー",
        CliLanguage::En => "error",
    }
}

fn cause_label(language: CliLanguage) -> &'static str {
    match language {
        CliLanguage::Ja => "原因",
        CliLanguage::En => "caused by",
    }
}

fn translate_error_message(language: CliLanguage, message: &str) -> String {
    if matches!(language, CliLanguage::Ja) {
        return message.to_string();
    }

    [
        ("プロジェクトを読み込めません", "failed to read project"),
        ("mm.toml の parse に失敗しました", "failed to parse mm.toml"),
        ("settings.title は必須です", "settings.title is required"),
        (
            "settings.width と settings.height は 1 以上である必要があります",
            "settings.width and settings.height must be at least 1",
        ),
        (
            "settings.fps は 1 以上である必要があります",
            "settings.fps must be at least 1",
        ),
        (
            "settings.sample_rate は 1 以上である必要があります",
            "settings.sample_rate must be at least 1",
        ),
        (
            "settings.duration は 0 より大きい必要があります",
            "settings.duration must be greater than 0",
        ),
        (
            "preview 出力先ディレクトリを作成できません",
            "failed to create preview output directory",
        ),
        (
            "preview 画像を保存できません",
            "failed to save preview image",
        ),
        ("cache を削除できません", "failed to remove cache"),
        (
            "ffmpeg が見つかりません。MM_FFMPEG または PATH を確認してください",
            "ffmpeg was not found. Check MM_FFMPEG or PATH",
        ),
        (
            "ffprobe が見つかりません。PATH を確認してください",
            "ffprobe was not found. Check PATH",
        ),
    ]
    .into_iter()
    .fold(message.to_string(), |translated, (ja, en)| {
        translated.replace(ja, en)
    })
}

fn resolve_language(language: Option<CliLanguage>) -> CliLanguage {
    language
        .or_else(
            || match env::var("MM_LANG").ok()?.to_ascii_lowercase().as_str() {
                "en" | "english" => Some(CliLanguage::En),
                "ja" | "jp" | "japanese" => Some(CliLanguage::Ja),
                _ => None,
            },
        )
        .unwrap_or(CliLanguage::Ja)
}

fn localized_help(args: &[String]) -> Option<&'static str> {
    if !args
        .iter()
        .skip(1)
        .any(|arg| arg == "--help" || arg == "-h" || arg == "help")
    {
        return None;
    }

    let language = resolve_language(language_from_args(args));
    let path = help_command_path(args);
    Some(help_text(language, &path))
}

fn language_from_args(args: &[String]) -> Option<CliLanguage> {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--lang" {
            return iter.next().and_then(|value| parse_language_value(value));
        }
        if let Some(value) = arg.strip_prefix("--lang=") {
            return parse_language_value(value);
        }
    }
    None
}

fn parse_language_value(value: &str) -> Option<CliLanguage> {
    match value.to_ascii_lowercase().as_str() {
        "en" | "english" => Some(CliLanguage::En),
        "ja" | "jp" | "japanese" => Some(CliLanguage::Ja),
        _ => None,
    }
}

fn help_command_path(args: &[String]) -> Vec<&str> {
    let mut path = Vec::new();
    let mut skip_next = false;

    for arg in args.iter().skip(1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--lang" {
            skip_next = true;
            continue;
        }
        if arg.starts_with("--lang=") || arg == "--help" || arg == "-h" || arg == "help" {
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }

        match (path.as_slice(), arg.as_str()) {
            ([], "build" | "validate" | "preview" | "cleanup" | "package" | "doctor") => {
                path.push(arg.as_str());
                break;
            }
            ([], "plugin") => path.push(arg.as_str()),
            (["plugin"], "install" | "update" | "remove") => {
                path.push(arg.as_str());
                break;
            }
            _ => {}
        }
    }

    path
}

fn help_text(language: CliLanguage, path: &[&str]) -> &'static str {
    match (language, path) {
        (CliLanguage::Ja, ["build"]) => BUILD_HELP_JA,
        (CliLanguage::En, ["build"]) => BUILD_HELP_EN,
        (CliLanguage::Ja, ["validate"]) => VALIDATE_HELP_JA,
        (CliLanguage::En, ["validate"]) => VALIDATE_HELP_EN,
        (CliLanguage::Ja, ["preview"]) => PREVIEW_HELP_JA,
        (CliLanguage::En, ["preview"]) => PREVIEW_HELP_EN,
        (CliLanguage::Ja, ["cleanup"]) => CLEANUP_HELP_JA,
        (CliLanguage::En, ["cleanup"]) => CLEANUP_HELP_EN,
        (CliLanguage::Ja, ["package"]) => PACKAGE_HELP_JA,
        (CliLanguage::En, ["package"]) => PACKAGE_HELP_EN,
        (CliLanguage::Ja, ["doctor"]) => DOCTOR_HELP_JA,
        (CliLanguage::En, ["doctor"]) => DOCTOR_HELP_EN,
        (CliLanguage::Ja, ["plugin"]) => PLUGIN_HELP_JA,
        (CliLanguage::En, ["plugin"]) => PLUGIN_HELP_EN,
        (CliLanguage::Ja, ["plugin", "install"]) => PLUGIN_INSTALL_HELP_JA,
        (CliLanguage::En, ["plugin", "install"]) => PLUGIN_INSTALL_HELP_EN,
        (CliLanguage::Ja, ["plugin", "update"]) => PLUGIN_UPDATE_HELP_JA,
        (CliLanguage::En, ["plugin", "update"]) => PLUGIN_UPDATE_HELP_EN,
        (CliLanguage::Ja, ["plugin", "remove"]) => PLUGIN_REMOVE_HELP_JA,
        (CliLanguage::En, ["plugin", "remove"]) => PLUGIN_REMOVE_HELP_EN,
        (CliLanguage::Ja, _) => ROOT_HELP_JA,
        (CliLanguage::En, _) => ROOT_HELP_EN,
    }
}

fn build(args: ProjectArgs, messages: Messages) -> Result<()> {
    let project = load_project(&args.project)?;
    let project_root = project_root(&args.project);
    validate_project(&project, &project_root)?;
    let output = args
        .output
        .unwrap_or_else(|| project.settings.output.clone());
    let mut options = RenderOptions::new(project_root, output);
    options.backend = args.backend.into();
    render_project(&project, &options)?;
    println!("{}: {}", messages.build_done, options.output_path.display());
    Ok(())
}

fn validate(args: ProjectArgs, messages: Messages) -> Result<()> {
    let project = load_project(&args.project)?;
    let project_root = project_root(&args.project);
    validate_project(&project, project_root)?;
    println!("{}: {}", messages.validate_done, args.project.display());
    Ok(())
}

fn preview(args: PreviewArgs, messages: Messages) -> Result<()> {
    let project = load_project(&args.project)?;
    let project_root = project_root(&args.project);
    validate_project(&project, &project_root)?;
    let output = args
        .output
        .unwrap_or_else(|| project_root.join("cache/preview.png"));
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "preview 出力先ディレクトリを作成できません: {}",
                parent.display()
            )
        })?;
    }
    let options = RenderOptions::new(&project_root, &output);
    let frame = render_frame(
        &project,
        &options,
        args.time.clamp(0.0, project.settings.duration),
    )?;
    frame
        .save(&output)
        .with_context(|| format!("preview 画像を保存できません: {}", output.display()))?;
    println!("{}: {}", messages.preview_done, output.display());
    Ok(())
}

fn cleanup(args: ProjectRootArgs, messages: Messages) -> Result<()> {
    let cache = args.project_root.join("cache");
    if cache.exists() {
        fs::remove_dir_all(&cache)
            .with_context(|| format!("cache を削除できません: {}", cache.display()))?;
    }
    println!("{}: {}", messages.cleanup_done, cache.display());
    Ok(())
}

fn package(args: ProjectRootArgs, messages: Messages) -> Result<()> {
    let project = args.project_root.join("mm.toml");
    let loaded = load_project(&project)?;
    validate_project(&loaded, &args.project_root)?;
    println!("{}: {}", messages.package_done, args.project_root.display());
    Ok(())
}

fn doctor(messages: Messages) -> Result<()> {
    let locator = SystemFfmpegLocator;
    println!("mm doctor");
    match locator.ffmpeg_path(&empty_project()) {
        Ok(path) => println!("ffmpeg: {}", path.display()),
        Err(error) => println!("ffmpeg: {} ({error})", messages.not_found),
    }
    match locator.ffprobe_path() {
        Ok(path) => println!("ffprobe: {}", path.display()),
        Err(error) => println!("ffprobe: {} ({error})", messages.not_found),
    }
    Ok(())
}

fn plugin(command: PluginCommand, _messages: Messages) -> Result<()> {
    let manager = PluginManager::default();
    match command {
        PluginCommand::Install(args) => {
            if let Some(path) = args.manifest {
                manager.install_manifest(load_manifest(path)?)?;
            } else if let Some(name) = args.name {
                manager.install(PluginReference::named(name))?;
            } else {
                let project_manager = PluginManager::new(
                    default_plugin_dir(),
                    project_root(&args.project).join("mm.lock"),
                );
                let global_config = args
                    .global_config
                    .unwrap_or_else(default_global_config_path);
                project_manager.install_configured_plugins(global_config, args.project)?;
            }
        }
        PluginCommand::Update(args) => {
            project_plugin_manager(&args.project).update(PluginReference::named(args.name))?
        }
        PluginCommand::Remove(args) => {
            project_plugin_manager(&args.project).remove(PluginReference::named(args.name))?
        }
    }
    Ok(())
}

fn project_plugin_manager(project: &Path) -> PluginManager {
    PluginManager::new(default_plugin_dir(), project_root(project).join("mm.lock"))
}

#[derive(Clone, Copy)]
struct Messages {
    build_done: &'static str,
    validate_done: &'static str,
    preview_done: &'static str,
    cleanup_done: &'static str,
    package_done: &'static str,
    not_found: &'static str,
}

impl Messages {
    fn new(language: CliLanguage) -> Self {
        match language {
            CliLanguage::Ja => Self {
                build_done: "build が完了しました",
                validate_done: "validate が完了しました",
                preview_done: "preview を出力しました",
                cleanup_done: "cleanup が完了しました",
                package_done: "package の検証が完了しました",
                not_found: "未検出",
            },
            CliLanguage::En => Self {
                build_done: "build completed",
                validate_done: "validate completed",
                preview_done: "preview exported",
                cleanup_done: "cleanup completed",
                package_done: "package validation completed",
                not_found: "not found",
            },
        }
    }
}

const ROOT_HELP_JA: &str = r#"mm - 宣言的動画生成エンジン兼デスクトップ動画編集ソフト

使用方法: mm [オプション] <コマンド>

コマンド:
  build      mm.toml から動画を書き出す
  validate   mm.toml と参照ファイルを検証する
  preview    指定時刻のプレビュー画像を書き出す
  cleanup    cache ディレクトリを削除する
  package    配布前のプロジェクト検証を行う
  doctor     ffmpeg / ffprobe の検出状況を表示する
  plugin     plugin の install / update / remove を実行する
  help       このヘルプ、または指定コマンドのヘルプを表示する

オプション:
      --lang <LANG>  表示言語 [指定可能な値: ja, en]
  -h, --help         ヘルプを表示する
  -V, --version      バージョンを表示する
"#;

const ROOT_HELP_EN: &str = r#"mm - Declarative video generation engine and desktop video editor

Usage: mm [OPTIONS] <COMMAND>

Commands:
  build      Render a movie from mm.toml
  validate   Validate mm.toml and referenced files
  preview    Export a preview image at a given time
  cleanup    Remove the cache directory
  package    Validate a project before packaging
  doctor     Show ffmpeg / ffprobe detection status
  plugin     Install, update, or remove plugins
  help       Print this help or the help of a given command

Options:
      --lang <LANG>  Display language [possible values: ja, en]
  -h, --help         Print help
  -V, --version      Print version
"#;

const BUILD_HELP_JA: &str = r#"動画を書き出す

使用方法: mm build [オプション]

オプション:
      --project <PROJECT>  project toml のパス [既定値: mm.toml]
      --output <OUTPUT>    出力先ファイル
      --backend <BACKEND>  render backend [指定可能な値: auto, cpu, skia, gpu]
  -h, --help              ヘルプを表示する
"#;

const BUILD_HELP_EN: &str = r#"Render a movie

Usage: mm build [OPTIONS]

Options:
      --project <PROJECT>  Path to project toml [default: mm.toml]
      --output <OUTPUT>    Output file
      --backend <BACKEND>  Render backend [possible values: auto, cpu, skia, gpu]
  -h, --help              Print help
"#;

const VALIDATE_HELP_JA: &str = r#"プロジェクトを検証する

使用方法: mm validate [オプション]

オプション:
      --project <PROJECT>  project toml のパス [既定値: mm.toml]
      --output <OUTPUT>    互換用オプション
      --backend <BACKEND>  render backend [指定可能な値: auto, cpu, skia, gpu]
  -h, --help              ヘルプを表示する
"#;

const VALIDATE_HELP_EN: &str = r#"Validate a project

Usage: mm validate [OPTIONS]

Options:
      --project <PROJECT>  Path to project toml [default: mm.toml]
      --output <OUTPUT>    Compatibility option
      --backend <BACKEND>  Render backend [possible values: auto, cpu, skia, gpu]
  -h, --help              Print help
"#;

const PREVIEW_HELP_JA: &str = r#"プレビュー画像を書き出す

使用方法: mm preview [オプション]

オプション:
      --project <PROJECT>  project toml のパス [既定値: mm.toml]
      --output <OUTPUT>    出力先 PNG
      --time <TIME>        プレビュー時刻（秒） [既定値: 0]
  -h, --help              ヘルプを表示する
"#;

const PREVIEW_HELP_EN: &str = r#"Export a preview image

Usage: mm preview [OPTIONS]

Options:
      --project <PROJECT>  Path to project toml [default: mm.toml]
      --output <OUTPUT>    Output PNG
      --time <TIME>        Preview time in seconds [default: 0]
  -h, --help              Print help
"#;

const CLEANUP_HELP_JA: &str = r#"cache ディレクトリを削除する

使用方法: mm cleanup [オプション]

オプション:
      --project-root <PROJECT_ROOT>  project root [既定値: .]
  -h, --help                       ヘルプを表示する
"#;

const CLEANUP_HELP_EN: &str = r#"Remove the cache directory

Usage: mm cleanup [OPTIONS]

Options:
      --project-root <PROJECT_ROOT>  Project root [default: .]
  -h, --help                       Print help
"#;

const PACKAGE_HELP_JA: &str = r#"配布前のプロジェクト検証を行う

使用方法: mm package [オプション]

オプション:
      --project-root <PROJECT_ROOT>  project root [既定値: .]
  -h, --help                       ヘルプを表示する
"#;

const PACKAGE_HELP_EN: &str = r#"Validate a project before packaging

Usage: mm package [OPTIONS]

Options:
      --project-root <PROJECT_ROOT>  Project root [default: .]
  -h, --help                       Print help
"#;

const DOCTOR_HELP_JA: &str = r#"ffmpeg / ffprobe の検出状況を表示する

使用方法: mm doctor [オプション]

オプション:
  -h, --help  ヘルプを表示する
"#;

const DOCTOR_HELP_EN: &str = r#"Show ffmpeg / ffprobe detection status

Usage: mm doctor [OPTIONS]

Options:
  -h, --help  Print help
"#;

const PLUGIN_HELP_JA: &str = r#"plugin を管理する

使用方法: mm plugin <コマンド>

コマンド:
  install  plugin をインストールする
  update   plugin を更新する
  remove   plugin を削除する
  help     このヘルプ、または指定コマンドのヘルプを表示する

オプション:
  -h, --help  ヘルプを表示する
"#;

const PLUGIN_HELP_EN: &str = r#"Manage plugins

Usage: mm plugin <COMMAND>

Commands:
  install  Install a plugin
  update   Update a plugin
  remove   Remove a plugin
  help     Print this help or the help of a given command

Options:
  -h, --help  Print help
"#;

const PLUGIN_INSTALL_HELP_JA: &str = r#"plugin をインストールする

使用方法: mm plugin install [オプション] [NAME]

引数:
  [NAME]  plugin 名

オプション:
      --manifest <MANIFEST>  plugin manifest のパス
      --project <PROJECT>    project toml のパス [既定値: mm.toml]
      --global-config <GLOBAL_CONFIG>
                              global config のパス
  -h, --help                 ヘルプを表示する
"#;

const PLUGIN_INSTALL_HELP_EN: &str = r#"Install a plugin

Usage: mm plugin install [OPTIONS] [NAME]

Arguments:
  [NAME]  Plugin name

Options:
      --manifest <MANIFEST>  Path to plugin manifest
      --project <PROJECT>    Path to project toml [default: mm.toml]
      --global-config <GLOBAL_CONFIG>
                              Path to global config
  -h, --help                 Print help
"#;

const PLUGIN_UPDATE_HELP_JA: &str = r#"plugin を更新する

使用方法: mm plugin update <NAME>

引数:
  <NAME>  plugin 名

オプション:
      --project <PROJECT>  project toml のパス [既定値: mm.toml]
  -h, --help               ヘルプを表示する
"#;

const PLUGIN_UPDATE_HELP_EN: &str = r#"Update a plugin

Usage: mm plugin update <NAME>

Arguments:
  <NAME>  Plugin name

Options:
      --project <PROJECT>  Path to project toml [default: mm.toml]
  -h, --help               Print help
"#;

const PLUGIN_REMOVE_HELP_JA: &str = r#"plugin を削除する

使用方法: mm plugin remove <NAME>

引数:
  <NAME>  plugin 名

オプション:
      --project <PROJECT>  project toml のパス [既定値: mm.toml]
  -h, --help               ヘルプを表示する
"#;

const PLUGIN_REMOVE_HELP_EN: &str = r#"Remove a plugin

Usage: mm plugin remove <NAME>

Arguments:
  <NAME>  Plugin name

Options:
      --project <PROJECT>  Path to project toml [default: mm.toml]
  -h, --help               Print help
"#;

fn project_root(project_path: &Path) -> PathBuf {
    project_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn empty_project() -> mm_core::Project {
    mm_core::Project {
        settings: mm_core::ProjectSettings {
            title: "doctor".to_string(),
            width: 1,
            height: 1,
            fps: 1,
            sample_rate: 1,
            duration: 1.0,
            output: PathBuf::from("output.mp4"),
            asset_mode: mm_core::AssetMode::Copy,
            ffmpeg: None,
        },
        assets: vec![],
        tracks: vec![],
        scenes: vec![],
        groups: vec![],
        plugins: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_validate_command() {
        let cli = Cli::try_parse_from(["mm", "validate", "--project", "example/mm.toml"]).unwrap();
        match cli.command {
            Command::Validate(args) => assert_eq!(args.project, PathBuf::from("example/mm.toml")),
            _ => panic!("validate command として parse されていません"),
        }
    }

    #[test]
    fn cli_parses_build_backend() {
        let cli = Cli::try_parse_from(["mm", "build", "--backend", "skia"]).unwrap();
        match cli.command {
            Command::Build(args) => assert!(matches!(args.backend, CliRenderBackend::Skia)),
            _ => panic!("build command として parse されていません"),
        }
    }

    #[test]
    fn cli_parses_language_option() {
        let cli = Cli::try_parse_from(["mm", "--lang", "en", "doctor"]).unwrap();

        assert!(matches!(resolve_language(cli.lang), CliLanguage::En));
    }

    #[test]
    fn localized_help_uses_language_option() {
        let help =
            localized_help(&["mm".into(), "--lang".into(), "ja".into(), "--help".into()]).unwrap();
        assert!(help.contains("使用方法"));

        let help = localized_help(&[
            "mm".into(),
            "--lang=en".into(),
            "plugin".into(),
            "--help".into(),
        ])
        .unwrap();
        assert!(help.contains("Manage plugins"));
    }

    #[test]
    fn localized_help_detects_plugin_subcommand() {
        let help = localized_help(&[
            "mm".into(),
            "--lang".into(),
            "ja".into(),
            "plugin".into(),
            "install".into(),
            "--help".into(),
        ])
        .unwrap();

        assert!(help.contains("使用方法: mm plugin install"));
        assert!(help.contains("--manifest"));

        let help = localized_help(&[
            "mm".into(),
            "--lang".into(),
            "en".into(),
            "plugin".into(),
            "update".into(),
            "--help".into(),
        ])
        .unwrap();
        assert!(help.contains("Usage: mm plugin update"));
        assert!(help.contains("--project"));
    }

    #[test]
    fn cli_parses_preview_time_and_output() {
        let cli = Cli::try_parse_from([
            "mm",
            "preview",
            "--project",
            "example/mm.toml",
            "--time",
            "0.5",
            "--output",
            "preview.png",
        ])
        .unwrap();
        match cli.command {
            Command::Preview(args) => {
                assert_eq!(args.project, PathBuf::from("example/mm.toml"));
                assert_eq!(args.time, 0.5);
                assert_eq!(args.output, Some(PathBuf::from("preview.png")));
            }
            _ => panic!("preview command として parse されていません"),
        }
    }

    #[test]
    fn cli_parses_plugin_install_manifest() {
        let cli =
            Cli::try_parse_from(["mm", "plugin", "install", "--manifest", "plugin.toml"]).unwrap();
        match cli.command {
            Command::Plugin {
                command: PluginCommand::Install(args),
            } => assert_eq!(args.manifest, Some(PathBuf::from("plugin.toml"))),
            _ => panic!("plugin install command として parse されていません"),
        }
    }

    #[test]
    fn cli_parses_plugin_update_project() {
        let cli = Cli::try_parse_from([
            "mm",
            "plugin",
            "update",
            "theme",
            "--project",
            "examples/basic/mm.toml",
        ])
        .unwrap();
        match cli.command {
            Command::Plugin {
                command: PluginCommand::Update(args),
            } => {
                assert_eq!(args.name, "theme");
                assert_eq!(args.project, PathBuf::from("examples/basic/mm.toml"));
            }
            _ => panic!("plugin update command として parse されていません"),
        }
    }

    #[test]
    fn project_root_uses_parent_directory() {
        assert_eq!(
            project_root(Path::new("examples/basic/mm.toml")),
            PathBuf::from("examples/basic")
        );
    }
}
