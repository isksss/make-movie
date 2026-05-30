use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use mm_core::{load_project, validate_project};
use mm_plugin_runtime::{load_manifest, PluginManager, PluginReference};
use mm_render::{render_project, FfmpegLocator, RenderBackend, RenderOptions, SystemFfmpegLocator};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "mm")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Build(ProjectArgs),
    Validate(ProjectArgs),
    Preview(ProjectArgs),
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
    match cli.command {
        Command::Build(args) => build(args),
        Command::Validate(args) => validate(args),
        Command::Preview(args) => preview(args),
        Command::Cleanup(args) => cleanup(args),
        Command::Package(args) => package(args),
        Command::Doctor => doctor(),
        Command::Plugin { command } => plugin(command),
    }
}

fn build(args: ProjectArgs) -> Result<()> {
    let project = load_project(&args.project)?;
    let project_root = project_root(&args.project);
    validate_project(&project, &project_root)?;
    let output = args
        .output
        .unwrap_or_else(|| project.settings.output.clone());
    let mut options = RenderOptions::new(project_root, output);
    options.backend = args.backend.into();
    render_project(&project, &options)?;
    println!("build が完了しました: {}", options.output_path.display());
    Ok(())
}

fn validate(args: ProjectArgs) -> Result<()> {
    let project = load_project(&args.project)?;
    let project_root = project_root(&args.project);
    validate_project(&project, project_root)?;
    println!("validate が完了しました: {}", args.project.display());
    Ok(())
}

fn preview(args: ProjectArgs) -> Result<()> {
    let project = load_project(&args.project)?;
    validate_project(&project, project_root(&args.project))?;
    println!("preview は利用可能です: {}", args.project.display());
    Ok(())
}

fn cleanup(args: ProjectRootArgs) -> Result<()> {
    let cache = args.project_root.join("cache");
    if cache.exists() {
        fs::remove_dir_all(&cache)
            .with_context(|| format!("cache を削除できません: {}", cache.display()))?;
    }
    println!("cleanup が完了しました: {}", cache.display());
    Ok(())
}

fn package(args: ProjectRootArgs) -> Result<()> {
    let project = args.project_root.join("mm.toml");
    let loaded = load_project(&project)?;
    validate_project(&loaded, &args.project_root)?;
    println!(
        "package の検証が完了しました: {}",
        args.project_root.display()
    );
    Ok(())
}

fn doctor() -> Result<()> {
    let locator = SystemFfmpegLocator;
    println!("mm doctor");
    match locator.ffmpeg_path(&empty_project()) {
        Ok(path) => println!("ffmpeg: {}", path.display()),
        Err(error) => println!("ffmpeg: 未検出 ({error})"),
    }
    match locator.ffprobe_path() {
        Ok(path) => println!("ffprobe: {}", path.display()),
        Err(error) => println!("ffprobe: 未検出 ({error})"),
    }
    Ok(())
}

fn plugin(command: PluginCommand) -> Result<()> {
    let manager = PluginManager::default();
    match command {
        PluginCommand::Install(args) => {
            if let Some(path) = args.manifest {
                manager.install_manifest(load_manifest(path)?)?;
            } else {
                let name = args
                    .name
                    .context("plugin install には name または --manifest が必要です")?;
                manager.install(PluginReference::named(name))?;
            }
        }
        PluginCommand::Update(args) => manager.update(PluginReference::named(args.name))?,
        PluginCommand::Remove(args) => manager.remove(PluginReference::named(args.name))?,
    }
    Ok(())
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
