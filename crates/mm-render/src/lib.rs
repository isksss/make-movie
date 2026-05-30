use ab_glyph::{point, Font, FontArc, GlyphId, PxScale, ScaleFont};
use anyhow::{bail, Context, Result};
use image::{imageops, Rgba, RgbaImage};
use mm_core::{
    AssetKind, Layer, LayerContent, Project, SubtitleLayer, TextAlign, TextLayer, TextShadow,
    TextStroke, Transform,
};
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

pub fn render_project(project: &Project, options: &RenderOptions) -> Result<()> {
    let active_backend = match options.backend {
        RenderBackend::Cpu => ActiveRenderBackend::Cpu,
        RenderBackend::Auto => match GpuFrameRenderer::new() {
            Ok(renderer) => ActiveRenderBackend::GpuHybrid(renderer),
            Err(_) => ActiveRenderBackend::Cpu,
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

    let mut child = Command::new(ffmpeg)
        .args([
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
            &format!("{width}x{height}"),
            "-r",
            &fps.to_string(),
            "-i",
            "-",
            "-an",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&options.output_path)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("ffmpeg を起動できません: {}", ffmpeg.display()))?;

    {
        let stdin = child.stdin.as_mut().context("ffmpeg stdin を開けません")?;
        for frame_index in 0..frame_count {
            let time = frame_index as f64 / f64::from(fps);
            let frame = match &backend {
                ActiveRenderBackend::Cpu => render_frame(project, options, time)?,
                ActiveRenderBackend::GpuHybrid(renderer) => {
                    render_frame_gpu_hybrid_with_renderer(project, options, time, renderer)?
                }
            };
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

pub fn render_frame(project: &Project, options: &RenderOptions, time: f64) -> Result<RgbaImage> {
    let frame = RgbaImage::from_pixel(
        project.settings.width,
        project.settings.height,
        options.background,
    );
    render_frame_on_base(project, options, time, frame)
}

pub fn render_frame_gpu_hybrid(
    project: &Project,
    options: &RenderOptions,
    time: f64,
) -> Result<RgbaImage> {
    let renderer = GpuFrameRenderer::new()?;
    render_frame_gpu_hybrid_with_renderer(project, options, time, &renderer)
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
    render_frame_on_base(project, options, time, frame)
}

fn render_frame_on_base(
    project: &Project,
    options: &RenderOptions,
    time: f64,
    mut frame: RgbaImage,
) -> Result<RgbaImage> {
    let mut layers = project
        .tracks
        .iter()
        .flat_map(|track| track.layers.iter())
        .filter(|layer| layer.start <= time && time < layer.start + layer.duration)
        .collect::<Vec<_>>();
    layers.sort_by_key(|layer| layer.z_index);

    for layer in layers {
        draw_layer(project, options, &mut frame, layer, time)?;
    }
    Ok(frame)
}

pub fn gpu_background_frame(width: u32, height: u32, color: Rgba<u8>) -> Result<RgbaImage> {
    GpuFrameRenderer::new()?.background_frame(width, height, color)
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
            draw_image(frame, &image, layer.transform);
        }
        LayerContent::Text(content) => {
            let font = load_font(project, options, content.font_asset_id.as_deref())?;
            draw_text_layer(frame, content, layer.transform, &font);
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
                    align: TextAlign::Center,
                };
                draw_text_layer(
                    frame,
                    &subtitle,
                    subtitle_transform(frame, layer.transform),
                    &font,
                );
            }
        }
        LayerContent::Video(_) | LayerContent::Audio(_) | LayerContent::Voice(_) => {}
    }
    Ok(())
}

fn draw_image(frame: &mut RgbaImage, image: &RgbaImage, transform: Transform) {
    let width = resolved_size(transform.width, image.width());
    let height = resolved_size(transform.height, image.height());
    let resized = imageops::resize(image, width, height, imageops::FilterType::Lanczos3);
    alpha_blend(
        frame,
        &resized,
        transform.x.round() as i32,
        transform.y.round() as i32,
        transform.opacity,
    );
}

fn resolved_size(value: f32, fallback: u32) -> u32 {
    if value > 0.0 {
        value.round().max(1.0) as u32
    } else {
        fallback
    }
}

fn draw_text_layer(frame: &mut RgbaImage, text: &TextLayer, transform: Transform, font: &FontArc) {
    let scale = PxScale::from(text.font_size * transform.scale.max(0.01));
    let scaled = font.as_scaled(scale);
    let line_height =
        (scaled.ascent() - scaled.descent() + scaled.line_gap()) * text.line_spacing.max(0.1);
    let lines = text.text.lines().collect::<Vec<_>>();
    let base_color = parse_color(&text.color).unwrap_or(Rgba([255, 255, 255, 255]));
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
            color,
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
                    color,
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
        base_color,
        transform.opacity,
    );
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
    color: Rgba<u8>,
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
            color,
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
    color: Rgba<u8>,
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
                blend_pixel(frame, pixel_x, pixel_y, color, coverage * opacity);
            });
        }
        pen_x += scaled.h_advance(glyph_id) + letter_spacing;
    }
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
            for channel in 0..3 {
                destination[channel] = ((f32::from(source[channel]) * alpha)
                    + (f32::from(destination[channel]) * (1.0 - alpha)))
                    .round() as u8;
            }
            destination[3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mm_core::{
        Asset, AssetKind, AssetMode, ProjectSettings, SubtitleLayer, TextLayer, Track, TrackKind,
    };

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

    fn has_non_background_pixel(frame: &RgbaImage, background: Rgba<u8>) -> bool {
        frame.pixels().any(|pixel| pixel != &background)
    }
}
