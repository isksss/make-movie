use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub settings: ProjectSettings,
    #[serde(default)]
    pub assets: Vec<Asset>,
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub plugins: Vec<PluginDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub sample_rate: u32,
    pub duration: f64,
    pub output: PathBuf,
    #[serde(default = "default_asset_mode")]
    pub asset_mode: AssetMode,
    #[serde(default)]
    pub ffmpeg: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetMode {
    Copy,
    Link,
}

fn default_asset_mode() -> AssetMode {
    AssetMode::Copy
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub kind: AssetKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Video,
    Image,
    Audio,
    Subtitle,
    Font,
    Mask,
}

impl AssetKind {
    pub fn media_dir(self) -> &'static str {
        match self {
            AssetKind::Video => "video",
            AssetKind::Image => "image",
            AssetKind::Audio => "audio",
            AssetKind::Subtitle => "subtitle",
            AssetKind::Font => "font",
            AssetKind::Mask => "mask",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub start: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub kind: TrackKind,
    #[serde(default)]
    pub layers: Vec<Layer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub start: f64,
    pub duration: f64,
    pub z_index: i32,
    pub content: LayerContent,
    #[serde(default)]
    pub transform: Transform,
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub animations: Vec<Animation>,
    #[serde(default)]
    pub transition: Option<Transition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayerContent {
    Video(VideoLayer),
    Image(ImageLayer),
    Audio(AudioLayer),
    Text(TextLayer),
    Subtitle(SubtitleLayer),
    Voice(VoiceLayer),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoLayer {
    pub asset_id: String,
    #[serde(default)]
    pub crop: Option<Crop>,
    #[serde(default)]
    pub trim_start: Option<f64>,
    #[serde(default)]
    pub trim_end: Option<f64>,
    #[serde(default)]
    pub fit: Option<FitMode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageLayer {
    pub asset_id: String,
    #[serde(default)]
    pub crop: Option<Crop>,
    #[serde(default)]
    pub mask: Option<Mask>,
    #[serde(default)]
    pub fit: Option<FitMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioLayer {
    pub asset_id: String,
    #[serde(default)]
    pub trim_start: Option<Millis>,
    #[serde(default)]
    pub trim_end: Option<Millis>,
}

pub type Millis = u64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextLayer {
    pub text: String,
    #[serde(default)]
    pub font_asset_id: Option<String>,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub letter_spacing: f32,
    #[serde(default = "default_line_spacing")]
    pub line_spacing: f32,
    #[serde(default)]
    pub stroke: Option<TextStroke>,
    #[serde(default)]
    pub shadow: Option<TextShadow>,
    #[serde(default)]
    pub align: TextAlign,
}

fn default_font_size() -> f32 {
    48.0
}

fn default_color() -> String {
    "#ffffff".to_string()
}

fn default_line_spacing() -> f32 {
    1.2
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStroke {
    pub color: String,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextShadow {
    pub color: String,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleLayer {
    pub asset_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceLayer {
    pub provider: TtsProviderKind,
    pub speaker: String,
    pub text: String,
    #[serde(default = "default_tts_speed")]
    pub speed: f32,
    #[serde(default)]
    pub pitch: f32,
    #[serde(default)]
    pub emotion: Option<String>,
}

fn default_tts_speed() -> f32 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsProviderKind {
    Voicevox,
    AivisSpeech,
    CoeiroInk,
}

pub trait TtsProvider {
    fn synthesize(&self, request: SynthesisRequest) -> Result<AudioBuffer>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct SynthesisRequest {
    pub speaker: String,
    pub text: String,
    pub speed: f32,
    pub pitch: f32,
    pub emotion: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitMode {
    Contain,
    Cover,
    Stretch,
    BlurBackground,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Mask {
    Circle,
    RoundedRect { radius: f32 },
    Ellipse,
    Svg { path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    pub rotation: f32,
    pub opacity: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            scale: 1.0,
            rotation: 0.0,
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Animation {
    pub property: AnimatedProperty,
    pub easing: Easing,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimatedProperty {
    X,
    Y,
    Scale,
    Rotation,
    Opacity,
    Width,
    Height,
    CropX,
    CropY,
    CropWidth,
    CropHeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f64,
    pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseOutBack,
    Bounce,
    Elastic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Effect {
    FadeIn { duration: f64 },
    FadeOut { duration: f64 },
    Blur { radius: f32 },
    Zoom { amount: f32 },
    Slide { x: f32, y: f32 },
    Brightness { amount: f32 },
    Contrast { amount: f32 },
    Saturation { amount: f32 },
    Pixelate { size: u32 },
    MotionBlur { amount: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transition {
    CrossFade { duration: f64 },
    Wipe { duration: f64, shape: WipeShape },
    Push { duration: f64 },
    Zoom { duration: f64 },
    Blur { duration: f64 },
    Flash { duration: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WipeShape {
    Circle,
    RoundedRect {
        radius: f32,
        #[serde(default)]
        border: Option<WipeBorder>,
        #[serde(default)]
        shadow: Option<TextShadow>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WipeBorder {
    pub color: String,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginDeclaration {
    pub repository: PluginRepository,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRepository {
    Github,
    Gitlab,
    Url,
    Local,
}

pub fn load_project(path: impl AsRef<Path>) -> Result<Project> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .with_context(|| format!("プロジェクトを読み込めません: {}", path.display()))?;
    let project = toml::from_str(&text).context("mm.toml の parse に失敗しました")?;
    Ok(project)
}

pub fn save_project(project: &Project, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("保存先ディレクトリを作成できません: {}", parent.display()))?;
    }
    let text =
        toml::to_string_pretty(project).context("Project の TOML serialize に失敗しました")?;
    fs::write(path, text)
        .with_context(|| format!("プロジェクトを保存できません: {}", path.display()))?;
    Ok(())
}

pub fn validate_project(project: &Project, project_root: impl AsRef<Path>) -> Result<()> {
    if project.settings.title.trim().is_empty() {
        bail!("settings.title は必須です");
    }
    if project.settings.width == 0 || project.settings.height == 0 {
        bail!("settings.width と settings.height は 1 以上である必要があります");
    }
    if project.settings.fps == 0 {
        bail!("settings.fps は 1 以上である必要があります");
    }
    if project.settings.sample_rate == 0 {
        bail!("settings.sample_rate は 1 以上である必要があります");
    }
    if project.settings.duration <= 0.0 {
        bail!("settings.duration は 0 より大きい必要があります");
    }
    for scene in &project.scenes {
        validate_time_range(&scene.id, scene.start, scene.duration)?;
    }
    for track in &project.tracks {
        for layer in &track.layers {
            validate_time_range(&layer.id, layer.start, layer.duration)?;
            validate_layer_content(project, layer)?;
        }
    }
    let root = project_root.as_ref();
    for asset in &project.assets {
        let path = root.join(&asset.path);
        if !path.exists() {
            bail!(
                "asset '{}' のファイルが存在しません: {}",
                asset.id,
                path.display()
            );
        }
    }
    Ok(())
}

fn validate_time_range(id: &str, start: f64, duration: f64) -> Result<()> {
    if start < 0.0 {
        bail!("'{}' の start は 0 以上である必要があります", id);
    }
    if duration <= 0.0 {
        bail!("'{}' の duration は 0 より大きい必要があります", id);
    }
    Ok(())
}

fn validate_layer_content(project: &Project, layer: &Layer) -> Result<()> {
    let asset_id = match &layer.content {
        LayerContent::Video(content) => Some(content.asset_id.as_str()),
        LayerContent::Image(content) => Some(content.asset_id.as_str()),
        LayerContent::Audio(content) => Some(content.asset_id.as_str()),
        LayerContent::Subtitle(content) => Some(content.asset_id.as_str()),
        LayerContent::Text(_) | LayerContent::Voice(_) => None,
    };
    if let Some(asset_id) = asset_id {
        if !project.assets.iter().any(|asset| asset.id == asset_id) {
            bail!(
                "layer '{}' が未知の asset '{}' を参照しています",
                layer.id,
                asset_id
            );
        }
    }
    Ok(())
}

pub fn import_asset(
    project_root: impl AsRef<Path>,
    source_path: impl AsRef<Path>,
    kind: AssetKind,
) -> Result<Asset> {
    let project_root = project_root.as_ref();
    let source_path = source_path.as_ref();
    if !source_path.is_file() {
        bail!("import 元ファイルが存在しません: {}", source_path.display());
    }
    let file_name = source_path
        .file_name()
        .context("import 元ファイル名を取得できません")?;
    let media_relative = PathBuf::from("media")
        .join(kind.media_dir())
        .join(file_name);
    let destination = project_root.join(&media_relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("media ディレクトリを作成できません: {}", parent.display()))?;
    }
    fs::copy(source_path, &destination).with_context(|| {
        format!(
            "asset をコピーできません: {} -> {}",
            source_path.display(),
            destination.display()
        )
    })?;
    let id = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("asset")
        .to_string();
    Ok(Asset {
        id,
        kind,
        path: media_relative,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sample_project(asset_path: PathBuf) -> Project {
        Project {
            settings: ProjectSettings {
                title: "サンプル".to_string(),
                width: 1080,
                height: 1920,
                fps: 30,
                sample_rate: 48000,
                duration: 3.0,
                output: PathBuf::from("output/movie.mp4"),
                asset_mode: AssetMode::Copy,
                ffmpeg: None,
            },
            assets: vec![Asset {
                id: "image1".to_string(),
                kind: AssetKind::Image,
                path: asset_path,
            }],
            tracks: vec![Track {
                id: "v1".to_string(),
                name: "V1 Main Video".to_string(),
                kind: TrackKind::Video,
                layers: vec![Layer {
                    id: "layer1".to_string(),
                    start: 0.0,
                    duration: 3.0,
                    z_index: 0,
                    content: LayerContent::Text(TextLayer {
                        text: "Hello\nmake-movie".to_string(),
                        font_asset_id: None,
                        font_size: 64.0,
                        color: "#ffffff".to_string(),
                        letter_spacing: 0.0,
                        line_spacing: 1.2,
                        stroke: None,
                        shadow: None,
                        align: TextAlign::Center,
                    }),
                    transform: Transform::default(),
                    effects: vec![],
                    animations: vec![],
                    transition: None,
                }],
            }],
            scenes: vec![Scene {
                id: "scene1".to_string(),
                name: "Opening".to_string(),
                start: 0.0,
                duration: 3.0,
            }],
            plugins: vec![],
        }
    }

    #[test]
    fn project_round_trip_toml() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let asset_path = dir.path().join("media/image/sample.png");
        fs::create_dir_all(asset_path.parent().unwrap())?;
        fs::write(&asset_path, [])?;
        let project = sample_project(PathBuf::from("media/image/sample.png"));
        let project_path = dir.path().join("mm.toml");

        save_project(&project, &project_path)?;
        let loaded = load_project(&project_path)?;

        assert_eq!(loaded.settings.title, "サンプル");
        assert_eq!(loaded.assets[0].kind, AssetKind::Image);
        assert_eq!(loaded.tracks[0].layers.len(), 1);
        Ok(())
    }

    #[test]
    fn validate_rejects_missing_asset() {
        let dir = tempfile::tempdir().unwrap();
        let project = sample_project(PathBuf::from("media/image/missing.png"));

        let err = validate_project(&project, dir.path()).unwrap_err();

        assert!(err.to_string().contains("存在しません"));
    }

    #[test]
    fn validate_accepts_existing_asset_and_text_layer() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let asset_path = dir.path().join("media/image/sample.png");
        fs::create_dir_all(asset_path.parent().unwrap())?;
        fs::write(asset_path, [])?;
        let project = sample_project(PathBuf::from("media/image/sample.png"));

        validate_project(&project, dir.path())
    }

    #[test]
    fn import_asset_copies_to_media_directory() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let source_path = dir.path().join("source.png");
        let mut source = fs::File::create(&source_path)?;
        source.write_all(b"image")?;

        let asset = import_asset(dir.path(), &source_path, AssetKind::Image)?;

        assert_eq!(asset.id, "source");
        assert_eq!(asset.path, PathBuf::from("media/image/source.png"));
        assert!(dir.path().join(asset.path).exists());
        Ok(())
    }
}
