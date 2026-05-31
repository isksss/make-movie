use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io::{Cursor, Read};
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
    pub groups: Vec<TimelineGroup>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineGroup {
    pub id: String,
    pub name: String,
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
    #[serde(default)]
    pub group_id: Option<String>,
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
    #[serde(default)]
    pub font_family: Option<String>,
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
    pub gradient: Option<TextGradient>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextGradient {
    pub start_color: String,
    pub end_color: String,
    pub direction: GradientDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GradientDirection {
    #[default]
    Vertical,
    Horizontal,
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

#[derive(Debug, Clone)]
pub struct VoicevoxProvider {
    endpoint: String,
    client: reqwest::blocking::Client,
}

impl VoicevoxProvider {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl Default for VoicevoxProvider {
    fn default() -> Self {
        Self::new(
            env::var("MM_VOICEVOX_ENDPOINT")
                .unwrap_or_else(|_| "http://127.0.0.1:50021".to_string()),
        )
    }
}

impl TtsProvider for VoicevoxProvider {
    fn synthesize(&self, request: SynthesisRequest) -> Result<AudioBuffer> {
        synthesize_voicevox_compatible(&self.client, &self.endpoint, request)
    }
}

#[derive(Debug, Clone)]
pub struct AivisProvider {
    endpoint: String,
    client: reqwest::blocking::Client,
}

impl AivisProvider {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl Default for AivisProvider {
    fn default() -> Self {
        Self::new(
            env::var("MM_AIVIS_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:10101".to_string()),
        )
    }
}

impl TtsProvider for AivisProvider {
    fn synthesize(&self, request: SynthesisRequest) -> Result<AudioBuffer> {
        synthesize_voicevox_compatible(&self.client, &self.endpoint, request)
    }
}

#[derive(Debug, Clone)]
pub struct CoeiroInkProvider {
    endpoint: String,
    client: reqwest::blocking::Client,
}

impl CoeiroInkProvider {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl Default for CoeiroInkProvider {
    fn default() -> Self {
        Self::new(
            env::var("MM_COEIROINK_ENDPOINT")
                .unwrap_or_else(|_| "http://127.0.0.1:50032".to_string()),
        )
    }
}

impl TtsProvider for CoeiroInkProvider {
    fn synthesize(&self, request: SynthesisRequest) -> Result<AudioBuffer> {
        let url = format!("{}/v1/synthesis", self.endpoint.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .json(&json!({
                "speaker": request.speaker,
                "text": request.text,
                "speedScale": request.speed,
                "pitchScale": request.pitch,
                "emotion": request.emotion,
            }))
            .send()
            .context("CoeiroInk synthesis request に失敗しました")?
            .error_for_status()
            .context("CoeiroInk synthesis が失敗しました")?
            .bytes()
            .context("CoeiroInk synthesis response を読み込めません")?;
        decode_wav_audio(&response)
    }
}

pub fn synthesize_with_default_provider(
    provider: TtsProviderKind,
    request: SynthesisRequest,
) -> Result<AudioBuffer> {
    match provider {
        TtsProviderKind::Voicevox => VoicevoxProvider::default().synthesize(request),
        TtsProviderKind::AivisSpeech => AivisProvider::default().synthesize(request),
        TtsProviderKind::CoeiroInk => CoeiroInkProvider::default().synthesize(request),
    }
}

fn synthesize_voicevox_compatible(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    request: SynthesisRequest,
) -> Result<AudioBuffer> {
    let endpoint = endpoint.trim_end_matches('/');
    let speaker = request.speaker.parse::<u32>().unwrap_or(1);
    let query_url = format!("{endpoint}/audio_query");
    let synthesis_url = format!("{endpoint}/synthesis");
    let mut query: Value = client
        .post(query_url)
        .query(&[
            ("text", request.text.clone()),
            ("speaker", speaker.to_string()),
        ])
        .send()
        .context("audio_query request に失敗しました")?
        .error_for_status()
        .context("audio_query が失敗しました")?
        .json()
        .context("audio_query response をJSONとして読み込めません")?;
    query["speedScale"] = json!(request.speed);
    query["pitchScale"] = json!(request.pitch);
    let response = client
        .post(synthesis_url)
        .query(&[("speaker", speaker.to_string())])
        .json(&query)
        .send()
        .context("synthesis request に失敗しました")?
        .error_for_status()
        .context("synthesis が失敗しました")?
        .bytes()
        .context("synthesis response を読み込めません")?;
    decode_wav_audio(&response)
}

pub fn decode_wav_audio(bytes: &[u8]) -> Result<AudioBuffer> {
    let mut cursor = Cursor::new(bytes);
    let mut header = [0u8; 12];
    cursor
        .read_exact(&mut header)
        .context("WAV header を読み込めません")?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        bail!("WAV RIFF/WAVE header ではありません");
    }

    let mut format: Option<(u16, u16, u32, u16)> = None;
    let mut data = Vec::new();
    while (cursor.position() as usize) + 8 <= bytes.len() {
        let mut chunk_header = [0u8; 8];
        cursor
            .read_exact(&mut chunk_header)
            .context("WAV chunk header を読み込めません")?;
        let chunk_id = &chunk_header[0..4];
        let chunk_size = u32::from_le_bytes(chunk_header[4..8].try_into().unwrap()) as usize;
        if (cursor.position() as usize) + chunk_size > bytes.len() {
            bail!("WAV chunk size が不正です");
        }
        let mut chunk = vec![0; chunk_size];
        cursor
            .read_exact(&mut chunk)
            .context("WAV chunk を読み込めません")?;
        if chunk_size % 2 == 1 && (cursor.position() as usize) < bytes.len() {
            let mut padding = [0u8; 1];
            cursor
                .read_exact(&mut padding)
                .context("WAV chunk padding を読み込めません")?;
        }
        match chunk_id {
            b"fmt " => {
                if chunk.len() < 16 {
                    bail!("WAV fmt chunk が短すぎます");
                }
                let audio_format = u16::from_le_bytes(chunk[0..2].try_into().unwrap());
                let channels = u16::from_le_bytes(chunk[2..4].try_into().unwrap());
                let sample_rate = u32::from_le_bytes(chunk[4..8].try_into().unwrap());
                let bits_per_sample = u16::from_le_bytes(chunk[14..16].try_into().unwrap());
                format = Some((audio_format, channels, sample_rate, bits_per_sample));
            }
            b"data" => data = chunk,
            _ => {}
        }
    }
    let (audio_format, channels, sample_rate, bits_per_sample) =
        format.context("WAV fmt chunk が見つかりません")?;
    if channels == 0 {
        bail!("WAV channels が 0 です");
    }
    let samples = decode_wav_samples(audio_format, bits_per_sample, &data)?;
    Ok(AudioBuffer {
        sample_rate,
        channels,
        samples,
    })
}

fn decode_wav_samples(audio_format: u16, bits_per_sample: u16, data: &[u8]) -> Result<Vec<f32>> {
    match (audio_format, bits_per_sample) {
        (1, 16) => Ok(data
            .chunks_exact(2)
            .map(|bytes| {
                let value = i16::from_le_bytes(bytes.try_into().unwrap());
                f32::from(value) / f32::from(i16::MAX)
            })
            .collect()),
        (1, 24) => Ok(data
            .chunks_exact(3)
            .map(|bytes| {
                let value = i32::from_le_bytes([
                    bytes[0],
                    bytes[1],
                    bytes[2],
                    if bytes[2] & 0x80 == 0 { 0 } else { 0xff },
                ]);
                value as f32 / 8_388_607.0
            })
            .collect()),
        (1, 32) => Ok(data
            .chunks_exact(4)
            .map(|bytes| {
                let value = i32::from_le_bytes(bytes.try_into().unwrap());
                value as f32 / i32::MAX as f32
            })
            .collect()),
        (3, 32) => Ok(data
            .chunks_exact(4)
            .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()).clamp(-1.0, 1.0))
            .collect()),
        _ => bail!("未対応のWAV形式です: format={audio_format}, bits={bits_per_sample}"),
    }
}

pub fn encode_wav_audio(buffer: &AudioBuffer) -> Vec<u8> {
    let bytes_per_sample = 2u16;
    let block_align = buffer.channels * bytes_per_sample;
    let byte_rate = buffer.sample_rate * u32::from(block_align);
    let data_size = buffer.samples.len() as u32 * u32::from(bytes_per_sample);
    let mut bytes = Vec::with_capacity(44 + data_size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&buffer.channels.to_le_bytes());
    bytes.extend_from_slice(&buffer.sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in &buffer.samples {
        let value = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub fn save_wav_audio(buffer: &AudioBuffer, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "WAV保存先ディレクトリを作成できません: {}",
                parent.display()
            )
        })?;
    }
    fs::write(path, encode_wav_audio(buffer))
        .with_context(|| format!("WAVを保存できません: {}", path.display()))
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
            validate_layer_group(project, layer)?;
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

fn validate_layer_group(project: &Project, layer: &Layer) -> Result<()> {
    if let Some(group_id) = &layer.group_id
        && !project.groups.iter().any(|group| group.id == *group_id)
    {
        bail!(
            "layer '{}' の group_id '{}' が groups に存在しません",
            layer.id,
            group_id
        );
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
    if let Some(asset_id) = asset_id
        && !project.assets.iter().any(|asset| asset.id == asset_id)
    {
        bail!(
            "layer '{}' が未知の asset '{}' を参照しています",
            layer.id,
            asset_id
        );
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

pub fn import_asset_into_project(
    project_path: impl AsRef<Path>,
    source_path: impl AsRef<Path>,
    kind: AssetKind,
) -> Result<Project> {
    let project_path = project_path.as_ref();
    let project_root = project_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let mut project = load_project(project_path)?;
    let mut asset = import_asset(&project_root, source_path, kind)?;
    asset.id = unique_asset_id(&project, &asset.id);
    let layer = imported_asset_layer(&project, &asset);
    project.assets.push(asset);
    if let Some(layer) = layer {
        let track_index = ensure_import_track(&mut project, kind);
        project.tracks[track_index].layers.push(layer);
    }
    save_project(&project, project_path)?;
    Ok(project)
}

fn unique_asset_id(project: &Project, base: &str) -> String {
    let normalized = if base.trim().is_empty() {
        "asset"
    } else {
        base
    };
    if !project.assets.iter().any(|asset| asset.id == normalized) {
        return normalized.to_string();
    }
    for index in 2.. {
        let candidate = format!("{normalized}-{index}");
        if !project.assets.iter().any(|asset| asset.id == candidate) {
            return candidate;
        }
    }
    unreachable!("usize の探索は必ず終了する")
}

fn unique_layer_id(project: &Project, base: &str) -> String {
    let normalized = if base.trim().is_empty() {
        "layer"
    } else {
        base
    };
    if !project
        .tracks
        .iter()
        .flat_map(|track| track.layers.iter())
        .any(|layer| layer.id == normalized)
    {
        return normalized.to_string();
    }
    for index in 2.. {
        let candidate = format!("{normalized}-{index}");
        if !project
            .tracks
            .iter()
            .flat_map(|track| track.layers.iter())
            .any(|layer| layer.id == candidate)
        {
            return candidate;
        }
    }
    unreachable!("usize の探索は必ず終了する")
}

fn ensure_import_track(project: &mut Project, kind: AssetKind) -> usize {
    let target_kind = match kind {
        AssetKind::Audio => TrackKind::Audio,
        AssetKind::Video
        | AssetKind::Image
        | AssetKind::Subtitle
        | AssetKind::Font
        | AssetKind::Mask => TrackKind::Video,
    };
    if let Some(index) = project
        .tracks
        .iter()
        .position(|track| track.kind == target_kind)
    {
        return index;
    }
    let (id, name) = match target_kind {
        TrackKind::Video => ("v1", "V1 Main Video"),
        TrackKind::Audio => ("a1", "A1 Voice"),
    };
    let track_id = unique_track_id(project, id);
    project.tracks.push(Track {
        id: track_id,
        name: name.to_string(),
        kind: target_kind,
        layers: vec![],
    });
    project.tracks.len() - 1
}

fn unique_track_id(project: &Project, base: &str) -> String {
    if !project.tracks.iter().any(|track| track.id == base) {
        return base.to_string();
    }
    for index in 2.. {
        let candidate = format!("{base}-{index}");
        if !project.tracks.iter().any(|track| track.id == candidate) {
            return candidate;
        }
    }
    unreachable!("usize の探索は必ず終了する")
}

fn imported_asset_layer(project: &Project, asset: &Asset) -> Option<Layer> {
    if matches!(asset.kind, AssetKind::Font | AssetKind::Mask) {
        return None;
    }
    let layer_id = unique_layer_id(project, &format!("{}-layer", asset.id));
    let duration = project.settings.duration.max(0.001);
    let z_index = project
        .tracks
        .iter()
        .flat_map(|track| track.layers.iter())
        .map(|layer| layer.z_index)
        .max()
        .unwrap_or(0)
        + 1;
    Some(Layer {
        id: layer_id,
        group_id: None,
        start: 0.0,
        duration,
        z_index,
        content: imported_layer_content(asset),
        transform: imported_asset_transform(project, asset.kind),
        effects: vec![],
        animations: vec![],
        transition: None,
    })
}

fn imported_layer_content(asset: &Asset) -> LayerContent {
    match asset.kind {
        AssetKind::Video => LayerContent::Video(VideoLayer {
            asset_id: asset.id.clone(),
            crop: None,
            trim_start: None,
            trim_end: None,
            fit: None,
        }),
        AssetKind::Image => LayerContent::Image(ImageLayer {
            asset_id: asset.id.clone(),
            crop: None,
            mask: None,
            fit: Some(FitMode::Contain),
        }),
        AssetKind::Audio => LayerContent::Audio(AudioLayer {
            asset_id: asset.id.clone(),
            trim_start: None,
            trim_end: None,
        }),
        AssetKind::Subtitle => LayerContent::Subtitle(SubtitleLayer {
            asset_id: asset.id.clone(),
        }),
        AssetKind::Font | AssetKind::Mask => {
            unreachable!("Font/Mask asset は timeline layer を生成しない")
        }
    }
}

fn imported_asset_transform(project: &Project, kind: AssetKind) -> Transform {
    match kind {
        AssetKind::Video | AssetKind::Image => Transform {
            x: 0.0,
            y: 0.0,
            width: project.settings.width as f32,
            height: project.settings.height as f32,
            scale: 1.0,
            rotation: 0.0,
            opacity: 1.0,
        },
        AssetKind::Subtitle => Transform {
            x: project.settings.width as f32 / 2.0,
            y: project.settings.height as f32 * 0.82,
            width: project.settings.width as f32 * 0.9,
            height: 80.0,
            scale: 1.0,
            rotation: 0.0,
            opacity: 1.0,
        },
        AssetKind::Audio | AssetKind::Font | AssetKind::Mask => Transform::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

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
                    group_id: Some("opening".to_string()),
                    start: 0.0,
                    duration: 3.0,
                    z_index: 0,
                    content: LayerContent::Text(TextLayer {
                        text: "Hello\nmake-movie".to_string(),
                        font_asset_id: None,
                        font_family: None,
                        font_size: 64.0,
                        color: "#ffffff".to_string(),
                        letter_spacing: 0.0,
                        line_spacing: 1.2,
                        stroke: None,
                        shadow: None,
                        gradient: Some(TextGradient {
                            start_color: "#ffcc00".to_string(),
                            end_color: "#ffffff".to_string(),
                            direction: GradientDirection::Vertical,
                        }),
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
            groups: vec![TimelineGroup {
                id: "opening".to_string(),
                name: "Opening Group".to_string(),
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
        assert_eq!(loaded.groups[0].name, "Opening Group");
        assert_eq!(
            loaded.tracks[0].layers[0].group_id.as_deref(),
            Some("opening")
        );
        let LayerContent::Text(text) = &loaded.tracks[0].layers[0].content else {
            panic!("text layer として読み込まれていません");
        };
        assert_eq!(
            text.gradient.as_ref().map(|gradient| gradient.direction),
            Some(GradientDirection::Vertical)
        );
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
    fn validate_rejects_missing_group() {
        let dir = tempfile::tempdir().unwrap();
        let mut project = sample_project(PathBuf::from("media/image/sample.png"));
        project.groups.clear();

        let err = validate_project(&project, dir.path()).unwrap_err();

        assert!(err.to_string().contains("group_id"));
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

    #[test]
    fn import_asset_into_project_registers_asset_and_layer() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let project_path = dir.path().join("mm.toml");
        save_project(
            &Project {
                settings: ProjectSettings {
                    title: "Import".to_string(),
                    width: 320,
                    height: 180,
                    fps: 30,
                    sample_rate: 48000,
                    duration: 2.0,
                    output: PathBuf::from("output/movie.mp4"),
                    asset_mode: AssetMode::Copy,
                    ffmpeg: None,
                },
                assets: vec![],
                tracks: vec![],
                scenes: vec![],
                groups: vec![],
                plugins: vec![],
            },
            &project_path,
        )?;
        let source_path = dir.path().join("photo.png");
        fs::write(&source_path, b"image")?;

        let project = import_asset_into_project(&project_path, &source_path, AssetKind::Image)?;

        assert_eq!(project.assets.len(), 1);
        assert_eq!(project.tracks.len(), 1);
        assert_eq!(project.tracks[0].layers.len(), 1);
        assert!(dir.path().join("media/image/photo.png").exists());
        let saved = load_project(&project_path)?;
        assert_eq!(saved.assets[0].id, "photo");
        assert!(matches!(
            saved.tracks[0].layers[0].content,
            LayerContent::Image(_)
        ));
        Ok(())
    }

    #[test]
    fn wav_audio_round_trip() -> Result<()> {
        let buffer = AudioBuffer {
            sample_rate: 24_000,
            channels: 1,
            samples: vec![-1.0, 0.0, 1.0],
        };

        let bytes = encode_wav_audio(&buffer);
        let decoded = decode_wav_audio(&bytes)?;

        assert_eq!(decoded.sample_rate, 24_000);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.samples.len(), 3);
        assert!(decoded.samples[0] < -0.99);
        assert!(decoded.samples[2] > 0.99);
        Ok(())
    }

    #[test]
    fn voicevox_provider_calls_audio_query_and_synthesis() -> Result<()> {
        let wav = encode_wav_audio(&AudioBuffer {
            sample_rate: 24_000,
            channels: 1,
            samples: vec![0.0, 0.25, -0.25],
        });
        let endpoint = start_voicevox_mock(wav);
        let provider = VoicevoxProvider::new(endpoint);

        let audio = provider.synthesize(SynthesisRequest {
            speaker: "1".to_string(),
            text: "こんにちは".to_string(),
            speed: 1.2,
            pitch: 0.1,
            emotion: Some("neutral".to_string()),
        })?;

        assert_eq!(audio.sample_rate, 24_000);
        assert_eq!(audio.channels, 1);
        assert_eq!(audio.samples.len(), 3);
        Ok(())
    }

    fn start_voicevox_mock(wav: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            for index in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 4096];
                let size = stream.read(&mut request).unwrap();
                let request_text = String::from_utf8_lossy(&request[..size]);
                if index == 0 {
                    assert!(request_text.starts_with("POST /audio_query"));
                    let body = r#"{"speedScale":1.0,"pitchScale":0.0}"#;
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .unwrap();
                } else {
                    assert!(request_text.starts_with("POST /synthesis"));
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        wav.len()
                    )
                    .unwrap();
                    stream.write_all(&wav).unwrap();
                }
            }
        });
        format!("http://{address}")
    }
}
