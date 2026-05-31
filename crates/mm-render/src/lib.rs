use ab_glyph::{point, Font, FontArc, GlyphId, PxScale, ScaleFont};
use anyhow::{bail, Context, Result};
use image::{imageops, Rgba, RgbaImage};
use mm_core::{
    save_wav_audio, synthesize_with_default_provider, AnimatedProperty, Animation, AssetKind,
    AudioLayer, Crop, Easing, Effect, FitMode, GradientDirection, Layer, LayerContent, Mask,
    Project, SubtitleLayer, SynthesisRequest, TextAlign, TextLayer, TextShadow, TextStroke,
    Transform, Transition, VideoLayer, VoiceLayer, WipeShape,
};
use skia_safe::{image::CachingHint, surfaces, AlphaType, Color, ColorType, ImageInfo};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;

const GPU_COPY_ALIGNMENT: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub project_root: PathBuf,
    pub output_path: PathBuf,
    pub ffmpeg_path: Option<PathBuf>,
    pub background: Rgba<u8>,
    pub backend: RenderBackend,
}

impl RenderOptions {
    pub fn new(project_root: impl Into<PathBuf>, output_path: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            output_path: output_path.into(),
            ffmpeg_path: None,
            background: Rgba([20, 20, 24, 255]),
            backend: RenderBackend::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackend {
    Auto,
    Cpu,
    Skia,
    Gpu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuProbe {
    pub available: bool,
    pub adapter_name: Option<String>,
}

pub trait FfmpegLocator {
    fn ffmpeg_path(&self, project: &Project) -> Result<PathBuf>;
    fn ffprobe_path(&self) -> Result<PathBuf>;
}

#[derive(Debug, Clone, Default)]
pub struct SystemFfmpegLocator;

impl FfmpegLocator for SystemFfmpegLocator {
    fn ffmpeg_path(&self, project: &Project) -> Result<PathBuf> {
        if let Some(path) = &project.settings.ffmpeg {
            return Ok(path.clone());
        }
        if let Ok(path) = env::var("MM_FFMPEG") {
            return Ok(PathBuf::from(path));
        }
        which::which("ffmpeg")
            .context("ffmpeg が見つかりません。MM_FFMPEG または PATH を確認してください")
    }

    fn ffprobe_path(&self) -> Result<PathBuf> {
        which::which("ffprobe").context("ffprobe が見つかりません。PATH を確認してください")
    }
}

#[derive(Debug, Clone)]
pub struct BundledFfmpegLocator {
    bundle_dir: PathBuf,
}

impl BundledFfmpegLocator {
    pub fn new(bundle_dir: impl Into<PathBuf>) -> Self {
        Self {
            bundle_dir: bundle_dir.into(),
        }
    }

    fn binary_path(&self, name: &str) -> Result<PathBuf> {
        bundled_binary_path(&self.bundle_dir, name).with_context(|| {
            format!(
                "同梱 {name} が見つかりません: {}",
                self.bundle_dir.display()
            )
        })
    }
}

impl FfmpegLocator for BundledFfmpegLocator {
    fn ffmpeg_path(&self, _project: &Project) -> Result<PathBuf> {
        self.binary_path("ffmpeg")
    }

    fn ffprobe_path(&self) -> Result<PathBuf> {
        self.binary_path("ffprobe")
    }
}

fn bundled_binary_path(bundle_dir: &Path, name: &str) -> Option<PathBuf> {
    executable_names(name)
        .into_iter()
        .flat_map(|file_name| {
            [
                bundle_dir.join(&file_name),
                bundle_dir.join("bin").join(&file_name),
            ]
        })
        .find(|path| path.is_file())
}

fn executable_names(name: &str) -> Vec<String> {
    let mut names = vec![name.to_string()];
    let exe_name = format!("{name}.exe");
    if !names.iter().any(|candidate| candidate == &exe_name) {
        names.push(exe_name);
    }
    names
}

pub fn render_project(project: &Project, options: &RenderOptions) -> Result<()> {
    let active_backend = match options.backend {
        RenderBackend::Cpu => ActiveRenderBackend::Cpu,
        RenderBackend::Skia => ActiveRenderBackend::Skia(SkiaFrameRenderer::new()?),
        RenderBackend::Auto => match GpuFrameRenderer::new() {
            Ok(renderer) => ActiveRenderBackend::GpuHybrid(renderer),
            Err(_) => ActiveRenderBackend::Skia(SkiaFrameRenderer::new()?),
        },
        RenderBackend::Gpu => ActiveRenderBackend::GpuHybrid(GpuFrameRenderer::new()?),
    };
    let ffmpeg = match &options.ffmpeg_path {
        Some(path) => path.clone(),
        None => SystemFfmpegLocator.ffmpeg_path(project)?,
    };
    encode_with_ffmpeg(project, options, &ffmpeg, active_backend)
}

pub fn probe_gpu_backend() -> GpuProbe {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let Ok(adapter) = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
        else {
            return GpuProbe {
                available: false,
                adapter_name: None,
            };
        };
        let info = adapter.get_info();
        GpuProbe {
            available: true,
            adapter_name: Some(info.name),
        }
    })
}

enum ActiveRenderBackend {
    Cpu,
    Skia(SkiaFrameRenderer),
    GpuHybrid(GpuFrameRenderer),
}

fn encode_with_ffmpeg(
    project: &Project,
    options: &RenderOptions,
    ffmpeg: &Path,
    backend: ActiveRenderBackend,
) -> Result<()> {
    let width = project.settings.width;
    let height = project.settings.height;
    let fps = project.settings.fps;
    let frame_count = (project.settings.duration * f64::from(fps)).ceil().max(1.0) as u64;

    if let Some(parent) = options.output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("出力先ディレクトリを作成できません: {}", parent.display()))?;
    }

    let media_plan = MediaPlan::new(project, options)?;
    let mut ffmpeg_args = base_ffmpeg_args(width, height, fps);
    ffmpeg_args.extend(media_plan.input_args());
    ffmpeg_args.extend(media_plan.output_args(project, options));
    ffmpeg_args.push(options.output_path.display().to_string());

    let mut child = Command::new(ffmpeg)
        .args(&ffmpeg_args)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("ffmpeg を起動できません: {}", ffmpeg.display()))?;

    {
        let stdin = child.stdin.as_mut().context("ffmpeg stdin を開けません")?;
        for frame_index in 0..frame_count {
            let time = frame_index as f64 / f64::from(fps);
            let frame = render_frame_for_encode(project, options, time, &backend, media_plan.mode)?;
            stdin
                .write_all(frame.as_raw())
                .context("ffmpeg へ frame を書き込めません")?;
        }
    }

    let output = child
        .wait_with_output()
        .context("ffmpeg の終了待機に失敗しました")?;
    if !output.status.success() {
        bail!(
            "ffmpeg encode に失敗しました: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn base_ffmpeg_args(width: u32, height: u32, fps: u32) -> Vec<String> {
    vec![
        "-y".to_string(),
        "-f".to_string(),
        "rawvideo".to_string(),
        "-pix_fmt".to_string(),
        "rgba".to_string(),
        "-s".to_string(),
        format!("{width}x{height}"),
        "-r".to_string(),
        fps.to_string(),
        "-i".to_string(),
        "-".to_string(),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RawFrameMode {
    FullFrame,
    TransparentOverlay,
}

#[derive(Debug, Clone)]
struct MediaPlan {
    mode: RawFrameMode,
    video_layers: Vec<PlannedVideoLayer>,
    audio_layers: Vec<PlannedAudioLayer>,
}

impl MediaPlan {
    fn new(project: &Project, options: &RenderOptions) -> Result<Self> {
        let mut video_layers = Vec::new();
        let mut audio_layers = Vec::new();
        for layer in project
            .tracks
            .iter()
            .flat_map(|track| track.layers.iter())
            .filter(|layer| layer.duration > 0.0)
        {
            match &layer.content {
                LayerContent::Video(content) => {
                    let path = asset_path(project, options, &content.asset_id, AssetKind::Video)?;
                    video_layers.push(PlannedVideoLayer::new(project, layer, content, path));
                }
                LayerContent::Audio(content) => {
                    let path = asset_path(project, options, &content.asset_id, AssetKind::Audio)?;
                    audio_layers.push(PlannedAudioLayer::new(layer, content, path));
                }
                LayerContent::Voice(content) => {
                    let path = synthesize_voice_layer(options, layer, content)?;
                    audio_layers.push(PlannedAudioLayer::from_voice(layer, path));
                }
                LayerContent::Image(_) | LayerContent::Text(_) | LayerContent::Subtitle(_) => {}
            }
        }
        video_layers.sort_by_key(|layer| layer.z_index);
        audio_layers.sort_by(|left, right| left.start.total_cmp(&right.start));
        let mode = if video_layers.is_empty() {
            RawFrameMode::FullFrame
        } else {
            RawFrameMode::TransparentOverlay
        };
        Ok(Self {
            mode,
            video_layers,
            audio_layers,
        })
    }

    fn input_args(&self) -> Vec<String> {
        self.video_layers
            .iter()
            .map(|layer| &layer.path)
            .chain(self.audio_layers.iter().map(|layer| &layer.path))
            .flat_map(|path| ["-i".to_string(), path.display().to_string()])
            .collect()
    }

    fn output_args(&self, project: &Project, options: &RenderOptions) -> Vec<String> {
        let mut args = Vec::new();
        let filter_complex = self.filter_complex(project, options.background);
        if let Some(filter_complex) = filter_complex {
            args.extend(["-filter_complex".to_string(), filter_complex]);
        }
        args.extend(["-map".to_string(), "[vout]".to_string()]);
        if self.audio_layers.is_empty() {
            args.push("-an".to_string());
        } else {
            args.extend(["-map".to_string(), "[aout]".to_string()]);
        }
        args.extend([
            "-c:v".to_string(),
            "libx264".to_string(),
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
        ]);
        if !self.audio_layers.is_empty() {
            args.extend([
                "-c:a".to_string(),
                "aac".to_string(),
                "-ar".to_string(),
                project.settings.sample_rate.to_string(),
                "-ac".to_string(),
                "2".to_string(),
            ]);
        }
        args.extend([
            "-shortest".to_string(),
            "-t".to_string(),
            project.settings.duration.to_string(),
        ]);
        args
    }

    fn filter_complex(&self, project: &Project, background: Rgba<u8>) -> Option<String> {
        let mut filters = Vec::new();
        let mut video_cursor = 1usize;
        if self.video_layers.is_empty() {
            filters.push("[0:v]format=rgba[vout]".to_string());
        } else {
            filters.push(format!(
                "color=c=0x{:02x}{:02x}{:02x}:s={}x{}:r={}:d={}[base0]",
                background[0],
                background[1],
                background[2],
                project.settings.width,
                project.settings.height,
                project.settings.fps,
                project.settings.duration
            ));
            let mut base_label = "base0".to_string();
            for (layer_index, layer) in self.video_layers.iter().enumerate() {
                let input_index = video_cursor;
                video_cursor += 1;
                let video_label = format!("video{layer_index}");
                let next_base_label = format!("base{}", layer_index + 1);
                filters.push(layer.video_filter(input_index, &video_label));
                filters.push(format!(
                    "[{base_label}][{video_label}]overlay=x={}:y={}:eof_action=pass[{next_base_label}]",
                    layer.x, layer.y
                ));
                base_label = next_base_label;
            }
            filters.push("[0:v]format=rgba[overlay]".to_string());
            filters.push(format!(
                "[{base_label}][overlay]overlay=x=0:y=0:format=auto:eof_action=pass[vout]"
            ));
        }

        let audio_input_offset = 1 + self.video_layers.len();
        let audio_labels = self
            .audio_layers
            .iter()
            .enumerate()
            .map(|(index, layer)| {
                let input_index = audio_input_offset + index;
                let label = format!("audio{index}");
                filters.push(layer.audio_filter(input_index, &label));
                format!("[{label}]")
            })
            .collect::<Vec<_>>();
        if !audio_labels.is_empty() {
            filters.push(format!(
                "{}amix=inputs={}:duration=longest:normalize=0,atrim=0:{}[aout]",
                audio_labels.join(""),
                audio_labels.len(),
                project.settings.duration
            ));
        }

        Some(filters.join(";"))
    }
}

#[derive(Debug, Clone)]
struct PlannedVideoLayer {
    path: PathBuf,
    start: f64,
    duration: f64,
    z_index: i32,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    opacity: f32,
    trim_start: f64,
    trim_end: Option<f64>,
    crop: Option<Crop>,
}

impl PlannedVideoLayer {
    fn new(project: &Project, layer: &Layer, content: &VideoLayer, path: PathBuf) -> Self {
        let width = if layer.transform.width > 0.0 {
            layer.transform.width.round() as u32
        } else {
            project.settings.width
        };
        let height = if layer.transform.height > 0.0 {
            layer.transform.height.round() as u32
        } else {
            project.settings.height
        };
        Self {
            path,
            start: layer.start,
            duration: layer.duration,
            z_index: layer.z_index,
            x: layer.transform.x.round() as i32,
            y: layer.transform.y.round() as i32,
            width,
            height,
            opacity: layer.transform.opacity.clamp(0.0, 1.0),
            trim_start: content.trim_start.unwrap_or(0.0).max(0.0),
            trim_end: content.trim_end,
            crop: content.crop,
        }
    }

    fn video_filter(&self, input_index: usize, label: &str) -> String {
        let trim_end = self
            .trim_end
            .unwrap_or(self.trim_start + self.duration)
            .max(self.trim_start);
        let crop_filter = self
            .crop
            .map(|crop| format!("crop={}:{}:{}:{},", crop.width, crop.height, crop.x, crop.y))
            .unwrap_or_default();
        format!(
            "[{input_index}:v]trim=start={}:end={},setpts=PTS-STARTPTS+{}/TB,{}scale={}:{},format=rgba,colorchannelmixer=aa={}[{label}]",
            self.trim_start,
            trim_end,
            self.start,
            crop_filter,
            self.width,
            self.height,
            self.opacity
        )
    }
}

#[derive(Debug, Clone)]
struct PlannedAudioLayer {
    path: PathBuf,
    start: f64,
    duration: f64,
    trim_start: f64,
    trim_end: Option<f64>,
}

impl PlannedAudioLayer {
    fn new(layer: &Layer, content: &AudioLayer, path: PathBuf) -> Self {
        Self {
            path,
            start: layer.start,
            duration: layer.duration,
            trim_start: content.trim_start.unwrap_or(0) as f64 / 1000.0,
            trim_end: content.trim_end.map(|value| value as f64 / 1000.0),
        }
    }

    fn from_voice(layer: &Layer, path: PathBuf) -> Self {
        Self {
            path,
            start: layer.start,
            duration: layer.duration,
            trim_start: 0.0,
            trim_end: Some(layer.duration),
        }
    }

    fn audio_filter(&self, input_index: usize, label: &str) -> String {
        let trim_end = self
            .trim_end
            .unwrap_or(self.trim_start + self.duration)
            .max(self.trim_start);
        let delay_ms = (self.start.max(0.0) * 1000.0).round() as u64;
        format!(
            "[{input_index}:a]atrim=start={}:end={},asetpts=PTS-STARTPTS,adelay={delay_ms}:all=1[{label}]",
            self.trim_start, trim_end
        )
    }
}

fn synthesize_voice_layer(
    options: &RenderOptions,
    layer: &Layer,
    content: &VoiceLayer,
) -> Result<PathBuf> {
    let path = voice_cache_path(options, layer, content);
    if path.exists() {
        return Ok(path);
    }

    let buffer = synthesize_with_default_provider(
        content.provider,
        SynthesisRequest {
            speaker: content.speaker.clone(),
            text: content.text.clone(),
            speed: content.speed,
            pitch: content.pitch,
            emotion: content.emotion.clone(),
        },
    )
    .with_context(|| format!("Voice layer の音声合成に失敗しました: {}", layer.id))?;
    save_wav_audio(&buffer, &path)?;
    Ok(path)
}

fn voice_cache_path(options: &RenderOptions, layer: &Layer, content: &VoiceLayer) -> PathBuf {
    let key = format!(
        "{:?}\0{}\0{}\0{}\0{}\0{:?}\0{}\0{}",
        content.provider,
        content.speaker,
        content.text,
        content.speed,
        content.pitch,
        content.emotion,
        layer.start,
        layer.duration
    );
    options
        .project_root
        .join("cache/tts")
        .join(format!("voice-{:016x}.wav", stable_hash(key.as_bytes())))
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn asset_path(
    project: &Project,
    options: &RenderOptions,
    asset_id: &str,
    kind: AssetKind,
) -> Result<PathBuf> {
    let asset = project
        .assets
        .iter()
        .find(|asset| asset.id == asset_id && asset.kind == kind)
        .with_context(|| format!("{kind:?} asset が見つかりません: {asset_id}"))?;
    Ok(options.project_root.join(&asset.path))
}

fn preview_ffmpeg_path(project: &Project, options: &RenderOptions) -> Result<PathBuf> {
    match &options.ffmpeg_path {
        Some(path) => Ok(path.clone()),
        None => SystemFfmpegLocator.ffmpeg_path(project),
    }
}

fn decode_video_layer_frame(
    project: &Project,
    options: &RenderOptions,
    layer: &Layer,
    content: &VideoLayer,
    local_time: f64,
) -> Result<RgbaImage> {
    let trim_start = content.trim_start.unwrap_or(0.0).max(0.0);
    let source_time = trim_start + local_time.max(0.0);
    if let Some(trim_end) = content.trim_end {
        if source_time >= trim_end {
            return Ok(RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0])));
        }
    }

    let path = asset_path(project, options, &content.asset_id, AssetKind::Video)?;
    let ffmpeg = preview_ffmpeg_path(project, options)?;
    let source_time = source_time.to_string();
    let input_path = path.display().to_string();
    let output = Command::new(&ffmpeg)
        .args([
            "-v",
            "error",
            "-ss",
            &source_time,
            "-i",
            &input_path,
            "-frames:v",
            "1",
            "-f",
            "image2pipe",
            "-vcodec",
            "png",
            "-",
        ])
        .output()
        .with_context(|| {
            format!(
                "Video layer の frame decode を開始できません: layer={}, ffmpeg={}, asset={}",
                layer.id,
                ffmpeg.display(),
                path.display()
            )
        })?;

    if !output.status.success() {
        bail!(
            "Video layer の frame decode に失敗しました: layer={}, asset={}, stderr={}",
            layer.id,
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    if output.stdout.is_empty() {
        bail!(
            "Video layer の frame decode 結果が空です: layer={}, asset={}, time={}",
            layer.id,
            path.display(),
            source_time
        );
    }

    image::load_from_memory(&output.stdout)
        .with_context(|| {
            format!(
                "Video layer の decoded frame を読み込めません: layer={}, asset={}",
                layer.id,
                path.display()
            )
        })
        .map(|image| image.to_rgba8())
}

fn render_frame_for_encode(
    project: &Project,
    options: &RenderOptions,
    time: f64,
    backend: &ActiveRenderBackend,
    mode: RawFrameMode,
) -> Result<RgbaImage> {
    match (backend, mode) {
        (ActiveRenderBackend::Cpu, RawFrameMode::FullFrame) => render_frame(project, options, time),
        (ActiveRenderBackend::Cpu, RawFrameMode::TransparentOverlay) => {
            render_transparent_overlay_frame(project, options, time)
        }
        (ActiveRenderBackend::Skia(renderer), RawFrameMode::FullFrame) => {
            render_frame_skia_with_renderer(project, options, time, renderer)
        }
        (ActiveRenderBackend::Skia(renderer), RawFrameMode::TransparentOverlay) => {
            let frame = renderer.background_frame(
                project.settings.width,
                project.settings.height,
                Rgba([0, 0, 0, 0]),
            )?;
            render_frame_on_base(project, options, time, frame, false)
        }
        (ActiveRenderBackend::GpuHybrid(renderer), RawFrameMode::FullFrame) => {
            render_frame_gpu_hybrid_with_renderer(project, options, time, renderer)
        }
        (ActiveRenderBackend::GpuHybrid(renderer), RawFrameMode::TransparentOverlay) => {
            let frame = renderer.background_frame(
                project.settings.width,
                project.settings.height,
                Rgba([0, 0, 0, 0]),
            )?;
            render_frame_on_base(project, options, time, frame, false)
        }
    }
}

pub fn render_transparent_overlay_frame(
    project: &Project,
    options: &RenderOptions,
    time: f64,
) -> Result<RgbaImage> {
    let frame = RgbaImage::from_pixel(
        project.settings.width,
        project.settings.height,
        Rgba([0, 0, 0, 0]),
    );
    render_frame_on_base(project, options, time, frame, false)
}

pub fn render_frame(project: &Project, options: &RenderOptions, time: f64) -> Result<RgbaImage> {
    let frame = RgbaImage::from_pixel(
        project.settings.width,
        project.settings.height,
        options.background,
    );
    render_frame_on_base(project, options, time, frame, true)
}

pub fn render_frame_gpu_hybrid(
    project: &Project,
    options: &RenderOptions,
    time: f64,
) -> Result<RgbaImage> {
    let renderer = GpuFrameRenderer::new()?;
    render_frame_gpu_hybrid_with_renderer(project, options, time, &renderer)
}

pub fn render_frame_skia(
    project: &Project,
    options: &RenderOptions,
    time: f64,
) -> Result<RgbaImage> {
    let renderer = SkiaFrameRenderer::new()?;
    render_frame_skia_with_renderer(project, options, time, &renderer)
}

fn render_frame_skia_with_renderer(
    project: &Project,
    options: &RenderOptions,
    time: f64,
    renderer: &SkiaFrameRenderer,
) -> Result<RgbaImage> {
    let frame = renderer.background_frame(
        project.settings.width,
        project.settings.height,
        options.background,
    )?;
    render_frame_on_base(project, options, time, frame, true)
}

fn render_frame_gpu_hybrid_with_renderer(
    project: &Project,
    options: &RenderOptions,
    time: f64,
    renderer: &GpuFrameRenderer,
) -> Result<RgbaImage> {
    let frame = renderer.background_frame(
        project.settings.width,
        project.settings.height,
        options.background,
    )?;
    render_frame_on_base(project, options, time, frame, true)
}

fn render_frame_on_base(
    project: &Project,
    options: &RenderOptions,
    time: f64,
    mut frame: RgbaImage,
    include_video_layers: bool,
) -> Result<RgbaImage> {
    let mut layers = project
        .tracks
        .iter()
        .flat_map(|track| track.layers.iter())
        .filter(|layer| layer.start <= time && time < layer.start + layer.duration)
        .filter(|layer| include_video_layers || !matches!(layer.content, LayerContent::Video(_)))
        .collect::<Vec<_>>();
    layers.sort_by_key(|layer| layer.z_index);

    for layer in layers {
        let local_time = time - layer.start;
        let transform = resolve_layer_transform(layer, local_time);
        draw_layer(project, options, &mut frame, layer, transform, time)?;
    }
    Ok(frame)
}

pub fn gpu_background_frame(width: u32, height: u32, color: Rgba<u8>) -> Result<RgbaImage> {
    GpuFrameRenderer::new()?.background_frame(width, height, color)
}

pub fn skia_background_frame(width: u32, height: u32, color: Rgba<u8>) -> Result<RgbaImage> {
    SkiaFrameRenderer::new()?.background_frame(width, height, color)
}

pub struct SkiaFrameRenderer;

impl SkiaFrameRenderer {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn background_frame(&self, width: u32, height: u32, color: Rgba<u8>) -> Result<RgbaImage> {
        if width == 0 || height == 0 {
            bail!("Skia frame size は 1px 以上である必要があります");
        }
        let mut surface = surfaces::raster_n32_premul((width as i32, height as i32))
            .context("Skia raster surface を作成できません")?;
        surface
            .canvas()
            .clear(Color::from_argb(color[3], color[0], color[1], color[2]));
        let image = surface.image_snapshot();
        let info = ImageInfo::new(
            (width as i32, height as i32),
            ColorType::RGBA8888,
            AlphaType::Unpremul,
            None,
        );
        let mut pixels = vec![0; (u64::from(width) * u64::from(height) * 4) as usize];
        if !image.read_pixels(
            &info,
            &mut pixels,
            (width * 4) as usize,
            (0, 0),
            CachingHint::Disallow,
        ) {
            bail!("Skia surface から pixels を読み出せません");
        }
        RgbaImage::from_raw(width, height, pixels)
            .context("Skia frame を RgbaImage に変換できません")
    }
}

pub struct GpuFrameRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl GpuFrameRenderer {
    pub fn new() -> Result<Self> {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .context("GPU adapter を取得できません")?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("mm gpu renderer"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .context("GPU device を作成できません")?;
        Ok(Self { device, queue })
    }

    pub fn background_frame(&self, width: u32, height: u32, color: Rgba<u8>) -> Result<RgbaImage> {
        if width == 0 || height == 0 {
            bail!("GPU frame size は 1px 以上である必要があります");
        }

        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mm gpu background texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let unpadded_bytes_per_row = width * 4;
        let padded_bytes_per_row = align_to(unpadded_bytes_per_row, GPU_COPY_ALIGNMENT);
        let output_buffer_size = u64::from(padded_bytes_per_row) * u64::from(height);
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mm gpu readback buffer"),
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let clear_color = wgpu::Color {
            r: f64::from(color[0]) / 255.0,
            g: f64::from(color[1]) / 255.0,
            b: f64::from(color[2]) / 255.0,
            a: f64::from(color[3]) / 255.0,
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mm gpu frame encoder"),
            });
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mm gpu clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            texture_size,
        );
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = output_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::Wait)
            .context("GPU readback の完了待機に失敗しました")?;
        receiver
            .recv()
            .context("GPU readback callback を受信できません")?
            .context("GPU readback buffer を map できません")?;

        let mapped = buffer_slice.get_mapped_range();
        let mut pixels = vec![0; (u64::from(width) * u64::from(height) * 4) as usize];
        for row in 0..height as usize {
            let source_offset = row * padded_bytes_per_row as usize;
            let target_offset = row * unpadded_bytes_per_row as usize;
            pixels[target_offset..target_offset + unpadded_bytes_per_row as usize].copy_from_slice(
                &mapped[source_offset..source_offset + unpadded_bytes_per_row as usize],
            );
        }
        drop(mapped);
        output_buffer.unmap();

        RgbaImage::from_raw(width, height, pixels)
            .context("GPU frame を RgbaImage に変換できません")
    }
}

fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

fn draw_layer(
    project: &Project,
    options: &RenderOptions,
    frame: &mut RgbaImage,
    layer: &Layer,
    transform: Transform,
    time: f64,
) -> Result<()> {
    let mut layer_frame = RgbaImage::from_pixel(frame.width(), frame.height(), Rgba([0, 0, 0, 0]));
    draw_layer_content(project, options, &mut layer_frame, layer, transform, time)?;
    apply_transition_to_frame(&mut layer_frame, layer, time - layer.start);
    alpha_blend(frame, &layer_frame, 0, 0, 1.0);
    Ok(())
}

fn draw_layer_content(
    project: &Project,
    options: &RenderOptions,
    frame: &mut RgbaImage,
    layer: &Layer,
    transform: Transform,
    time: f64,
) -> Result<()> {
    match &layer.content {
        LayerContent::Image(content) => {
            let asset = project
                .assets
                .iter()
                .find(|asset| asset.id == content.asset_id)
                .with_context(|| format!("画像 asset が見つかりません: {}", content.asset_id))?;
            let path = options.project_root.join(&asset.path);
            let image = image::open(&path)
                .with_context(|| format!("画像を読み込めません: {}", path.display()))?
                .to_rgba8();
            let image = apply_crop(
                image,
                resolve_layer_crop(layer, content.crop, time - layer.start),
            );
            let image = apply_image_effects(image, &layer.effects);
            let image = apply_mask(image, content.mask.as_ref(), &options.project_root)?;
            draw_image(frame, &image, transform, content.fit);
        }
        LayerContent::Video(content) => {
            let mut image =
                decode_video_layer_frame(project, options, layer, content, time - layer.start)?;
            image = apply_crop(
                image,
                resolve_layer_crop(layer, content.crop, time - layer.start),
            );
            image = apply_image_effects(image, &layer.effects);
            draw_image(frame, &image, transform, content.fit);
        }
        LayerContent::Text(content) => {
            let font = load_font(project, options, content.font_asset_id.as_deref())?;
            draw_text_layer(frame, content, transform, &font);
        }
        LayerContent::Subtitle(content) => {
            if let Some(text) = active_subtitle_text(project, options, content, time - layer.start)?
            {
                let font = load_font(project, options, None)?;
                let subtitle = TextLayer {
                    text,
                    font_asset_id: None,
                    font_size: resolved_subtitle_font_size(layer.transform),
                    color: "#ffffff".to_string(),
                    letter_spacing: 0.0,
                    line_spacing: 1.2,
                    stroke: Some(TextStroke {
                        color: "#000000".to_string(),
                        width: 2.0,
                    }),
                    shadow: Some(TextShadow {
                        color: "#000000".to_string(),
                        offset_x: 2.0,
                        offset_y: 2.0,
                        blur: 0.0,
                    }),
                    gradient: None,
                    align: TextAlign::Center,
                };
                draw_text_layer(
                    frame,
                    &subtitle,
                    subtitle_transform(frame, transform),
                    &font,
                );
            }
        }
        LayerContent::Audio(_) | LayerContent::Voice(_) => {}
    }
    Ok(())
}

fn apply_transition_to_frame(frame: &mut RgbaImage, layer: &Layer, local_time: f64) {
    let Some(transition) = &layer.transition else {
        return;
    };
    let progress = transition_progress(transition, layer.duration, local_time);
    match transition {
        Transition::CrossFade { duration } if *duration > 0.0 => {
            multiply_alpha(frame, progress);
        }
        Transition::Wipe { duration, shape } if *duration > 0.0 => {
            apply_wipe_transition(frame, shape, progress);
        }
        Transition::Blur { duration } if *duration > 0.0 => {
            let radius = (1.0 - progress) * 8.0;
            if radius > 0.01 {
                *frame = imageops::blur(frame, radius);
            }
        }
        Transition::Flash { duration } if *duration > 0.0 => {
            let amount = ((1.0 - progress) * 180.0) as i32;
            if amount > 0 {
                *frame = imageops::brighten(frame, amount);
            }
        }
        Transition::Zoom { duration } if *duration > 0.0 => {
            multiply_alpha(frame, progress.clamp(0.0, 1.0));
        }
        Transition::Push { .. }
        | Transition::CrossFade { .. }
        | Transition::Wipe { .. }
        | Transition::Zoom { .. }
        | Transition::Blur { .. }
        | Transition::Flash { .. } => {}
    }
}

fn transition_progress(transition: &Transition, layer_duration: f64, local_time: f64) -> f32 {
    let duration = transition_duration(transition);
    if duration <= 0.0 {
        return 1.0;
    }
    let intro = (local_time / duration).clamp(0.0, 1.0);
    let outro = ((layer_duration - local_time) / duration).clamp(0.0, 1.0);
    intro.min(outro).clamp(0.0, 1.0) as f32
}

fn transition_duration(transition: &Transition) -> f64 {
    match transition {
        Transition::CrossFade { duration }
        | Transition::Push { duration }
        | Transition::Zoom { duration }
        | Transition::Blur { duration }
        | Transition::Flash { duration } => *duration,
        Transition::Wipe { duration, .. } => *duration,
    }
}

fn multiply_alpha(frame: &mut RgbaImage, amount: f32) {
    let amount = amount.clamp(0.0, 1.0);
    for pixel in frame.pixels_mut() {
        pixel[3] = (f32::from(pixel[3]) * amount).round() as u8;
    }
}

fn apply_wipe_transition(frame: &mut RgbaImage, shape: &WipeShape, progress: f32) {
    let progress = progress.clamp(0.0, 1.0);
    match shape {
        WipeShape::Circle => apply_circle_wipe(frame, progress),
        WipeShape::RoundedRect {
            radius,
            border,
            shadow,
        } => apply_rounded_rect_wipe(frame, *radius, border.as_ref(), shadow.as_ref(), progress),
    }
}

fn apply_circle_wipe(frame: &mut RgbaImage, progress: f32) {
    let center_x = frame.width() as f32 / 2.0;
    let center_y = frame.height() as f32 / 2.0;
    let max_radius = (center_x * center_x + center_y * center_y).sqrt();
    let radius = max_radius * progress;
    for (x, y, pixel) in frame.enumerate_pixels_mut() {
        let dx = x as f32 + 0.5 - center_x;
        let dy = y as f32 + 0.5 - center_y;
        if (dx * dx + dy * dy).sqrt() > radius {
            pixel[3] = 0;
        }
    }
}

fn apply_rounded_rect_wipe(
    frame: &mut RgbaImage,
    radius: f32,
    border: Option<&mm_core::WipeBorder>,
    shadow: Option<&TextShadow>,
    progress: f32,
) {
    let wipe_width = frame.width() as f32 * progress;
    let wipe_height = frame.height() as f32 * progress;
    let origin_x = (frame.width() as f32 - wipe_width) / 2.0;
    let origin_y = (frame.height() as f32 - wipe_height) / 2.0;
    let radius = radius.max(0.0).min(wipe_width.min(wipe_height) / 2.0);
    let border_color = border.and_then(|value| parse_color(&value.color));
    let border_width = border.map(|value| value.width.max(0.0)).unwrap_or_default();
    let shadow_color = shadow.and_then(|value| parse_color(&value.color));
    for (x, y, pixel) in frame.enumerate_pixels_mut() {
        let px = x as f32 + 0.5;
        let py = y as f32 + 0.5;
        let distance = rounded_rect_signed_distance(
            px,
            py,
            origin_x,
            origin_y,
            wipe_width,
            wipe_height,
            radius,
        );
        if distance > 0.0 {
            if let (Some(shadow), Some(color)) = (shadow, shadow_color) {
                let shadow_distance = rounded_rect_signed_distance(
                    px - shadow.offset_x,
                    py - shadow.offset_y,
                    origin_x,
                    origin_y,
                    wipe_width,
                    wipe_height,
                    radius,
                );
                if shadow_distance <= shadow.blur.max(0.0) {
                    let falloff = if shadow_distance <= 0.0 {
                        1.0
                    } else {
                        1.0 - shadow_distance / shadow.blur.max(1.0)
                    };
                    let alpha = (f32::from(color[3]) * falloff).round().clamp(0.0, 255.0) as u8;
                    *pixel = Rgba([color[0], color[1], color[2], alpha]);
                    continue;
                }
            }
            pixel[3] = 0;
            continue;
        }
        if let Some(color) = border_color {
            if border_width > 0.0 && -distance <= border_width {
                *pixel = color;
            }
        }
    }
}

fn rounded_rect_signed_distance(
    px: f32,
    py: f32,
    origin_x: f32,
    origin_y: f32,
    width: f32,
    height: f32,
    radius: f32,
) -> f32 {
    let center_x = origin_x + width / 2.0;
    let center_y = origin_y + height / 2.0;
    let half_width = (width / 2.0 - radius).max(0.0);
    let half_height = (height / 2.0 - radius).max(0.0);
    let qx = (px - center_x).abs() - half_width;
    let qy = (py - center_y).abs() - half_height;
    let outside_x = qx.max(0.0);
    let outside_y = qy.max(0.0);
    let outside = (outside_x * outside_x + outside_y * outside_y).sqrt();
    let inside = qx.max(qy).min(0.0);
    outside + inside - radius
}

fn resolve_layer_transform(layer: &Layer, local_time: f64) -> Transform {
    let mut transform = layer.transform;
    for animation in &layer.animations {
        if let Some(value) = resolve_animation_value(animation, local_time) {
            match animation.property {
                AnimatedProperty::X => transform.x = value,
                AnimatedProperty::Y => transform.y = value,
                AnimatedProperty::Scale => transform.scale = value,
                AnimatedProperty::Rotation => transform.rotation = value,
                AnimatedProperty::Opacity => transform.opacity = value,
                AnimatedProperty::Width => transform.width = value,
                AnimatedProperty::Height => transform.height = value,
                AnimatedProperty::CropX
                | AnimatedProperty::CropY
                | AnimatedProperty::CropWidth
                | AnimatedProperty::CropHeight => {}
            }
        }
    }
    let transform = apply_transform_effects(layer, local_time, transform);
    apply_transition_transform(layer, local_time, transform)
}

fn resolve_layer_crop(layer: &Layer, base: Option<Crop>, local_time: f64) -> Option<Crop> {
    let mut crop = base.unwrap_or(Crop {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    });
    let mut changed = base.is_some();
    for animation in &layer.animations {
        if let Some(value) = resolve_animation_value(animation, local_time) {
            match animation.property {
                AnimatedProperty::CropX => {
                    crop.x = value.round().max(0.0) as u32;
                    changed = true;
                }
                AnimatedProperty::CropY => {
                    crop.y = value.round().max(0.0) as u32;
                    changed = true;
                }
                AnimatedProperty::CropWidth => {
                    crop.width = value.round().max(0.0) as u32;
                    changed = true;
                }
                AnimatedProperty::CropHeight => {
                    crop.height = value.round().max(0.0) as u32;
                    changed = true;
                }
                AnimatedProperty::X
                | AnimatedProperty::Y
                | AnimatedProperty::Scale
                | AnimatedProperty::Rotation
                | AnimatedProperty::Opacity
                | AnimatedProperty::Width
                | AnimatedProperty::Height => {}
            }
        }
    }
    changed.then_some(crop)
}

fn apply_transition_transform(
    layer: &Layer,
    local_time: f64,
    mut transform: Transform,
) -> Transform {
    let Some(transition) = &layer.transition else {
        return transform;
    };
    let progress = transition_progress(transition, layer.duration, local_time);
    match transition {
        Transition::Push { duration } if *duration > 0.0 => {
            let offset = (1.0 - progress) * transform.width.max(1.0);
            transform.x -= offset;
        }
        Transition::Zoom { duration } if *duration > 0.0 => {
            transform.scale *= 0.85 + 0.15 * progress;
        }
        Transition::CrossFade { .. }
        | Transition::Wipe { .. }
        | Transition::Push { .. }
        | Transition::Zoom { .. }
        | Transition::Blur { .. }
        | Transition::Flash { .. } => {}
    }
    transform
}

fn resolve_animation_value(animation: &Animation, local_time: f64) -> Option<f32> {
    let first = animation.keyframes.first()?;
    if local_time <= first.time {
        return Some(first.value);
    }
    for pair in animation.keyframes.windows(2) {
        let start = pair[0];
        let end = pair[1];
        if start.time <= local_time && local_time <= end.time {
            let span = (end.time - start.time).max(f64::EPSILON);
            let progress = ((local_time - start.time) / span).clamp(0.0, 1.0) as f32;
            let eased = apply_easing(animation.easing, progress);
            return Some(start.value + (end.value - start.value) * eased);
        }
    }
    animation.keyframes.last().map(|keyframe| keyframe.value)
}

fn apply_easing(easing: Easing, progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    match easing {
        Easing::Linear => t,
        Easing::EaseIn => t * t,
        Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        Easing::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        Easing::EaseOutBack => {
            let c1 = 1.70158;
            let c3 = c1 + 1.0;
            1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
        }
        Easing::Bounce => ease_out_bounce(t),
        Easing::Elastic => {
            if t == 0.0 || t == 1.0 {
                t
            } else {
                let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
            }
        }
    }
}

fn ease_out_bounce(t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let shifted = t - 1.5 / d1;
        n1 * shifted * shifted + 0.75
    } else if t < 2.5 / d1 {
        let shifted = t - 2.25 / d1;
        n1 * shifted * shifted + 0.9375
    } else {
        let shifted = t - 2.625 / d1;
        n1 * shifted * shifted + 0.984375
    }
}

fn apply_transform_effects(layer: &Layer, local_time: f64, mut transform: Transform) -> Transform {
    for effect in &layer.effects {
        match *effect {
            Effect::FadeIn { duration } if duration > 0.0 => {
                let amount = (local_time / duration).clamp(0.0, 1.0) as f32;
                transform.opacity *= amount;
            }
            Effect::FadeOut { duration } if duration > 0.0 => {
                let remaining = (layer.duration - local_time).max(0.0);
                let amount = (remaining / duration).clamp(0.0, 1.0) as f32;
                transform.opacity *= amount;
            }
            Effect::Zoom { amount } => {
                transform.scale *= amount.max(0.01);
            }
            Effect::Slide { x, y } => {
                transform.x += x;
                transform.y += y;
            }
            Effect::FadeIn { .. }
            | Effect::FadeOut { .. }
            | Effect::Blur { .. }
            | Effect::Brightness { .. }
            | Effect::Contrast { .. }
            | Effect::Saturation { .. }
            | Effect::Pixelate { .. }
            | Effect::MotionBlur { .. } => {}
        }
    }
    transform.opacity = transform.opacity.clamp(0.0, 1.0);
    transform
}

fn apply_image_effects(mut image: RgbaImage, effects: &[Effect]) -> RgbaImage {
    for effect in effects {
        image = match *effect {
            Effect::Blur { radius } if radius > 0.0 => imageops::blur(&image, radius),
            Effect::Brightness { amount } => imageops::brighten(&image, (amount * 255.0) as i32),
            Effect::Contrast { amount } => imageops::contrast(&image, amount),
            Effect::Saturation { amount } => adjust_saturation(&image, amount),
            Effect::Pixelate { size } if size > 1 => pixelate(&image, size),
            Effect::MotionBlur { amount } if amount > 0.0 => motion_blur(&image, amount),
            Effect::FadeIn { .. }
            | Effect::FadeOut { .. }
            | Effect::Blur { .. }
            | Effect::Zoom { .. }
            | Effect::Slide { .. }
            | Effect::Pixelate { .. }
            | Effect::MotionBlur { .. } => image,
        };
    }
    image
}

fn apply_crop(image: RgbaImage, crop: Option<Crop>) -> RgbaImage {
    let Some(crop) = crop else {
        return image;
    };
    if crop.width == 0 || crop.height == 0 || crop.x >= image.width() || crop.y >= image.height() {
        return image;
    }
    let width = crop.width.min(image.width() - crop.x);
    let height = crop.height.min(image.height() - crop.y);
    imageops::crop_imm(&image, crop.x, crop.y, width, height).to_image()
}

fn apply_mask(mut image: RgbaImage, mask: Option<&Mask>, project_root: &Path) -> Result<RgbaImage> {
    match mask {
        Some(Mask::Circle) => apply_ellipse_mask(&mut image, true),
        Some(Mask::Ellipse) => apply_ellipse_mask(&mut image, false),
        Some(Mask::RoundedRect { radius }) => apply_rounded_rect_mask(&mut image, *radius),
        Some(Mask::Svg { path }) => apply_svg_mask(&mut image, project_root, path)?,
        None => {}
    }
    Ok(image)
}

fn apply_ellipse_mask(image: &mut RgbaImage, force_circle: bool) {
    let width = image.width() as f32;
    let height = image.height() as f32;
    let radius_x = if force_circle {
        width.min(height) / 2.0
    } else {
        width / 2.0
    };
    let radius_y = if force_circle {
        width.min(height) / 2.0
    } else {
        height / 2.0
    };
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let dx = (x as f32 + 0.5 - center_x) / radius_x.max(0.1);
        let dy = (y as f32 + 0.5 - center_y) / radius_y.max(0.1);
        if dx * dx + dy * dy > 1.0 {
            pixel[3] = 0;
        }
    }
}

fn apply_rounded_rect_mask(image: &mut RgbaImage, radius: f32) {
    let width = image.width() as f32;
    let height = image.height() as f32;
    let radius = radius.max(0.0).min(width.min(height) / 2.0);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let px = x as f32 + 0.5;
        let py = y as f32 + 0.5;
        let corner_x = if px < radius {
            radius
        } else if px > width - radius {
            width - radius
        } else {
            px
        };
        let corner_y = if py < radius {
            radius
        } else if py > height - radius {
            height - radius
        } else {
            py
        };
        let dx = px - corner_x;
        let dy = py - corner_y;
        if dx * dx + dy * dy > radius * radius {
            pixel[3] = 0;
        }
    }
}

fn apply_svg_mask(image: &mut RgbaImage, project_root: &Path, path: &Path) -> Result<()> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };
    let text = fs::read_to_string(&path)
        .with_context(|| format!("SVG mask を読み込めません: {}", path.display()))?;
    let document = roxmltree::Document::parse(&text)
        .with_context(|| format!("SVG mask を parse できません: {}", path.display()))?;
    let root = document.root_element();
    let svg_width = svg_length(root.attribute("width")).unwrap_or(image.width() as f32);
    let svg_height = svg_length(root.attribute("height")).unwrap_or(image.height() as f32);
    let scale_x = image.width() as f32 / svg_width.max(1.0);
    let scale_y = image.height() as f32 / svg_height.max(1.0);
    let mut mask = vec![0u8; (image.width() * image.height()) as usize];
    let mut painted = false;

    for node in document.descendants().filter(|node| node.is_element()) {
        let alpha = svg_paint_alpha(&node);
        if alpha == 0 {
            continue;
        }
        match node.tag_name().name() {
            "rect" => {
                painted |= paint_svg_rect(
                    &mut mask,
                    image.width(),
                    image.height(),
                    &node,
                    alpha,
                    scale_x,
                    scale_y,
                );
            }
            "circle" => {
                painted |= paint_svg_circle(
                    &mut mask,
                    image.width(),
                    image.height(),
                    &node,
                    alpha,
                    scale_x,
                    scale_y,
                );
            }
            "ellipse" => {
                painted |= paint_svg_ellipse(
                    &mut mask,
                    image.width(),
                    image.height(),
                    &node,
                    alpha,
                    scale_x,
                    scale_y,
                );
            }
            _ => {}
        }
    }

    if !painted {
        bail!(
            "SVG mask に対応図形がありません: {} (rect/circle/ellipse)",
            path.display()
        );
    }

    for (index, pixel) in image.pixels_mut().enumerate() {
        pixel[3] = ((u16::from(pixel[3]) * u16::from(mask[index])) / 255) as u8;
    }
    Ok(())
}

fn paint_svg_rect(
    mask: &mut [u8],
    width: u32,
    height: u32,
    node: &roxmltree::Node<'_, '_>,
    alpha: u8,
    scale_x: f32,
    scale_y: f32,
) -> bool {
    let x = svg_length(node.attribute("x")).unwrap_or(0.0) * scale_x;
    let y = svg_length(node.attribute("y")).unwrap_or(0.0) * scale_y;
    let rect_width = svg_length(node.attribute("width")).unwrap_or(0.0) * scale_x;
    let rect_height = svg_length(node.attribute("height")).unwrap_or(0.0) * scale_y;
    if rect_width <= 0.0 || rect_height <= 0.0 {
        return false;
    }
    let radius = svg_length(node.attribute("rx"))
        .or_else(|| svg_length(node.attribute("ry")))
        .unwrap_or(0.0)
        * scale_x.min(scale_y);
    let mut painted = false;
    for py in 0..height {
        for px in 0..width {
            let cx = px as f32 + 0.5;
            let cy = py as f32 + 0.5;
            let inside = if radius > 0.0 {
                rounded_rect_signed_distance(cx, cy, x, y, rect_width, rect_height, radius) <= 0.0
            } else {
                x <= cx && cx < x + rect_width && y <= cy && cy < y + rect_height
            };
            if inside {
                mask[(py * width + px) as usize] = mask[(py * width + px) as usize].max(alpha);
                painted = true;
            }
        }
    }
    painted
}

fn paint_svg_circle(
    mask: &mut [u8],
    width: u32,
    height: u32,
    node: &roxmltree::Node<'_, '_>,
    alpha: u8,
    scale_x: f32,
    scale_y: f32,
) -> bool {
    let cx = svg_length(node.attribute("cx")).unwrap_or(0.0) * scale_x;
    let cy = svg_length(node.attribute("cy")).unwrap_or(0.0) * scale_y;
    let radius = svg_length(node.attribute("r")).unwrap_or(0.0) * scale_x.min(scale_y);
    paint_svg_ellipse_pixels(mask, width, height, cx, cy, radius, radius, alpha)
}

fn paint_svg_ellipse(
    mask: &mut [u8],
    width: u32,
    height: u32,
    node: &roxmltree::Node<'_, '_>,
    alpha: u8,
    scale_x: f32,
    scale_y: f32,
) -> bool {
    let cx = svg_length(node.attribute("cx")).unwrap_or(0.0) * scale_x;
    let cy = svg_length(node.attribute("cy")).unwrap_or(0.0) * scale_y;
    let rx = svg_length(node.attribute("rx")).unwrap_or(0.0) * scale_x;
    let ry = svg_length(node.attribute("ry")).unwrap_or(0.0) * scale_y;
    paint_svg_ellipse_pixels(mask, width, height, cx, cy, rx, ry, alpha)
}

fn paint_svg_ellipse_pixels(
    mask: &mut [u8],
    width: u32,
    height: u32,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    alpha: u8,
) -> bool {
    if rx <= 0.0 || ry <= 0.0 {
        return false;
    }
    let mut painted = false;
    for py in 0..height {
        for px in 0..width {
            let dx = (px as f32 + 0.5 - cx) / rx;
            let dy = (py as f32 + 0.5 - cy) / ry;
            if dx * dx + dy * dy <= 1.0 {
                mask[(py * width + px) as usize] = mask[(py * width + px) as usize].max(alpha);
                painted = true;
            }
        }
    }
    painted
}

fn svg_paint_alpha(node: &roxmltree::Node<'_, '_>) -> u8 {
    if node.attribute("fill") == Some("none") {
        return 0;
    }
    let opacity = svg_length(node.attribute("opacity")).unwrap_or(1.0)
        * svg_length(node.attribute("fill-opacity")).unwrap_or(1.0);
    let luminance = node
        .attribute("fill")
        .and_then(parse_color)
        .map(|color| {
            (0.2126 * f32::from(color[0])
                + 0.7152 * f32::from(color[1])
                + 0.0722 * f32::from(color[2]))
                / 255.0
        })
        .unwrap_or(1.0);
    (opacity.clamp(0.0, 1.0) * luminance.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn svg_length(value: Option<&str>) -> Option<f32> {
    let value = value?.trim();
    let end = value
        .char_indices()
        .take_while(|(_, character)| {
            character.is_ascii_digit() || matches!(character, '.' | '-' | '+')
        })
        .map(|(index, character)| index + character.len_utf8())
        .last()?;
    value[..end].parse().ok()
}

fn adjust_saturation(image: &RgbaImage, amount: f32) -> RgbaImage {
    let saturation = amount.max(0.0);
    let mut adjusted = image.clone();
    for pixel in adjusted.pixels_mut() {
        let r = f32::from(pixel[0]);
        let g = f32::from(pixel[1]);
        let b = f32::from(pixel[2]);
        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        pixel[0] = (luma + (r - luma) * saturation).round().clamp(0.0, 255.0) as u8;
        pixel[1] = (luma + (g - luma) * saturation).round().clamp(0.0, 255.0) as u8;
        pixel[2] = (luma + (b - luma) * saturation).round().clamp(0.0, 255.0) as u8;
    }
    adjusted
}

fn motion_blur(image: &RgbaImage, amount: f32) -> RgbaImage {
    let radius = amount.round().max(1.0) as i32;
    let mut blurred = image.clone();
    for y in 0..image.height() {
        for x in 0..image.width() {
            let mut sum = [0u32; 4];
            let mut count = 0u32;
            for offset in -radius..=radius {
                let sample_x = x as i32 + offset;
                if sample_x < 0 || sample_x >= image.width() as i32 {
                    continue;
                }
                let pixel = image.get_pixel(sample_x as u32, y);
                for channel in 0..4 {
                    sum[channel] += u32::from(pixel[channel]);
                }
                count += 1;
            }
            let target = blurred.get_pixel_mut(x, y);
            for channel in 0..4 {
                target[channel] = (sum[channel] / count.max(1)) as u8;
            }
        }
    }
    blurred
}

fn pixelate(image: &RgbaImage, size: u32) -> RgbaImage {
    let small_width = (image.width() / size).max(1);
    let small_height = (image.height() / size).max(1);
    let small = imageops::resize(
        image,
        small_width,
        small_height,
        imageops::FilterType::Nearest,
    );
    imageops::resize(
        &small,
        image.width(),
        image.height(),
        imageops::FilterType::Nearest,
    )
}

fn draw_image(
    frame: &mut RgbaImage,
    image: &RgbaImage,
    transform: Transform,
    fit: Option<FitMode>,
) {
    let Some(fit) = fit else {
        draw_resized_image(frame, image, transform);
        return;
    };

    let target_width = resolved_size(transform.width, image.width());
    let target_height = resolved_size(transform.height, image.height());
    let mut fitted = RgbaImage::from_pixel(target_width, target_height, Rgba([0, 0, 0, 0]));
    match fit {
        FitMode::Contain => draw_fit_into_canvas(&mut fitted, image, FitMode::Contain),
        FitMode::Cover => draw_fit_into_canvas(&mut fitted, image, FitMode::Cover),
        FitMode::Stretch => draw_fit_into_canvas(&mut fitted, image, FitMode::Stretch),
        FitMode::BlurBackground => {
            let mut background =
                fit_image_to_canvas(image, target_width, target_height, FitMode::Cover);
            background = imageops::blur(&background, 18.0);
            alpha_blend(&mut fitted, &background, 0, 0, 1.0);
            draw_fit_into_canvas(&mut fitted, image, FitMode::Contain);
        }
    }
    let mut x = transform.x.round() as i32;
    let mut y = transform.y.round() as i32;
    if transform.rotation != 0.0 {
        let rotated = rotate_image(&fitted, transform.rotation);
        x -= (rotated.width() as i32 - fitted.width() as i32) / 2;
        y -= (rotated.height() as i32 - fitted.height() as i32) / 2;
        fitted = rotated;
    }
    alpha_blend(frame, &fitted, x, y, transform.opacity);
}

fn draw_resized_image(frame: &mut RgbaImage, image: &RgbaImage, transform: Transform) {
    let width = resolved_size(transform.width, image.width());
    let height = resolved_size(transform.height, image.height());
    let mut resized = imageops::resize(image, width, height, imageops::FilterType::Lanczos3);
    let mut x = transform.x.round() as i32;
    let mut y = transform.y.round() as i32;
    if transform.rotation != 0.0 {
        let rotated = rotate_image(&resized, transform.rotation);
        x -= (rotated.width() as i32 - resized.width() as i32) / 2;
        y -= (rotated.height() as i32 - resized.height() as i32) / 2;
        resized = rotated;
    }
    alpha_blend(frame, &resized, x, y, transform.opacity);
}

fn draw_fit_into_canvas(canvas: &mut RgbaImage, image: &RgbaImage, fit: FitMode) {
    let fitted = fit_image_to_canvas(image, canvas.width(), canvas.height(), fit);
    alpha_blend(canvas, &fitted, 0, 0, 1.0);
}

fn fit_image_to_canvas(
    image: &RgbaImage,
    target_width: u32,
    target_height: u32,
    fit: FitMode,
) -> RgbaImage {
    let (width, height) = fit_dimensions(
        image.width(),
        image.height(),
        target_width,
        target_height,
        fit,
    );
    let resized = imageops::resize(image, width, height, imageops::FilterType::Lanczos3);
    let mut canvas = RgbaImage::from_pixel(target_width, target_height, Rgba([0, 0, 0, 0]));
    let x = (target_width as i32 - width as i32) / 2;
    let y = (target_height as i32 - height as i32) / 2;
    alpha_blend(&mut canvas, &resized, x, y, 1.0);
    canvas
}

fn fit_dimensions(
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
    fit: FitMode,
) -> (u32, u32) {
    if source_width == 0 || source_height == 0 || target_width == 0 || target_height == 0 {
        return (target_width.max(1), target_height.max(1));
    }
    match fit {
        FitMode::Stretch => (target_width, target_height),
        FitMode::Contain | FitMode::BlurBackground => {
            let scale = (target_width as f32 / source_width as f32)
                .min(target_height as f32 / source_height as f32);
            (
                (source_width as f32 * scale).round().max(1.0) as u32,
                (source_height as f32 * scale).round().max(1.0) as u32,
            )
        }
        FitMode::Cover => {
            let scale = (target_width as f32 / source_width as f32)
                .max(target_height as f32 / source_height as f32);
            (
                (source_width as f32 * scale).round().max(1.0) as u32,
                (source_height as f32 * scale).round().max(1.0) as u32,
            )
        }
    }
}

fn rotate_image(image: &RgbaImage, degrees: f32) -> RgbaImage {
    let radians = degrees.to_radians();
    let sin = radians.sin().abs();
    let cos = radians.cos().abs();
    let new_width = (image.width() as f32 * cos + image.height() as f32 * sin)
        .ceil()
        .max(1.0) as u32;
    let new_height = (image.width() as f32 * sin + image.height() as f32 * cos)
        .ceil()
        .max(1.0) as u32;
    let mut rotated = RgbaImage::from_pixel(new_width, new_height, Rgba([0, 0, 0, 0]));
    let source_cx = image.width() as f32 / 2.0;
    let source_cy = image.height() as f32 / 2.0;
    let target_cx = new_width as f32 / 2.0;
    let target_cy = new_height as f32 / 2.0;
    let cos = radians.cos();
    let sin = radians.sin();
    for y in 0..new_height {
        for x in 0..new_width {
            let dx = x as f32 + 0.5 - target_cx;
            let dy = y as f32 + 0.5 - target_cy;
            let source_x = cos * dx + sin * dy + source_cx;
            let source_y = -sin * dx + cos * dy + source_cy;
            if source_x >= 0.0
                && source_y >= 0.0
                && source_x < image.width() as f32
                && source_y < image.height() as f32
            {
                let pixel = image.get_pixel(source_x.floor() as u32, source_y.floor() as u32);
                rotated.put_pixel(x, y, *pixel);
            }
        }
    }
    rotated
}

fn resolved_size(value: f32, fallback: u32) -> u32 {
    if value > 0.0 {
        value.round().max(1.0) as u32
    } else {
        fallback
    }
}

fn draw_text_layer(frame: &mut RgbaImage, text: &TextLayer, transform: Transform, font: &FontArc) {
    if transform.rotation != 0.0 {
        let mut text_frame =
            RgbaImage::from_pixel(frame.width(), frame.height(), Rgba([0, 0, 0, 0]));
        let mut transform_without_rotation = transform;
        transform_without_rotation.rotation = 0.0;
        draw_text_layer_unrotated(&mut text_frame, text, transform_without_rotation, font);
        let rotated =
            rotate_canvas_about_point(&text_frame, transform.rotation, transform.x, transform.y);
        alpha_blend(frame, &rotated, 0, 0, 1.0);
        return;
    }
    draw_text_layer_unrotated(frame, text, transform, font);
}

fn draw_text_layer_unrotated(
    frame: &mut RgbaImage,
    text: &TextLayer,
    transform: Transform,
    font: &FontArc,
) {
    let scale = PxScale::from(text.font_size * transform.scale.max(0.01));
    let scaled = font.as_scaled(scale);
    let line_height =
        (scaled.ascent() - scaled.descent() + scaled.line_gap()) * text.line_spacing.max(0.1);
    let lines = text.text.lines().collect::<Vec<_>>();
    let base_color = parse_color(&text.color).unwrap_or(Rgba([255, 255, 255, 255]));
    let base_paint = text_paint(text, transform, base_color);
    let origin_y = transform.y + scaled.ascent();

    if let Some(shadow) = &text.shadow {
        let color = parse_color(&shadow.color).unwrap_or(Rgba([0, 0, 0, 180]));
        draw_text_lines(
            frame,
            font,
            scale,
            &lines,
            text,
            transform.x + shadow.offset_x,
            origin_y + shadow.offset_y,
            line_height,
            TextPaint::Solid(color),
            transform.opacity,
        );
    }

    if let Some(stroke) = &text.stroke {
        let color = parse_color(&stroke.color).unwrap_or(Rgba([0, 0, 0, 255]));
        let radius = stroke.width.round().max(0.0) as i32;
        for offset_y in -radius..=radius {
            for offset_x in -radius..=radius {
                if offset_x == 0 && offset_y == 0 {
                    continue;
                }
                draw_text_lines(
                    frame,
                    font,
                    scale,
                    &lines,
                    text,
                    transform.x + offset_x as f32,
                    origin_y + offset_y as f32,
                    line_height,
                    TextPaint::Solid(color),
                    transform.opacity,
                );
            }
        }
    }

    draw_text_lines(
        frame,
        font,
        scale,
        &lines,
        text,
        transform.x,
        origin_y,
        line_height,
        base_paint,
        transform.opacity,
    );
}

fn rotate_canvas_about_point(
    image: &RgbaImage,
    degrees: f32,
    center_x: f32,
    center_y: f32,
) -> RgbaImage {
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let mut rotated = RgbaImage::from_pixel(image.width(), image.height(), Rgba([0, 0, 0, 0]));
    for y in 0..image.height() {
        for x in 0..image.width() {
            let dx = x as f32 + 0.5 - center_x;
            let dy = y as f32 + 0.5 - center_y;
            let source_x = cos * dx + sin * dy + center_x;
            let source_y = -sin * dx + cos * dy + center_y;
            if source_x >= 0.0
                && source_y >= 0.0
                && source_x < image.width() as f32
                && source_y < image.height() as f32
            {
                let pixel = image.get_pixel(source_x.floor() as u32, source_y.floor() as u32);
                rotated.put_pixel(x, y, *pixel);
            }
        }
    }
    rotated
}

#[allow(clippy::too_many_arguments)]
fn draw_text_lines(
    frame: &mut RgbaImage,
    font: &FontArc,
    scale: PxScale,
    lines: &[&str],
    layer: &TextLayer,
    x: f32,
    y: f32,
    line_height: f32,
    paint: TextPaint,
    opacity: f32,
) {
    for (line_index, line) in lines.iter().enumerate() {
        let line_width = measure_text(font, scale, line, layer.letter_spacing);
        let line_x = match layer.align {
            TextAlign::Left => x,
            TextAlign::Center => x - line_width / 2.0,
            TextAlign::Right => x - line_width,
        };
        let line_y = y + line_index as f32 * line_height;
        draw_text_run(
            frame,
            font,
            scale,
            line,
            line_x,
            line_y,
            layer.letter_spacing,
            paint,
            opacity,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_text_run(
    frame: &mut RgbaImage,
    font: &FontArc,
    scale: PxScale,
    text: &str,
    x: f32,
    baseline_y: f32,
    letter_spacing: f32,
    paint: TextPaint,
    opacity: f32,
) {
    let scaled = font.as_scaled(scale);
    let mut pen_x = x;
    for character in text.chars() {
        let glyph_id = scaled.glyph_id(character);
        let glyph = glyph_id.with_scale_and_position(scale, point(pen_x, baseline_y));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|glyph_x, glyph_y, coverage| {
                let pixel_x = bounds.min.x as i32 + glyph_x as i32;
                let pixel_y = bounds.min.y as i32 + glyph_y as i32;
                let color = text_paint_color(paint, pixel_x, pixel_y);
                blend_pixel(frame, pixel_x, pixel_y, color, coverage * opacity);
            });
        }
        pen_x += scaled.h_advance(glyph_id) + letter_spacing;
    }
}

#[derive(Debug, Clone, Copy)]
enum TextPaint {
    Solid(Rgba<u8>),
    Gradient {
        start: Rgba<u8>,
        end: Rgba<u8>,
        direction: GradientDirection,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
}

fn text_paint(text: &TextLayer, transform: Transform, fallback: Rgba<u8>) -> TextPaint {
    let Some(gradient) = &text.gradient else {
        return TextPaint::Solid(fallback);
    };
    TextPaint::Gradient {
        start: parse_color(&gradient.start_color).unwrap_or(fallback),
        end: parse_color(&gradient.end_color).unwrap_or(fallback),
        direction: gradient.direction,
        x: transform.x,
        y: transform.y,
        width: transform.width.max(1.0),
        height: transform.height.max(text.font_size.max(1.0)),
    }
}

fn text_paint_color(paint: TextPaint, x: i32, y: i32) -> Rgba<u8> {
    match paint {
        TextPaint::Solid(color) => color,
        TextPaint::Gradient {
            start,
            end,
            direction,
            x: origin_x,
            y: origin_y,
            width,
            height,
        } => {
            let progress = match direction {
                GradientDirection::Horizontal => (x as f32 - origin_x) / width,
                GradientDirection::Vertical => (y as f32 - origin_y) / height,
            }
            .clamp(0.0, 1.0);
            lerp_color(start, end, progress)
        }
    }
}

fn lerp_color(start: Rgba<u8>, end: Rgba<u8>, progress: f32) -> Rgba<u8> {
    let mut color = [0u8; 4];
    for channel in 0..4 {
        color[channel] = (f32::from(start[channel])
            + (f32::from(end[channel]) - f32::from(start[channel])) * progress)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    Rgba(color)
}

fn measure_text(font: &FontArc, scale: PxScale, text: &str, letter_spacing: f32) -> f32 {
    let scaled = font.as_scaled(scale);
    let mut width = 0.0;
    let mut count: usize = 0;
    for character in text.chars() {
        let glyph_id: GlyphId = scaled.glyph_id(character);
        width += scaled.h_advance(glyph_id);
        count += 1;
    }
    width + letter_spacing * count.saturating_sub(1) as f32
}

fn blend_pixel(frame: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>, alpha: f32) {
    if x < 0 || y < 0 || x >= frame.width() as i32 || y >= frame.height() as i32 {
        return;
    }
    let alpha = (f32::from(color[3]) / 255.0 * alpha).clamp(0.0, 1.0);
    let destination = frame.get_pixel_mut(x as u32, y as u32);
    for channel in 0..3 {
        destination[channel] = ((f32::from(color[channel]) * alpha)
            + (f32::from(destination[channel]) * (1.0 - alpha)))
            .round() as u8;
    }
    destination[3] = 255;
}

fn parse_color(value: &str) -> Option<Rgba<u8>> {
    let hex = value.strip_prefix('#')?;
    let parse_pair = |index| u8::from_str_radix(&hex[index..index + 2], 16).ok();
    match hex.len() {
        6 => Some(Rgba([parse_pair(0)?, parse_pair(2)?, parse_pair(4)?, 255])),
        8 => Some(Rgba([
            parse_pair(0)?,
            parse_pair(2)?,
            parse_pair(4)?,
            parse_pair(6)?,
        ])),
        _ => None,
    }
}

fn load_font(
    project: &Project,
    options: &RenderOptions,
    font_asset_id: Option<&str>,
) -> Result<FontArc> {
    let font_path = font_asset_id
        .and_then(|asset_id| {
            project
                .assets
                .iter()
                .find(|asset| asset.id == asset_id && asset.kind == AssetKind::Font)
        })
        .map(|asset| options.project_root.join(&asset.path))
        .or_else(system_font_path)
        .context("利用可能なフォントが見つかりません")?;
    let bytes = fs::read(&font_path)
        .with_context(|| format!("フォントを読み込めません: {}", font_path.display()))?;
    FontArc::try_from_vec(bytes)
        .map_err(|_| anyhow::anyhow!("フォントを parse できません: {}", font_path.display()))
}

fn system_font_path() -> Option<PathBuf> {
    let candidates = [
        "/usr/share/fonts/Adwaita/AdwaitaSans-Regular.ttf",
        "/usr/share/fonts/gnu-free/FreeSans.otf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "C:/Windows/Fonts/arial.ttf",
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
}

fn active_subtitle_text(
    project: &Project,
    options: &RenderOptions,
    content: &SubtitleLayer,
    local_time: f64,
) -> Result<Option<String>> {
    let asset = project
        .assets
        .iter()
        .find(|asset| asset.id == content.asset_id)
        .with_context(|| format!("字幕 asset が見つかりません: {}", content.asset_id))?;
    let path = options.project_root.join(&asset.path);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("字幕ファイルを読み込めません: {}", path.display()))?;
    let cues = parse_subtitles(&text);
    Ok(cues
        .into_iter()
        .find(|cue| cue.start <= local_time && local_time < cue.end)
        .map(|cue| cue.text))
}

#[derive(Debug, Clone, PartialEq)]
struct SubtitleCue {
    start: f64,
    end: f64,
    text: String,
}

fn parse_subtitles(input: &str) -> Vec<SubtitleCue> {
    if input
        .lines()
        .any(|line| line.trim_start().starts_with("Dialogue:"))
    {
        return parse_ass(input);
    }
    parse_srt_or_vtt(input)
}

fn parse_srt_or_vtt(input: &str) -> Vec<SubtitleCue> {
    let normalized = input.replace("\r\n", "\n");
    normalized
        .split("\n\n")
        .filter_map(|block| {
            let mut lines = block
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && *line != "WEBVTT");
            let first = lines.next()?;
            let timing = if first.contains("-->") {
                first
            } else {
                lines.next()?
            };
            let (start, end) = parse_time_range(timing)?;
            let text = lines.collect::<Vec<_>>().join("\n");
            if text.is_empty() {
                None
            } else {
                Some(SubtitleCue { start, end, text })
            }
        })
        .collect()
}

fn parse_ass(input: &str) -> Vec<SubtitleCue> {
    input
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let body = line.strip_prefix("Dialogue:")?.trim();
            let fields = body.splitn(10, ',').collect::<Vec<_>>();
            if fields.len() < 10 {
                return None;
            }
            let start = parse_timestamp(fields[1].trim())?;
            let end = parse_timestamp(fields[2].trim())?;
            let text = fields[9]
                .replace("\\N", "\n")
                .replace("\\n", "\n")
                .replace("{\\i1}", "")
                .replace("{\\i0}", "");
            Some(SubtitleCue { start, end, text })
        })
        .collect()
}

fn parse_time_range(value: &str) -> Option<(f64, f64)> {
    let mut parts = value.split("-->");
    let start = parse_timestamp(parts.next()?.trim())?;
    let end = parse_timestamp(parts.next()?.split_whitespace().next()?)?;
    Some((start, end))
}

fn parse_timestamp(value: &str) -> Option<f64> {
    let normalized = value.replace(',', ".");
    let parts = normalized.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [minutes, seconds] => {
            let minutes = minutes.parse::<f64>().ok()?;
            let seconds = seconds.parse::<f64>().ok()?;
            Some(minutes * 60.0 + seconds)
        }
        [hours, minutes, seconds] => {
            let hours = hours.parse::<f64>().ok()?;
            let minutes = minutes.parse::<f64>().ok()?;
            let seconds = seconds.parse::<f64>().ok()?;
            Some(hours * 3600.0 + minutes * 60.0 + seconds)
        }
        _ => None,
    }
}

fn subtitle_transform(frame: &RgbaImage, transform: Transform) -> Transform {
    if transform.width > 0.0 || transform.height > 0.0 || transform.x != 0.0 || transform.y != 0.0 {
        return transform;
    }
    Transform {
        x: frame.width() as f32 / 2.0,
        y: frame.height() as f32 * 0.82,
        width: frame.width() as f32 * 0.9,
        height: 80.0,
        scale: 1.0,
        rotation: 0.0,
        opacity: 1.0,
    }
}

fn resolved_subtitle_font_size(transform: Transform) -> f32 {
    if transform.height > 0.0 {
        (transform.height * 0.55).clamp(18.0, 72.0)
    } else {
        32.0
    }
}

fn alpha_blend(frame: &mut RgbaImage, overlay: &RgbaImage, x: i32, y: i32, opacity: f32) {
    let opacity = opacity.clamp(0.0, 1.0);
    for oy in 0..overlay.height() {
        for ox in 0..overlay.width() {
            let tx = x + ox as i32;
            let ty = y + oy as i32;
            if tx < 0 || ty < 0 || tx >= frame.width() as i32 || ty >= frame.height() as i32 {
                continue;
            }
            let source = overlay.get_pixel(ox, oy);
            let destination = frame.get_pixel_mut(tx as u32, ty as u32);
            let alpha = (f32::from(source[3]) / 255.0) * opacity;
            if alpha <= 0.0 {
                continue;
            }
            for channel in 0..3 {
                destination[channel] = ((f32::from(source[channel]) * alpha)
                    + (f32::from(destination[channel]) * (1.0 - alpha)))
                    .round() as u8;
            }
            destination[3] = (f32::from(source[3]) * opacity
                + f32::from(destination[3]) * (1.0 - alpha))
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mm_core::{
        AnimatedProperty, Animation, Asset, AssetKind, AssetMode, AudioLayer, Crop, Easing, Effect,
        GradientDirection, ImageLayer, Keyframe, Mask, ProjectSettings, SubtitleLayer,
        TextGradient, TextLayer, TextShadow, Track, TrackKind, Transition, TtsProviderKind,
        VideoLayer, VoiceLayer, WipeBorder, WipeShape,
    };
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn text_project() -> Project {
        Project {
            settings: ProjectSettings {
                title: "レンダー".to_string(),
                width: 320,
                height: 180,
                fps: 30,
                sample_rate: 48000,
                duration: 1.0,
                output: PathBuf::from("output/movie.mp4"),
                asset_mode: AssetMode::Copy,
                ffmpeg: None,
            },
            assets: vec![],
            tracks: vec![Track {
                id: "v1".to_string(),
                name: "V1".to_string(),
                kind: TrackKind::Video,
                layers: vec![Layer {
                    id: "text1".to_string(),
                    group_id: None,
                    start: 0.0,
                    duration: 1.0,
                    z_index: 1,
                    content: LayerContent::Text(TextLayer {
                        text: "Test\nTitle".to_string(),
                        font_asset_id: None,
                        font_size: 48.0,
                        color: "#ffffff".to_string(),
                        letter_spacing: 0.0,
                        line_spacing: 1.1,
                        stroke: None,
                        shadow: None,
                        gradient: None,
                        align: TextAlign::Center,
                    }),
                    transform: Transform {
                        x: 160.0,
                        y: 80.0,
                        width: 120.0,
                        height: 48.0,
                        scale: 1.0,
                        rotation: 0.0,
                        opacity: 1.0,
                    },
                    effects: vec![],
                    animations: vec![],
                    transition: None,
                }],
            }],
            scenes: vec![],
            groups: vec![],
            plugins: vec![],
        }
    }

    #[test]
    fn render_frame_draws_active_text_layer() -> Result<()> {
        let project = text_project();
        let options = RenderOptions::new(".", "output.mp4");

        let frame = render_frame(&project, &options, 0.5)?;

        assert!(has_non_background_pixel(&frame, options.background));
        Ok(())
    }

    #[test]
    fn render_frame_applies_text_gradient() -> Result<()> {
        let mut project = text_project();
        let LayerContent::Text(text) = &mut project.tracks[0].layers[0].content else {
            panic!("text layer ではありません");
        };
        text.gradient = Some(TextGradient {
            start_color: "#ff0000".to_string(),
            end_color: "#0000ff".to_string(),
            direction: GradientDirection::Vertical,
        });
        let mut options = RenderOptions::new(".", "output.mp4");
        options.background = Rgba([0, 0, 0, 255]);

        let frame = render_frame(&project, &options, 0.5)?;
        let text_pixels = frame
            .pixels()
            .filter(|pixel| **pixel != options.background)
            .collect::<Vec<_>>();

        assert!(text_pixels.iter().any(|pixel| pixel[0] > pixel[2]));
        assert!(text_pixels.iter().any(|pixel| pixel[2] > pixel[0]));
        Ok(())
    }

    #[test]
    fn render_frame_gpu_hybrid_draws_on_gpu_background_when_available() -> Result<()> {
        if !probe_gpu_backend().available {
            return Ok(());
        }
        let project = text_project();
        let mut options = RenderOptions::new(".", "output.mp4");
        options.background = Rgba([11, 22, 33, 255]);

        let frame = render_frame_gpu_hybrid(&project, &options, 2.0)?;

        assert_eq!(frame.get_pixel(0, 0), &options.background);
        Ok(())
    }

    #[test]
    fn render_frame_skia_draws_on_skia_background() -> Result<()> {
        let project = text_project();
        let mut options = RenderOptions::new(".", "output.mp4");
        options.background = Rgba([3, 6, 9, 255]);

        let frame = render_frame_skia(&project, &options, 0.5)?;

        assert_eq!(frame.get_pixel(0, 0), &options.background);
        assert!(has_non_background_pixel(&frame, options.background));
        Ok(())
    }

    #[test]
    fn gpu_background_frame_clears_texture_when_available() -> Result<()> {
        if !probe_gpu_backend().available {
            return Ok(());
        }

        let frame = gpu_background_frame(4, 3, Rgba([9, 18, 27, 255]))?;

        assert_eq!(frame.dimensions(), (4, 3));
        assert!(frame.pixels().all(|pixel| pixel == &Rgba([9, 18, 27, 255])));
        Ok(())
    }

    #[test]
    fn skia_background_frame_clears_surface() -> Result<()> {
        let frame = skia_background_frame(4, 3, Rgba([9, 18, 27, 255]))?;

        assert_eq!(frame.dimensions(), (4, 3));
        assert!(frame.pixels().all(|pixel| pixel == &Rgba([9, 18, 27, 255])));
        Ok(())
    }

    #[test]
    fn align_to_rounds_up_to_copy_alignment() {
        assert_eq!(align_to(4, GPU_COPY_ALIGNMENT), GPU_COPY_ALIGNMENT);
        assert_eq!(
            align_to(GPU_COPY_ALIGNMENT, GPU_COPY_ALIGNMENT),
            GPU_COPY_ALIGNMENT
        );
        assert_eq!(
            align_to(GPU_COPY_ALIGNMENT + 1, GPU_COPY_ALIGNMENT),
            GPU_COPY_ALIGNMENT * 2
        );
    }

    #[test]
    fn bundled_ffmpeg_locator_resolves_adjacent_binaries() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let ffmpeg_path = dir.path().join("ffmpeg");
        let ffprobe_path = dir.path().join("ffprobe");
        fs::write(&ffmpeg_path, [])?;
        fs::write(&ffprobe_path, [])?;
        let locator = BundledFfmpegLocator::new(dir.path());
        let project = text_project();

        assert_eq!(locator.ffmpeg_path(&project)?, ffmpeg_path);
        assert_eq!(locator.ffprobe_path()?, ffprobe_path);
        Ok(())
    }

    #[test]
    fn bundled_ffmpeg_locator_resolves_bin_dir_and_exe_names() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let bin_dir = dir.path().join("bin");
        fs::create_dir_all(&bin_dir)?;
        let ffmpeg_path = bin_dir.join("ffmpeg.exe");
        let ffprobe_path = bin_dir.join("ffprobe.exe");
        fs::write(&ffmpeg_path, [])?;
        fs::write(&ffprobe_path, [])?;
        let locator = BundledFfmpegLocator::new(dir.path());
        let project = text_project();

        assert_eq!(locator.ffmpeg_path(&project)?, ffmpeg_path);
        assert_eq!(locator.ffprobe_path()?, ffprobe_path);
        Ok(())
    }

    #[test]
    fn bundled_ffmpeg_locator_errors_when_missing() {
        let dir = tempfile::tempdir().expect("temp dir を作成できる");
        let locator = BundledFfmpegLocator::new(dir.path());

        let err = locator
            .ffprobe_path()
            .expect_err("ffprobe が無い場合は error");

        assert!(err.to_string().contains("同梱 ffprobe"));
    }

    #[test]
    fn resolve_layer_transform_applies_keyframes_and_fade() {
        let mut project = text_project();
        let layer = &mut project.tracks[0].layers[0];
        layer.effects.push(Effect::FadeIn { duration: 1.0 });
        layer.animations.push(Animation {
            property: AnimatedProperty::X,
            easing: Easing::Linear,
            keyframes: vec![
                Keyframe {
                    time: 0.0,
                    value: 10.0,
                },
                Keyframe {
                    time: 1.0,
                    value: 30.0,
                },
            ],
        });
        layer.animations.push(Animation {
            property: AnimatedProperty::Opacity,
            easing: Easing::EaseIn,
            keyframes: vec![
                Keyframe {
                    time: 0.0,
                    value: 0.2,
                },
                Keyframe {
                    time: 1.0,
                    value: 1.0,
                },
            ],
        });

        let transform = resolve_layer_transform(layer, 0.5);

        assert_eq!(transform.x, 20.0);
        assert!((transform.opacity - 0.2).abs() < 0.001);
    }

    #[test]
    fn resolve_layer_crop_applies_crop_keyframes() {
        let mut project = text_project();
        let layer = &mut project.tracks[0].layers[0];
        layer.animations.push(Animation {
            property: AnimatedProperty::CropX,
            easing: Easing::Linear,
            keyframes: vec![
                Keyframe {
                    time: 0.0,
                    value: 0.0,
                },
                Keyframe {
                    time: 1.0,
                    value: 4.0,
                },
            ],
        });
        layer.animations.push(Animation {
            property: AnimatedProperty::CropWidth,
            easing: Easing::Linear,
            keyframes: vec![
                Keyframe {
                    time: 0.0,
                    value: 8.0,
                },
                Keyframe {
                    time: 1.0,
                    value: 12.0,
                },
            ],
        });

        let crop = resolve_layer_crop(
            layer,
            Some(Crop {
                x: 0,
                y: 1,
                width: 8,
                height: 6,
            }),
            0.5,
        )
        .expect("crop が解決される");

        assert_eq!(crop.x, 2);
        assert_eq!(crop.y, 1);
        assert_eq!(crop.width, 10);
        assert_eq!(crop.height, 6);
    }

    #[test]
    fn transition_transform_applies_push_and_zoom() {
        let mut project = text_project();
        let layer = &mut project.tracks[0].layers[0];
        layer.transform.width = 100.0;
        layer.transition = Some(Transition::Push { duration: 1.0 });

        let pushed = resolve_layer_transform(layer, 0.25);

        assert!((pushed.x - 85.0).abs() < 0.001);

        layer.transition = Some(Transition::Zoom { duration: 1.0 });
        let zoomed = resolve_layer_transform(layer, 0.5);

        assert!((zoomed.scale - 0.925).abs() < 0.001);
    }

    #[test]
    fn transition_frame_applies_crossfade_wipe_blur_and_flash() {
        let mut project = text_project();
        let layer = &mut project.tracks[0].layers[0];
        layer.duration = 2.0;
        let mut frame = RgbaImage::from_pixel(16, 16, Rgba([20, 20, 20, 255]));
        layer.transition = Some(Transition::CrossFade { duration: 1.0 });
        apply_transition_to_frame(&mut frame, layer, 0.5);
        assert_eq!(frame.get_pixel(0, 0)[3], 128);

        let mut wipe_frame = RgbaImage::from_pixel(16, 16, Rgba([20, 20, 20, 255]));
        layer.transition = Some(Transition::Wipe {
            duration: 1.0,
            shape: WipeShape::Circle,
        });
        apply_transition_to_frame(&mut wipe_frame, layer, 0.2);
        assert_eq!(wipe_frame.get_pixel(0, 0)[3], 0);
        assert_eq!(wipe_frame.get_pixel(8, 8)[3], 255);

        let mut blur_frame = RgbaImage::from_pixel(4, 1, Rgba([0, 0, 0, 255]));
        blur_frame.put_pixel(0, 0, Rgba([255, 255, 255, 255]));
        layer.transition = Some(Transition::Blur { duration: 1.0 });
        apply_transition_to_frame(&mut blur_frame, layer, 0.1);
        assert!(blur_frame.get_pixel(1, 0)[0] > 0);

        let mut flash_frame = RgbaImage::from_pixel(1, 1, Rgba([20, 20, 20, 255]));
        layer.transition = Some(Transition::Flash { duration: 1.0 });
        apply_transition_to_frame(&mut flash_frame, layer, 0.1);
        assert!(flash_frame.get_pixel(0, 0)[0] > 20);
    }

    #[test]
    fn rounded_rect_wipe_draws_border_and_shadow() {
        let mut project = text_project();
        let layer = &mut project.tracks[0].layers[0];
        layer.duration = 2.0;

        let mut border_frame = RgbaImage::from_pixel(20, 20, Rgba([20, 20, 20, 255]));
        layer.transition = Some(Transition::Wipe {
            duration: 1.0,
            shape: WipeShape::RoundedRect {
                radius: 2.0,
                border: Some(WipeBorder {
                    color: "#ff0000".to_string(),
                    width: 2.0,
                }),
                shadow: None,
            },
        });
        apply_transition_to_frame(&mut border_frame, layer, 0.5);
        assert_eq!(*border_frame.get_pixel(5, 10), Rgba([255, 0, 0, 255]));
        assert_eq!(border_frame.get_pixel(0, 0)[3], 0);

        let mut shadow_frame = RgbaImage::from_pixel(20, 20, Rgba([20, 20, 20, 255]));
        layer.transition = Some(Transition::Wipe {
            duration: 1.0,
            shape: WipeShape::RoundedRect {
                radius: 2.0,
                border: None,
                shadow: Some(TextShadow {
                    color: "#0000ff".to_string(),
                    offset_x: 3.0,
                    offset_y: 0.0,
                    blur: 4.0,
                }),
            },
        });
        apply_transition_to_frame(&mut shadow_frame, layer, 0.5);
        assert_eq!(shadow_frame.get_pixel(16, 10)[2], 255);
        assert!(shadow_frame.get_pixel(16, 10)[3] > 0);
    }

    #[test]
    fn image_effects_adjust_pixels() {
        let image = RgbaImage::from_pixel(4, 4, Rgba([100, 50, 25, 255]));

        let adjusted = apply_image_effects(
            image,
            &[
                Effect::Brightness { amount: 0.1 },
                Effect::Saturation { amount: 0.0 },
                Effect::Pixelate { size: 2 },
            ],
        );

        assert_eq!(adjusted.dimensions(), (4, 4));
        let pixel = adjusted.get_pixel(0, 0);
        assert_eq!(pixel[0], pixel[1]);
        assert_eq!(pixel[1], pixel[2]);
    }

    #[test]
    fn fit_dimensions_respects_modes() {
        assert_eq!(
            fit_dimensions(100, 50, 200, 200, FitMode::Contain),
            (200, 100)
        );
        assert_eq!(
            fit_dimensions(100, 50, 200, 200, FitMode::Cover),
            (400, 200)
        );
        assert_eq!(
            fit_dimensions(100, 50, 200, 200, FitMode::Stretch),
            (200, 200)
        );
        assert_eq!(
            fit_dimensions(100, 50, 200, 200, FitMode::BlurBackground),
            (200, 100)
        );
    }

    #[test]
    fn crop_mask_rotation_and_motion_blur_adjust_image() {
        let mut image = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]));
        for y in 0..8 {
            for x in 0..8 {
                image.put_pixel(x, y, Rgba([(x * 30) as u8, (y * 30) as u8, 100, 255]));
            }
        }

        let cropped = apply_crop(
            image,
            Some(Crop {
                x: 2,
                y: 1,
                width: 4,
                height: 5,
            }),
        );
        let masked = apply_mask(cropped, Some(&Mask::Circle), Path::new(".")).unwrap();
        let blurred = apply_image_effects(masked, &[Effect::MotionBlur { amount: 2.0 }]);
        let rotated = rotate_image(&blurred, 45.0);

        assert_eq!(blurred.dimensions(), (4, 5));
        assert_eq!(blurred.get_pixel(0, 0)[3], 0);
        assert!(rotated.width() > blurred.width());
        assert!(rotated.height() > blurred.height());
    }

    #[test]
    fn svg_mask_applies_alpha_from_white_shape() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mask_path = dir.path().join("media/mask/circle.svg");
        std::fs::create_dir_all(mask_path.parent().unwrap())?;
        std::fs::write(
            &mask_path,
            r##"<svg width="10" height="10" xmlns="http://www.w3.org/2000/svg">
  <rect x="0" y="0" width="10" height="10" fill="#000000"/>
  <circle cx="5" cy="5" r="3" fill="#ffffff"/>
</svg>"##,
        )?;
        let image = RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 255]));

        let masked = apply_mask(
            image,
            Some(&Mask::Svg {
                path: PathBuf::from("media/mask/circle.svg"),
            }),
            dir.path(),
        )?;

        assert_eq!(masked.get_pixel(5, 5)[3], 255);
        assert_eq!(masked.get_pixel(0, 0)[3], 0);
        Ok(())
    }

    #[test]
    fn svg_mask_reports_missing_file() {
        let image = RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 255]));

        let error = apply_mask(
            image,
            Some(&Mask::Svg {
                path: PathBuf::from("missing.svg"),
            }),
            Path::new("."),
        )
        .unwrap_err();

        assert!(error.to_string().contains("SVG mask を読み込めません"));
    }

    #[test]
    fn render_frame_draws_blur_background_fit_for_image() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let image_path = dir.path().join("media/image/wide.png");
        std::fs::create_dir_all(image_path.parent().unwrap())?;
        let mut image = RgbaImage::from_pixel(8, 4, Rgba([0, 0, 255, 255]));
        for y in 0..4 {
            for x in 0..4 {
                image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        image.save(&image_path)?;
        let mut project = text_project();
        project.settings.width = 32;
        project.settings.height = 32;
        project.assets.push(Asset {
            id: "wide".to_string(),
            kind: AssetKind::Image,
            path: PathBuf::from("media/image/wide.png"),
        });
        project.tracks[0].layers[0].content = LayerContent::Image(ImageLayer {
            asset_id: "wide".to_string(),
            crop: None,
            mask: None,
            fit: Some(FitMode::BlurBackground),
        });
        project.tracks[0].layers[0].transform = Transform {
            x: 0.0,
            y: 0.0,
            width: 32.0,
            height: 32.0,
            scale: 1.0,
            rotation: 0.0,
            opacity: 1.0,
        };
        let mut options = RenderOptions::new(dir.path(), "output.mp4");
        options.background = Rgba([0, 0, 0, 255]);

        let frame = render_frame(&project, &options, 0.5)?;

        assert_ne!(frame.get_pixel(0, 0), &options.background);
        assert!(frame.get_pixel(8, 16)[0] > frame.get_pixel(8, 16)[2]);
        assert!(frame.get_pixel(24, 16)[2] > frame.get_pixel(24, 16)[0]);
        Ok(())
    }

    #[test]
    fn video_filter_includes_crop_when_present() {
        let project = text_project();
        let layer = Layer {
            id: "video".to_string(),
            group_id: None,
            start: 0.5,
            duration: 1.0,
            z_index: 0,
            content: LayerContent::Video(VideoLayer {
                asset_id: "video".to_string(),
                crop: Some(Crop {
                    x: 3,
                    y: 4,
                    width: 20,
                    height: 10,
                }),
                trim_start: Some(0.2),
                trim_end: Some(0.8),
                fit: None,
            }),
            transform: Transform {
                x: 0.0,
                y: 0.0,
                width: 64.0,
                height: 32.0,
                scale: 1.0,
                rotation: 0.0,
                opacity: 1.0,
            },
            effects: vec![],
            animations: vec![],
            transition: None,
        };
        let LayerContent::Video(content) = &layer.content else {
            unreachable!();
        };
        let planned = PlannedVideoLayer::new(&project, &layer, content, PathBuf::from("video.mp4"));

        let filter = planned.video_filter(1, "v");

        assert!(filter.contains("crop=20:10:3:4"));
        assert!(filter.contains("scale=64:32"));
    }

    #[test]
    fn render_frame_applies_image_fade_effect() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let image_path = dir.path().join("media/image/pixel.png");
        std::fs::create_dir_all(image_path.parent().unwrap())?;
        RgbaImage::from_pixel(8, 8, Rgba([255, 255, 255, 255])).save(&image_path)?;
        let mut project = text_project();
        project.assets.push(Asset {
            id: "pixel".to_string(),
            kind: AssetKind::Image,
            path: PathBuf::from("media/image/pixel.png"),
        });
        project.tracks[0].layers[0].content = LayerContent::Image(ImageLayer {
            asset_id: "pixel".to_string(),
            crop: None,
            mask: None,
            fit: None,
        });
        project.tracks[0].layers[0].transform = Transform {
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 8.0,
            scale: 1.0,
            rotation: 0.0,
            opacity: 1.0,
        };
        project.tracks[0].layers[0]
            .effects
            .push(Effect::FadeIn { duration: 1.0 });
        let mut options = RenderOptions::new(dir.path(), "output.mp4");
        options.background = Rgba([0, 0, 0, 255]);

        let early = render_frame(&project, &options, 0.25)?;
        let late = render_frame(&project, &options, 0.75)?;

        assert!(early.get_pixel(0, 0)[0] < late.get_pixel(0, 0)[0]);
        Ok(())
    }

    #[test]
    fn render_frame_skips_inactive_layer() -> Result<()> {
        let mut project = text_project();
        project.tracks[0].layers[0].start = 2.0;
        let options = RenderOptions::new(".", "output.mp4");

        let frame = render_frame(&project, &options, 0.5)?;

        assert_eq!(frame.get_pixel(160, 90), &options.background);
        Ok(())
    }

    #[test]
    fn locator_prefers_project_ffmpeg_path() -> Result<()> {
        let mut project = text_project();
        project.settings.ffmpeg = Some(PathBuf::from("/tmp/ffmpeg"));

        let path = SystemFfmpegLocator.ffmpeg_path(&project)?;

        assert_eq!(path, PathBuf::from("/tmp/ffmpeg"));
        Ok(())
    }

    #[test]
    fn parse_srt_subtitle_cues() {
        let cues = parse_subtitles(
            "1\n00:00:00,000 --> 00:00:01,500\nHello\n\n2\n00:00:02,000 --> 00:00:03,000\nWorld\n",
        );

        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start, 0.0);
        assert_eq!(cues[0].end, 1.5);
        assert_eq!(cues[0].text, "Hello");
    }

    #[test]
    fn parse_vtt_subtitle_cues() {
        let cues = parse_subtitles("WEBVTT\n\n00:00:00.500 --> 00:00:01.000\nHello VTT\n");

        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].start, 0.5);
        assert_eq!(cues[0].text, "Hello VTT");
    }

    #[test]
    fn parse_ass_subtitle_cues() {
        let cues = parse_subtitles(
            "[Events]\nDialogue: 0,0:00:01.00,0:00:02.50,Default,,0,0,0,,Hello\\NASS\n",
        );

        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].start, 1.0);
        assert_eq!(cues[0].end, 2.5);
        assert_eq!(cues[0].text, "Hello\nASS");
    }

    #[test]
    fn render_frame_draws_active_subtitle() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let subtitle_path = dir.path().join("media/subtitle/main.srt");
        std::fs::create_dir_all(subtitle_path.parent().unwrap())?;
        std::fs::write(
            &subtitle_path,
            "1\n00:00:00,000 --> 00:00:01,000\nSubtitle text\n",
        )?;
        let mut project = text_project();
        project.assets.push(Asset {
            id: "subtitle".to_string(),
            kind: AssetKind::Subtitle,
            path: PathBuf::from("media/subtitle/main.srt"),
        });
        project.tracks[0].layers[0].content = LayerContent::Subtitle(SubtitleLayer {
            asset_id: "subtitle".to_string(),
        });
        project.tracks[0].layers[0].transform = Transform {
            x: 160.0,
            y: 120.0,
            width: 260.0,
            height: 48.0,
            scale: 1.0,
            rotation: 0.0,
            opacity: 1.0,
        };
        let options = RenderOptions::new(dir.path(), "output.mp4");

        let frame = render_frame(&project, &options, 0.5)?;

        assert!(has_non_background_pixel(&frame, options.background));
        Ok(())
    }

    #[test]
    fn render_frame_draws_video_layer_from_ffmpeg_frame() -> Result<()> {
        let Some(ffmpeg) = available_ffmpeg() else {
            return Ok(());
        };
        let dir = tempfile::tempdir()?;
        let video_path = dir.path().join("media/video/red.mp4");
        std::fs::create_dir_all(video_path.parent().unwrap())?;
        let status = Command::new(&ffmpeg)
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:size=32x24:rate=10:duration=1",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&video_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !status.success() {
            return Ok(());
        }
        let project = video_project(
            PathBuf::from("media/video/red.mp4"),
            VideoLayer {
                asset_id: "video".to_string(),
                crop: None,
                trim_start: Some(0.0),
                trim_end: Some(1.0),
                fit: None,
            },
        );
        let mut options = RenderOptions::new(dir.path(), "output.mp4");
        options.ffmpeg_path = Some(ffmpeg);
        options.background = Rgba([0, 0, 0, 255]);

        let frame = render_frame(&project, &options, 0.2)?;
        let pixel = frame.get_pixel(16, 12);

        assert!(pixel[0] > 180);
        assert!(pixel[1] < 80);
        assert!(pixel[2] < 80);
        Ok(())
    }

    #[test]
    fn render_frame_applies_video_trim_and_crop() -> Result<()> {
        let Some(ffmpeg) = available_ffmpeg() else {
            return Ok(());
        };
        let dir = tempfile::tempdir()?;
        let video_path = dir.path().join("media/video/trim-crop.mp4");
        std::fs::create_dir_all(video_path.parent().unwrap())?;
        let status = Command::new(&ffmpeg)
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:size=16x16:rate=10:duration=0.5",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:size=16x16:rate=10:duration=0.5",
                "-filter_complex",
                "[0:v][1:v]concat=n=2:v=1:a=0,format=yuv420p[v]",
                "-map",
                "[v]",
            ])
            .arg(&video_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !status.success() {
            return Ok(());
        }
        let project = video_project(
            PathBuf::from("media/video/trim-crop.mp4"),
            VideoLayer {
                asset_id: "video".to_string(),
                crop: Some(Crop {
                    x: 4,
                    y: 4,
                    width: 8,
                    height: 8,
                }),
                trim_start: Some(0.6),
                trim_end: Some(0.9),
                fit: Some(FitMode::Stretch),
            },
        );
        let mut options = RenderOptions::new(dir.path(), "output.mp4");
        options.ffmpeg_path = Some(ffmpeg);
        options.background = Rgba([0, 0, 0, 255]);

        let frame = render_frame(&project, &options, 0.0)?;
        let pixel = frame.get_pixel(16, 12);

        assert!(pixel[2] > 120);
        assert!(pixel[0] < 100);
        Ok(())
    }

    #[test]
    fn media_plan_synthesizes_voice_layer_to_cached_wav() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let endpoint = start_voicevox_mock(mm_core::encode_wav_audio(&mm_core::AudioBuffer {
            sample_rate: 24_000,
            channels: 1,
            samples: vec![0.0, 0.4, -0.4, 0.0],
        }));
        env::set_var("MM_VOICEVOX_ENDPOINT", endpoint);
        let project = Project {
            settings: ProjectSettings {
                title: "音声合成".to_string(),
                width: 64,
                height: 64,
                fps: 10,
                sample_rate: 48000,
                duration: 0.5,
                output: PathBuf::from("output/movie.mp4"),
                asset_mode: AssetMode::Copy,
                ffmpeg: None,
            },
            assets: vec![],
            tracks: vec![Track {
                id: "a1".to_string(),
                name: "A1".to_string(),
                kind: TrackKind::Audio,
                layers: vec![Layer {
                    id: "voice".to_string(),
                    group_id: None,
                    start: 0.1,
                    duration: 0.4,
                    z_index: 0,
                    content: LayerContent::Voice(VoiceLayer {
                        provider: TtsProviderKind::Voicevox,
                        speaker: "1".to_string(),
                        text: "こんにちは".to_string(),
                        speed: 1.0,
                        pitch: 0.0,
                        emotion: None,
                    }),
                    transform: Transform::default(),
                    effects: vec![],
                    animations: vec![],
                    transition: None,
                }],
            }],
            scenes: vec![],
            groups: vec![],
            plugins: vec![],
        };
        let options = RenderOptions::new(dir.path(), "output.mp4");

        let plan = MediaPlan::new(&project, &options)?;

        env::remove_var("MM_VOICEVOX_ENDPOINT");
        assert_eq!(plan.audio_layers.len(), 1);
        assert!(plan.audio_layers[0].path.exists());
        assert!(plan.audio_layers[0]
            .path
            .starts_with(dir.path().join("cache/tts")));
        assert_eq!(plan.audio_layers[0].start, 0.1);
        assert_eq!(plan.audio_layers[0].trim_start, 0.0);
        assert_eq!(plan.audio_layers[0].trim_end, Some(0.4));
        Ok(())
    }

    #[test]
    fn render_project_muxes_video_and_audio_layers_when_ffmpeg_is_available() -> Result<()> {
        let Ok(ffmpeg) = which::which("ffmpeg") else {
            return Ok(());
        };
        let Ok(ffprobe) = which::which("ffprobe") else {
            return Ok(());
        };
        let dir = tempfile::tempdir()?;
        let video_path = dir.path().join("media/video/source.mp4");
        let audio_path = dir.path().join("media/audio/tone.wav");
        std::fs::create_dir_all(video_path.parent().unwrap())?;
        std::fs::create_dir_all(audio_path.parent().unwrap())?;
        let video_status = Command::new(&ffmpeg)
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=64x64:rate=10:duration=0.5",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&video_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !video_status.success() {
            return Ok(());
        }
        let audio_status = Command::new(&ffmpeg)
            .args(["-y", "-f", "lavfi", "-i", "sine=frequency=440:duration=0.5"])
            .arg(&audio_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !audio_status.success() {
            return Ok(());
        }
        let output_path = dir.path().join("output/movie.mp4");
        let project = Project {
            settings: ProjectSettings {
                title: "メディア".to_string(),
                width: 64,
                height: 64,
                fps: 10,
                sample_rate: 48000,
                duration: 0.5,
                output: PathBuf::from("output/movie.mp4"),
                asset_mode: AssetMode::Copy,
                ffmpeg: Some(ffmpeg),
            },
            assets: vec![
                Asset {
                    id: "video".to_string(),
                    kind: AssetKind::Video,
                    path: PathBuf::from("media/video/source.mp4"),
                },
                Asset {
                    id: "audio".to_string(),
                    kind: AssetKind::Audio,
                    path: PathBuf::from("media/audio/tone.wav"),
                },
            ],
            tracks: vec![
                Track {
                    id: "v1".to_string(),
                    name: "V1".to_string(),
                    kind: TrackKind::Video,
                    layers: vec![
                        Layer {
                            id: "video".to_string(),
                            group_id: None,
                            start: 0.0,
                            duration: 0.5,
                            z_index: 0,
                            content: LayerContent::Video(VideoLayer {
                                asset_id: "video".to_string(),
                                crop: None,
                                trim_start: Some(0.0),
                                trim_end: Some(0.5),
                                fit: None,
                            }),
                            transform: Transform {
                                x: 0.0,
                                y: 0.0,
                                width: 64.0,
                                height: 64.0,
                                scale: 1.0,
                                rotation: 0.0,
                                opacity: 1.0,
                            },
                            effects: vec![],
                            animations: vec![],
                            transition: None,
                        },
                        Layer {
                            id: "label".to_string(),
                            group_id: None,
                            start: 0.0,
                            duration: 0.5,
                            z_index: 10,
                            content: LayerContent::Text(TextLayer {
                                text: "mm".to_string(),
                                font_asset_id: None,
                                font_size: 18.0,
                                color: "#ffffff".to_string(),
                                letter_spacing: 0.0,
                                line_spacing: 1.0,
                                stroke: None,
                                shadow: None,
                                gradient: None,
                                align: TextAlign::Center,
                            }),
                            transform: Transform {
                                x: 32.0,
                                y: 28.0,
                                width: 64.0,
                                height: 20.0,
                                scale: 1.0,
                                rotation: 0.0,
                                opacity: 1.0,
                            },
                            effects: vec![],
                            animations: vec![],
                            transition: None,
                        },
                    ],
                },
                Track {
                    id: "a1".to_string(),
                    name: "A1".to_string(),
                    kind: TrackKind::Audio,
                    layers: vec![Layer {
                        id: "audio".to_string(),
                        group_id: None,
                        start: 0.0,
                        duration: 0.5,
                        z_index: 0,
                        content: LayerContent::Audio(AudioLayer {
                            asset_id: "audio".to_string(),
                            trim_start: Some(0),
                            trim_end: Some(500),
                        }),
                        transform: Transform::default(),
                        effects: vec![],
                        animations: vec![],
                        transition: None,
                    }],
                },
            ],
            scenes: vec![],
            groups: vec![],
            plugins: vec![],
        };
        let options = RenderOptions::new(dir.path(), &output_path);

        render_project(&project, &options)?;

        let stream_output = Command::new(ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_type",
                "-of",
                "csv=p=0",
            ])
            .arg(&output_path)
            .output()?;
        let streams = String::from_utf8_lossy(&stream_output.stdout);
        assert!(streams.lines().any(|line| line == "video"));
        assert!(streams.lines().any(|line| line == "audio"));
        Ok(())
    }

    fn available_ffmpeg() -> Option<PathBuf> {
        which::which("ffmpeg").ok()
    }

    fn video_project(path: PathBuf, content: VideoLayer) -> Project {
        Project {
            settings: ProjectSettings {
                title: "動画プレビュー".to_string(),
                width: 32,
                height: 24,
                fps: 10,
                sample_rate: 48000,
                duration: 1.0,
                output: PathBuf::from("output/movie.mp4"),
                asset_mode: AssetMode::Copy,
                ffmpeg: None,
            },
            assets: vec![Asset {
                id: "video".to_string(),
                kind: AssetKind::Video,
                path,
            }],
            tracks: vec![Track {
                id: "v1".to_string(),
                name: "V1".to_string(),
                kind: TrackKind::Video,
                layers: vec![Layer {
                    id: "video".to_string(),
                    group_id: None,
                    start: 0.0,
                    duration: 1.0,
                    z_index: 0,
                    content: LayerContent::Video(content),
                    transform: Transform {
                        x: 0.0,
                        y: 0.0,
                        width: 32.0,
                        height: 24.0,
                        scale: 1.0,
                        rotation: 0.0,
                        opacity: 1.0,
                    },
                    effects: vec![],
                    animations: vec![],
                    transition: None,
                }],
            }],
            scenes: vec![],
            groups: vec![],
            plugins: vec![],
        }
    }

    fn has_non_background_pixel(frame: &RgbaImage, background: Rgba<u8>) -> bool {
        frame.pixels().any(|pixel| pixel != &background)
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
