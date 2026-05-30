use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use mm_core::{load_project, validate_project};
use mm_plugin_runtime::{load_manifest, PluginManager, PluginReference};
use mm_render::{
    render_frame, render_project, FfmpegLocator, RenderBackend, RenderOptions, SystemFfmpegLocator,
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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
    Gpu,
}

impl From<CliRenderBackend> for RenderBackend {
    fn from(value: CliRenderBackend) -> Self {
        match value {
            CliRenderBackend::Auto => RenderBackend::Auto,
            CliRenderBackend::Cpu => RenderBackend::Cpu,
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
}

#[derive(Debug, Args)]
struct PluginRefArgs {
    name: String,
}

pub fn run() -> Result<()> {
    run_with(Cli::parse())
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

fn plugin(command: PluginCommand, messages: Messages) -> Result<()> {
    let manager = PluginManager::default();
    match command {
        PluginCommand::Install(args) => {
            if let Some(path) = args.manifest {
                manager.install_manifest(load_manifest(path)?)?;
            } else {
                let name = args
                    .name
                    .context(messages.plugin_install_requires_name_or_manifest)?;
                manager.install(PluginReference::named(name))?;
            }
        }
        PluginCommand::Update(args) => manager.update(PluginReference::named(args.name))?,
        PluginCommand::Remove(args) => manager.remove(PluginReference::named(args.name))?,
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Messages {
    build_done: &'static str,
    validate_done: &'static str,
    preview_done: &'static str,
    cleanup_done: &'static str,
    package_done: &'static str,
    not_found: &'static str,
    plugin_install_requires_name_or_manifest: &'static str,
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
                plugin_install_requires_name_or_manifest:
                    "plugin install には name または --manifest が必要です",
            },
            CliLanguage::En => Self {
                build_done: "build completed",
                validate_done: "validate completed",
                preview_done: "preview exported",
                cleanup_done: "cleanup completed",
                package_done: "package validation completed",
                not_found: "not found",
                plugin_install_requires_name_or_manifest:
                    "plugin install requires name or --manifest",
            },
        }
    }
}

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
        let cli = Cli::try_parse_from(["mm", "build", "--backend", "gpu"]).unwrap();
        match cli.command {
            Command::Build(args) => assert!(matches!(args.backend, CliRenderBackend::Gpu)),
            _ => panic!("build command として parse されていません"),
        }
    }

    #[test]
    fn cli_parses_language_option() {
        let cli = Cli::try_parse_from(["mm", "--lang", "en", "doctor"]).unwrap();

        assert!(matches!(resolve_language(cli.lang), CliLanguage::En));
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
    fn project_root_uses_parent_directory() {
        assert_eq!(
            project_root(Path::new("examples/basic/mm.toml")),
            PathBuf::from("examples/basic")
        );
    }
}
