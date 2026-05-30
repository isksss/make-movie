import { cropRect, layerTransform, layerTransition } from "../types";
import type {
  Asset,
  AssetKind,
  FitMode,
  LayerEffect,
  LayerTransition,
  ProjectState,
  TimelineLayer,
  Track,
} from "../types";
import { initialProject } from "./projectStore";

type Section = "settings" | "asset" | "track" | "layer" | "content" | "transform";

export function serializeProjectToToml(project: ProjectState): string {
  const lines: string[] = [
    "[settings]",
    `title = ${quote(project.settings.title)}`,
    `width = ${project.settings.width}`,
    `height = ${project.settings.height}`,
    `fps = ${project.settings.fps}`,
    `sample_rate = ${project.settings.sampleRate}`,
    `duration = ${project.settings.duration}`,
    `output = ${quote(project.settings.output)}`,
    'asset_mode = "copy"',
    "",
  ];

  for (const asset of project.assets) {
    lines.push(
      "[[assets]]",
      `id = ${quote(asset.id)}`,
      `kind = ${quote(asset.kind)}`,
      `path = ${quote(asset.path)}`,
      "",
    );
  }

  for (const track of project.tracks) {
    lines.push(
      "[[tracks]]",
      `id = ${quote(track.id)}`,
      `name = ${quote(track.name)}`,
      `kind = ${quote(track.kind)}`,
      "",
    );
    for (const layer of project.layers.filter((item) => item.trackId === track.id)) {
      appendLayer(lines, project, layer);
    }
  }

  return `${lines.join("\n")}\n`;
}

export function parseProjectToml(
  toml: string,
  fallback: ProjectState = initialProject,
): ProjectState {
  const project: ProjectState = {
    settings: { ...fallback.settings },
    assets: [],
    tracks: [],
    layers: [],
  };
  let section: Section | null = null;
  let currentTrack: Track | null = null;
  let currentLayer: TimelineLayer | null = null;

  for (const rawLine of toml.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) {
      continue;
    }
    if (line === "[settings]") {
      section = "settings";
      continue;
    }
    if (line === "[[assets]]") {
      section = "asset";
      project.assets.push({ id: "", kind: "image", path: "" });
      continue;
    }
    if (line === "[[tracks]]") {
      section = "track";
      currentTrack = { id: "", name: "", kind: "video" };
      project.tracks.push(currentTrack);
      continue;
    }
    if (line === "[[tracks.layers]]") {
      section = "layer";
      currentLayer = defaultLayer(currentTrack?.id ?? "v1");
      project.layers.push(currentLayer);
      continue;
    }
    if (line === "[tracks.layers.content]") {
      section = "content";
      continue;
    }
    if (line === "[tracks.layers.transform]") {
      section = "transform";
      continue;
    }

    const [key, rawValue] = splitTomlPair(line);
    if (!key || rawValue === undefined) {
      continue;
    }
    assignValue(project, section, key, parseTomlValue(rawValue), currentTrack, currentLayer);
  }

  return {
    settings: project.settings,
    assets: project.assets.length > 0 ? project.assets : structuredClone(fallback.assets),
    tracks: project.tracks.length > 0 ? project.tracks : structuredClone(fallback.tracks),
    layers: project.layers.length > 0 ? project.layers : structuredClone(fallback.layers),
  };
}

function appendLayer(lines: string[], project: ProjectState, layer: TimelineLayer) {
  lines.push(
    "[[tracks.layers]]",
    `id = ${quote(layer.id)}`,
    `label = ${quote(layer.label)}`,
    `start = ${layer.start}`,
    `duration = ${layer.duration}`,
    `z_index = ${layer.zIndex}`,
    "",
    "[tracks.layers.content]",
    `type = ${quote(layer.contentKind)}`,
  );
  appendContent(lines, project, layer);
  lines.push(
    "",
    "[tracks.layers.transform]",
    `x = ${layer.transform.x}`,
    `y = ${layer.transform.y}`,
    `width = ${layer.transform.width}`,
    `height = ${layer.transform.height}`,
    `scale = ${layer.transform.scale}`,
    `rotation = ${layer.transform.rotation}`,
    `opacity = ${layer.transform.opacity}`,
    "",
  );
  appendEffects(lines, layer.effects);
  appendTransition(lines, layer.transition);
  appendAnimations(lines, layer);
}

function appendContent(lines: string[], project: ProjectState, layer: TimelineLayer) {
  if (layer.contentKind === "text") {
    lines.push(`text = ${quote(layer.label)}`);
    return;
  }
  if (layer.contentKind === "voice") {
    lines.push(
      'provider = "voicevox"',
      'speaker = "ずんだもん"',
      `text = ${quote(layer.label)}`,
      "speed = 1",
      "pitch = 0",
    );
    return;
  }
  const asset = project.assets.find((item) =>
    contentKindMatchesAsset(layer.contentKind, item.kind),
  );
  if (asset) {
    lines.push(`asset_id = ${quote(asset.id)}`);
  }
  if (layer.contentKind === "image" || layer.contentKind === "video") {
    if (layer.fit !== "none") {
      lines.push(`fit = ${quote(layer.fit)}`);
    }
    if (layer.crop.width > 0 && layer.crop.height > 0) {
      lines.push(
        "[tracks.layers.content.crop]",
        `x = ${layer.crop.x}`,
        `y = ${layer.crop.y}`,
        `width = ${layer.crop.width}`,
        `height = ${layer.crop.height}`,
      );
    }
  }
  if (layer.contentKind === "image" && layer.mask !== "none") {
    lines.push("[tracks.layers.content.mask]", `type = ${quote(layer.mask)}`);
    if (layer.mask === "rounded_rect") {
      lines.push("radius = 16");
    }
  }
}

function appendEffects(lines: string[], effects: LayerEffect[]) {
  for (const effect of effects) {
    if (effect.kind === "none") {
      continue;
    }
    lines.push("[[tracks.layers.effects]]", `type = ${quote(effect.kind)}`);
    if (effect.kind === "fade_in" || effect.kind === "fade_out") {
      lines.push(`duration = ${effect.duration}`);
    } else if (effect.kind === "slide") {
      lines.push(`x = ${effect.x}`, `y = ${effect.y}`);
    } else if (effect.kind === "pixelate") {
      lines.push(`size = ${Math.max(1, Math.round(effect.amount))}`);
    } else if (effect.kind === "blur") {
      lines.push(`radius = ${effect.amount}`);
    } else {
      lines.push(`amount = ${effect.amount}`);
    }
    lines.push("");
  }
}

function appendTransition(lines: string[], transition: LayerTransition) {
  if (transition.kind === "none") {
    return;
  }
  lines.push(
    "[tracks.layers.transition]",
    `type = ${quote(transition.kind === "crossfade" ? "cross_fade" : transition.kind)}`,
    `duration = ${transition.duration}`,
  );
  if (transition.kind === "wipe") {
    lines.push("[tracks.layers.transition.shape]", 'type = "circle"');
  }
  lines.push("");
}

function appendAnimations(lines: string[], layer: TimelineLayer) {
  for (const animation of layer.animations) {
    if (animation.property === "none") {
      continue;
    }
    lines.push(
      "[[tracks.layers.animations]]",
      `property = ${quote(animation.property)}`,
      `easing = ${quote(animation.easing)}`,
    );
    for (const keyframe of animation.keyframes) {
      lines.push(
        "[[tracks.layers.animations.keyframes]]",
        `time = ${keyframe.time}`,
        `value = ${keyframe.value}`,
      );
    }
    lines.push("");
  }
}

function assignValue(
  project: ProjectState,
  section: Section | null,
  key: string,
  value: string | number,
  currentTrack: Track | null,
  currentLayer: TimelineLayer | null,
) {
  if (section === "settings") {
    assignSettings(project, key, value);
  } else if (section === "asset") {
    const asset = project.assets.at(-1);
    if (asset) assignAsset(asset, key, value);
  } else if (section === "track" && currentTrack) {
    assignTrack(currentTrack, key, value);
  } else if (section === "layer" && currentLayer) {
    assignLayer(currentLayer, currentTrack?.id ?? currentLayer.trackId, key, value);
  } else if (section === "content" && currentLayer) {
    assignContent(currentLayer, key, value);
  } else if (section === "transform" && currentLayer) {
    currentLayer.transform = {
      ...currentLayer.transform,
      [key === "opacity" ? "opacity" : key]: Number(value),
    };
  }
}

function assignSettings(project: ProjectState, key: string, value: string | number) {
  if (key === "sample_rate") project.settings.sampleRate = Number(value);
  else if (key === "title" || key === "output") project.settings[key] = String(value);
  else if (key in project.settings)
    project.settings[key as "width" | "height" | "fps" | "duration"] = Number(value);
}

function assignAsset(asset: Asset, key: string, value: string | number) {
  if (key === "kind") asset.kind = String(value) as AssetKind;
  else if (key === "id" || key === "path") asset[key] = String(value);
}

function assignTrack(track: Track, key: string, value: string | number) {
  if (key === "kind") track.kind = String(value) === "audio" ? "audio" : "video";
  else if (key === "id" || key === "name") track[key] = String(value);
}

function assignLayer(layer: TimelineLayer, trackId: string, key: string, value: string | number) {
  layer.trackId = trackId;
  if (key === "z_index") layer.zIndex = Number(value);
  else if (key === "start" || key === "duration") layer[key] = Number(value);
  else if (key === "id") {
    layer.id = String(value);
    if (!layer.label) layer.label = layer.id;
  } else if (key === "label") layer.label = String(value);
}

function assignContent(layer: TimelineLayer, key: string, value: string | number) {
  if (key === "type") {
    layer.contentKind = String(value) as TimelineLayer["contentKind"];
  } else if (key === "text" && layer.label === layer.id) {
    layer.label = String(value);
  } else if (key === "fit") {
    layer.fit = String(value) as FitMode;
  }
}

function defaultLayer(trackId: string): TimelineLayer {
  return {
    id: "",
    trackId,
    label: "",
    contentKind: "image",
    start: 0,
    duration: 1,
    zIndex: 0,
    transform: layerTransform(),
    crop: cropRect(),
    mask: "none",
    fit: "none",
    transition: layerTransition(),
    effects: [],
    animations: [],
  };
}

function contentKindMatchesAsset(kind: TimelineLayer["contentKind"], assetKind: AssetKind) {
  return (
    (kind === "video" && assetKind === "video") ||
    (kind === "image" && assetKind === "image") ||
    (kind === "audio" && assetKind === "audio") ||
    (kind === "subtitle" && assetKind === "subtitle")
  );
}

function splitTomlPair(line: string): [string, string] {
  const index = line.indexOf("=");
  if (index < 0) return ["", ""];
  return [line.slice(0, index).trim(), line.slice(index + 1).trim()];
}

function parseTomlValue(value: string) {
  if (value.startsWith('"') && value.endsWith('"')) {
    return JSON.parse(value);
  }
  const number = Number(value);
  return Number.isFinite(number) ? number : value;
}

function quote(value: string) {
  return JSON.stringify(value);
}
