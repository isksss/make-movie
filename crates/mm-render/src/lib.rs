use anyhow::{bail, Context, Result};
use image::{imageops, Rgba, RgbaImage};
use mm_core::{Layer, LayerContent, Project, TextAlign, Transform};
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub project_root: PathBuf,
    pub output_path: PathBuf,
    pub ffmpeg_path: Option<PathBuf>,
    pub background: Rgba<u8>,
}

impl RenderOptions {
    pub fn new(project_root: impl Into<PathBuf>, output_path: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            output_path: output_path.into(),
            ffmpeg_path: None,
            background: Rgba([20, 20, 24, 255]),
        }
    }
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
    let ffmpeg = match &options.ffmpeg_path {
        Some(path) => path.clone(),
        None => SystemFfmpegLocator.ffmpeg_path(project)?,
    };
    encode_with_ffmpeg(project, options, &ffmpeg)
}

fn encode_with_ffmpeg(project: &Project, options: &RenderOptions, ffmpeg: &Path) -> Result<()> {
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
            let frame = render_frame(project, options, time)?;
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
    let mut frame = RgbaImage::from_pixel(
        project.settings.width,
        project.settings.height,
        options.background,
    );
    let mut layers = project
        .tracks
        .iter()
        .flat_map(|track| track.layers.iter())
        .filter(|layer| layer.start <= time && time < layer.start + layer.duration)
        .collect::<Vec<_>>();
    layers.sort_by_key(|layer| layer.z_index);

    for layer in layers {
        draw_layer(project, options, &mut frame, layer)?;
    }
    Ok(frame)
}

fn draw_layer(
    project: &Project,
    options: &RenderOptions,
    frame: &mut RgbaImage,
    layer: &Layer,
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
            draw_text_placeholder(frame, &content.text, content.align, layer.transform);
        }
        LayerContent::Subtitle(content) => {
            draw_text_placeholder(frame, &content.asset_id, TextAlign::Center, layer.transform);
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

fn draw_text_placeholder(
    frame: &mut RgbaImage,
    text: &str,
    align: TextAlign,
    transform: Transform,
) {
    let width = resolved_size(
        transform.width,
        (text.chars().count() as u32).saturating_mul(18).max(120),
    );
    let height = resolved_size(transform.height, 64);
    let x = match align {
        TextAlign::Left => transform.x.round() as i32,
        TextAlign::Center => transform.x.round() as i32 - (width as i32 / 2),
        TextAlign::Right => transform.x.round() as i32 - width as i32,
    };
    let y = transform.y.round() as i32;
    let mut rect = RgbaImage::from_pixel(
        width,
        height,
        Rgba([245, 245, 245, (220.0 * transform.opacity) as u8]),
    );
    for (index, byte) in text.bytes().enumerate() {
        let marker_x = 8 + (index as u32 * 9) % width.saturating_sub(8).max(1);
        let marker_y = 8 + ((index as u32 * 9) / width.saturating_sub(8).max(1)) * 9;
        if marker_y + 6 < height {
            let color = Rgba([byte, 80, 180, 255]);
            fill_rect(&mut rect, marker_x, marker_y, 6, 6, color);
        }
    }
    alpha_blend(frame, &rect, x, y, transform.opacity);
}

fn fill_rect(image: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, color: Rgba<u8>) {
    for px in x..x.saturating_add(width).min(image.width()) {
        for py in y..y.saturating_add(height).min(image.height()) {
            image.put_pixel(px, py, color);
        }
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
    use mm_core::{AssetMode, ProjectSettings, TextLayer, Track, TrackKind};

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
                        text: "テスト".to_string(),
                        font_size: 48.0,
                        color: "#ffffff".to_string(),
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

        assert_ne!(frame.get_pixel(160, 90), &options.background);
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
}
