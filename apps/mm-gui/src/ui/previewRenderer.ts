import { layerTransform } from "../types";
import type { LayerTransform, PreviewState, ProjectState, TimelineLayer } from "../types";

export interface PreviewViewport {
  width: number;
  height: number;
  scale: number;
  offsetX: number;
  offsetY: number;
}

export interface PreviewDrawItem {
  layer: TimelineLayer;
  color: string;
  lane: number;
}

export function createPreviewViewport(
  canvasWidth: number,
  canvasHeight: number,
  projectWidth: number,
  projectHeight: number,
): PreviewViewport {
  const scale = Math.min(canvasWidth / projectWidth, canvasHeight / projectHeight);
  const width = projectWidth * scale;
  const height = projectHeight * scale;
  return {
    width,
    height,
    scale,
    offsetX: (canvasWidth - width) / 2,
    offsetY: (canvasHeight - height) / 2,
  };
}

export function activePreviewItems(project: ProjectState, time: number): PreviewDrawItem[] {
  return project.layers
    .filter((layer) => layer.start <= time && time < layer.start + layer.duration)
    .sort((a, b) => a.zIndex - b.zIndex)
    .map((layer, index) => ({
      layer,
      color: colorForLayer(layer),
      lane: index,
    }));
}

export function colorForLayer(layer: TimelineLayer): string {
  switch (layer.contentKind) {
    case "text":
      return "#58a9b8";
    case "subtitle":
      return "#f3d36b";
    case "image":
      return "#79b97a";
    case "video":
      return "#7f9cf5";
    case "voice":
      return "#d18ce0";
    case "audio":
      return "#ec8a65";
  }
}

export function drawPreviewFrame(
  context: CanvasRenderingContext2D,
  project: ProjectState,
  preview: PreviewState,
) {
  const canvas = context.canvas;
  const viewport = createPreviewViewport(
    canvas.width,
    canvas.height,
    project.settings.width,
    project.settings.height,
  );
  context.clearRect(0, 0, canvas.width, canvas.height);
  drawCheckerboard(context, canvas.width, canvas.height);
  drawProjectFrame(context, viewport);
  drawLayers(context, project, viewport, activePreviewItems(project, preview.currentTime));
  drawHud(context, project, preview, viewport);
}

function drawCheckerboard(context: CanvasRenderingContext2D, width: number, height: number) {
  context.fillStyle = "#101417";
  context.fillRect(0, 0, width, height);
  context.fillStyle = "#151d22";
  const size = 18;
  for (let y = 0; y < height; y += size) {
    for (let x = (y / size) % 2 === 0 ? 0 : size; x < width; x += size * 2) {
      context.fillRect(x, y, size, size);
    }
  }
}

function drawProjectFrame(context: CanvasRenderingContext2D, viewport: PreviewViewport) {
  context.fillStyle = "#182126";
  context.fillRect(viewport.offsetX, viewport.offsetY, viewport.width, viewport.height);
  context.strokeStyle = "#58a9b8";
  context.lineWidth = 2;
  context.strokeRect(viewport.offsetX, viewport.offsetY, viewport.width, viewport.height);
}

function drawLayers(
  context: CanvasRenderingContext2D,
  project: ProjectState,
  viewport: PreviewViewport,
  items: PreviewDrawItem[],
) {
  for (const item of items) {
    const transform = resolvePreviewTransform(item.layer.transform, project, item.lane);
    const crop = item.layer.crop;
    const cropActive = crop.width > 0 && crop.height > 0;
    const width = Math.max(24, transform.width * transform.scale * viewport.scale);
    const height = Math.max(18, transform.height * transform.scale * viewport.scale);
    const x = viewport.offsetX + transform.x * viewport.scale;
    const y = viewport.offsetY + transform.y * viewport.scale;
    const centerX = x + width / 2;
    const centerY = y + height / 2;
    context.save();
    context.translate(centerX, centerY);
    context.rotate((transform.rotation * Math.PI) / 180);
    context.translate(-centerX, -centerY);
    context.globalAlpha =
      (item.layer.contentKind === "audio" || item.layer.contentKind === "voice" ? 0.72 : 0.94) *
      transform.opacity;
    context.fillStyle = item.color;
    if (item.layer.fit === "blur_background") {
      context.globalAlpha *= 0.35;
      roundedRect(context, x - 8, y - 8, width + 16, height + 16, 8);
      context.fill();
      context.globalAlpha =
        (item.layer.contentKind === "audio" || item.layer.contentKind === "voice" ? 0.72 : 0.94) *
        transform.opacity;
    }
    drawMaskedLayerShape(context, item.layer.mask, x, y, width, height);
    context.globalAlpha = 1;
    if (cropActive) {
      const cropX = x + Math.min(width - 8, crop.x * viewport.scale);
      const cropY = y + Math.min(height - 8, crop.y * viewport.scale);
      const cropWidth = Math.min(width, crop.width * viewport.scale);
      const cropHeight = Math.min(height, crop.height * viewport.scale);
      context.strokeStyle = "#101417";
      context.lineWidth = 2;
      context.setLineDash([5, 4]);
      context.strokeRect(cropX, cropY, cropWidth, cropHeight);
      context.setLineDash([]);
    }
    if (item.layer.transition.kind !== "none") {
      context.strokeStyle = "#ffffff";
      context.lineWidth = 2;
      context.setLineDash([8, 5]);
      context.strokeRect(x - 3, y - 3, width + 6, height + 6);
      context.setLineDash([]);
      context.fillStyle = "#ffffff";
      context.font = "700 11px system-ui, sans-serif";
      context.fillText(item.layer.transition.kind, x + 12, Math.max(y + 14, y + height - 12));
    }
    context.fillStyle = "#101417";
    context.font = "600 14px system-ui, sans-serif";
    context.textBaseline = "middle";
    context.fillText(item.layer.label, x + 12, y + height / 2);
    context.restore();
  }
}

function drawMaskedLayerShape(
  context: CanvasRenderingContext2D,
  mask: TimelineLayer["mask"],
  x: number,
  y: number,
  width: number,
  height: number,
) {
  context.beginPath();
  switch (mask) {
    case "circle": {
      const radius = Math.min(width, height) / 2;
      context.ellipse(x + width / 2, y + height / 2, radius, radius, 0, 0, Math.PI * 2);
      break;
    }
    case "ellipse":
      context.ellipse(x + width / 2, y + height / 2, width / 2, height / 2, 0, 0, Math.PI * 2);
      break;
    case "rounded_rect":
      roundedRect(context, x, y, width, height, Math.min(18, width / 4, height / 4));
      break;
    case "none":
      roundedRect(context, x, y, width, height, 6);
      break;
  }
  context.fill();
}

function resolvePreviewTransform(
  transform: LayerTransform,
  project: ProjectState,
  lane: number,
): LayerTransform {
  const fallback = layerTransform({
    x: project.settings.width * 0.08,
    y: 120 + lane * 92,
    width: project.settings.width * 0.84,
    height: 64,
  });
  const resolved = layerTransform({ ...fallback, ...transform });
  return {
    ...resolved,
    width: resolved.width > 0 ? resolved.width : fallback.width,
    height: resolved.height > 0 ? resolved.height : fallback.height,
    scale: resolved.scale > 0 ? resolved.scale : fallback.scale,
    opacity: resolved.opacity >= 0 ? Math.min(resolved.opacity, 1) : fallback.opacity,
  };
}

function drawHud(
  context: CanvasRenderingContext2D,
  project: ProjectState,
  preview: PreviewState,
  viewport: PreviewViewport,
) {
  context.fillStyle = "#e8eef2";
  context.font = "600 13px system-ui, sans-serif";
  context.textBaseline = "alphabetic";
  context.fillText(
    `${project.settings.title} ${project.settings.width}x${project.settings.height} ${preview.currentTime.toFixed(
      2,
    )}s`,
    viewport.offsetX + 12,
    viewport.offsetY + viewport.height - 14,
  );
}

function roundedRect(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
) {
  context.beginPath();
  context.moveTo(x + radius, y);
  context.lineTo(x + width - radius, y);
  context.quadraticCurveTo(x + width, y, x + width, y + radius);
  context.lineTo(x + width, y + height - radius);
  context.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
  context.lineTo(x + radius, y + height);
  context.quadraticCurveTo(x, y + height, x, y + height - radius);
  context.lineTo(x, y + radius);
  context.quadraticCurveTo(x, y, x + radius, y);
  context.closePath();
}
