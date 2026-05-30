import {
  cropRect,
  layerTransform,
  layerTransition,
  textLayerStyle,
  voiceLayerSettings,
} from "../types";
import type {
  Asset,
  AssetKind,
  AssetMode,
  FitMode,
  LayerEffect,
  LayerTransition,
  PluginDeclaration,
  PluginRepository,
  ProjectState,
  Scene,
  TimelineLayer,
  Track,
  TtsProviderKind,
} from "../types";
import { initialProject } from "./projectStore";

type Section =
  | "settings"
  | "asset"
  | "scene"
  | "plugin"
  | "track"
  | "layer"
  | "content"
  | "textStroke"
  | "textShadow"
  | "transform";

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
    `asset_mode = ${quote(project.settings.assetMode)}`,
    "",
  ];
  if (project.settings.ffmpeg) {
    lines.push(`ffmpeg = ${quote(project.settings.ffmpeg)}`, "");
  }

  for (const asset of project.assets) {
    lines.push(
      "[[assets]]",
      `id = ${quote(asset.id)}`,
      `kind = ${quote(asset.kind)}`,
      `path = ${quote(asset.path)}`,
      "",
    );
  }

  for (const scene of project.scenes) {
    lines.push(
      "[[scenes]]",
      `id = ${quote(scene.id)}`,
      `name = ${quote(scene.name)}`,
      `start = ${scene.start}`,
      `duration = ${scene.duration}`,
      "",
    );
  }

  for (const plugin of project.plugins) {
    appendPlugin(lines, plugin);
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
    scenes: [],
    plugins: [],
    tracks: [],
    layers: [],
  };
  let section: Section | null = null;
  let currentScene: Scene | null = null;
  let currentPlugin: PluginDeclaration | null = null;
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
    if (line === "[[scenes]]") {
      section = "scene";
      currentScene = { id: "", name: "", start: 0, duration: 1 };
      project.scenes.push(currentScene);
      continue;
    }
    if (line === "[[plugin]]") {
      section = "plugin";
      currentPlugin = { repository: "github" };
      project.plugins.push(currentPlugin);
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
    if (line === "[tracks.layers.content.stroke]") {
      section = "textStroke";
      continue;
    }
    if (line === "[tracks.layers.content.shadow]") {
      section = "textShadow";
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
    assignValue(
      project,
      section,
      key,
      parseTomlValue(rawValue),
      currentScene,
      currentPlugin,
      currentTrack,
      currentLayer,
    );
  }

  return {
    settings: project.settings,
    assets: project.assets.length > 0 ? project.assets : structuredClone(fallback.assets),
    scenes: project.scenes.length > 0 ? project.scenes : structuredClone(fallback.scenes),
    plugins: project.plugins,
    tracks: project.tracks.length > 0 ? project.tracks : structuredClone(fallback.tracks),
    layers: project.layers.length > 0 ? project.layers : structuredClone(fallback.layers),
  };
}

function appendPlugin(lines: string[], plugin: PluginDeclaration) {
  lines.push("[[plugin]]", `repository = ${quote(plugin.repository)}`);
  if (plugin.owner) lines.push(`owner = ${quote(plugin.owner)}`);
  if (plugin.repo) lines.push(`repo = ${quote(plugin.repo)}`);
  if (plugin.version) lines.push(`version = ${quote(plugin.version)}`);
  if (plugin.url) lines.push(`url = ${quote(plugin.url)}`);
  if (plugin.path) lines.push(`path = ${quote(plugin.path)}`);
  lines.push("");
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
    lines.push(
      `text = ${quote(layer.text.text || layer.label)}`,
      `font_size = ${layer.text.fontSize}`,
      `color = ${quote(layer.text.color)}`,
      `letter_spacing = ${layer.text.letterSpacing}`,
      `line_spacing = ${layer.text.lineSpacing}`,
      `align = ${quote(layer.text.align)}`,
    );
    if (layer.text.stroke.width > 0) {
      lines.push(
        "[tracks.layers.content.stroke]",
        `color = ${quote(layer.text.stroke.color)}`,
        `width = ${layer.text.stroke.width}`,
      );
    }
    if (
      layer.text.shadow.offsetX !== 0 ||
      layer.text.shadow.offsetY !== 0 ||
      layer.text.shadow.blur > 0
    ) {
      lines.push(
        "[tracks.layers.content.shadow]",
        `color = ${quote(layer.text.shadow.color)}`,
        `offset_x = ${layer.text.shadow.offsetX}`,
        `offset_y = ${layer.text.shadow.offsetY}`,
        `blur = ${layer.text.shadow.blur}`,
      );
    }
    return;
  }
  if (layer.contentKind === "voice") {
    lines.push(
      `provider = ${quote(layer.voice.provider)}`,
      `speaker = ${quote(layer.voice.speaker)}`,
      `text = ${quote(layer.voice.text || layer.label)}`,
      `speed = ${layer.voice.speed}`,
      `pitch = ${layer.voice.pitch}`,
    );
    if (layer.voice.emotion) {
      lines.push(`emotion = ${quote(layer.voice.emotion)}`);
    }
    return;
  }
  const asset =
    project.assets.find((item) => item.id === layer.assetId) ??
    project.assets.find((item) => contentKindMatchesAsset(layer.contentKind, item.kind));
  if (asset) {
    lines.push(`asset_id = ${quote(asset.id)}`);
  }
  appendTrim(lines, layer);
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

function appendTrim(lines: string[], layer: TimelineLayer) {
  if (layer.contentKind !== "video" && layer.contentKind !== "audio") {
    return;
  }
  const trimStart = Math.max(0, layer.trimStart);
  const trimEnd = Math.max(0, layer.trimEnd);
  if (trimStart > 0) {
    lines.push(`trim_start = ${trimValue(layer, trimStart)}`);
  }
  if (trimEnd > 0) {
    lines.push(`trim_end = ${trimValue(layer, trimEnd)}`);
  }
}

function trimValue(layer: TimelineLayer, seconds: number) {
  return layer.contentKind === "audio" ? Math.round(seconds * 1000) : seconds;
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
  currentScene: Scene | null,
  currentPlugin: PluginDeclaration | null,
  currentTrack: Track | null,
  currentLayer: TimelineLayer | null,
) {
  if (section === "settings") {
    assignSettings(project, key, value);
  } else if (section === "asset") {
    const asset = project.assets.at(-1);
    if (asset) assignAsset(asset, key, value);
  } else if (section === "scene" && currentScene) {
    assignScene(currentScene, key, value);
  } else if (section === "plugin" && currentPlugin) {
    assignPlugin(currentPlugin, key, value);
  } else if (section === "track" && currentTrack) {
    assignTrack(currentTrack, key, value);
  } else if (section === "layer" && currentLayer) {
    assignLayer(currentLayer, currentTrack?.id ?? currentLayer.trackId, key, value);
  } else if (section === "content" && currentLayer) {
    assignContent(currentLayer, key, value);
  } else if (section === "textStroke" && currentLayer) {
    assignTextStroke(currentLayer, key, value);
  } else if (section === "textShadow" && currentLayer) {
    assignTextShadow(currentLayer, key, value);
  } else if (section === "transform" && currentLayer) {
    currentLayer.transform = {
      ...currentLayer.transform,
      [key === "opacity" ? "opacity" : key]: Number(value),
    };
  }
}

function assignSettings(project: ProjectState, key: string, value: string | number) {
  if (key === "sample_rate") project.settings.sampleRate = Number(value);
  else if (key === "asset_mode") project.settings.assetMode = parseAssetMode(value);
  else if (key === "ffmpeg") project.settings.ffmpeg = String(value);
  else if (key === "title" || key === "output") project.settings[key] = String(value);
  else if (key in project.settings)
    project.settings[key as "width" | "height" | "fps" | "duration"] = Number(value);
}

function parseAssetMode(value: string | number): AssetMode {
  return String(value) === "link" ? "link" : "copy";
}

function assignAsset(asset: Asset, key: string, value: string | number) {
  if (key === "kind") asset.kind = String(value) as AssetKind;
  else if (key === "id" || key === "path") asset[key] = String(value);
}

function assignScene(scene: Scene, key: string, value: string | number) {
  if (key === "id" || key === "name") scene[key] = String(value);
  else if (key === "start" || key === "duration") scene[key] = Number(value);
}

function assignPlugin(plugin: PluginDeclaration, key: string, value: string | number) {
  if (key === "repository") {
    plugin.repository = parsePluginRepository(value);
  } else if (
    key === "owner" ||
    key === "repo" ||
    key === "version" ||
    key === "url" ||
    key === "path"
  ) {
    plugin[key] = String(value);
  }
}

function parsePluginRepository(value: string | number): PluginRepository {
  const repository = String(value);
  if (
    repository === "github" ||
    repository === "gitlab" ||
    repository === "url" ||
    repository === "local"
  ) {
    return repository;
  }
  return "github";
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
  } else if (key === "text") {
    if (layer.contentKind === "voice") {
      layer.voice = { ...layer.voice, text: String(value) };
      layer.label = String(value);
      return;
    }
    layer.text = { ...layer.text, text: String(value) };
    if (layer.label === layer.id || layer.contentKind === "text") {
      layer.label = String(value);
    }
  } else if (key === "font_size") {
    layer.text = { ...layer.text, fontSize: Number(value) };
  } else if (key === "color") {
    layer.text = { ...layer.text, color: String(value) };
  } else if (key === "letter_spacing") {
    layer.text = { ...layer.text, letterSpacing: Number(value) };
  } else if (key === "line_spacing") {
    layer.text = { ...layer.text, lineSpacing: Number(value) };
  } else if (key === "align") {
    const align = String(value);
    if (align === "left" || align === "center" || align === "right") {
      layer.text = { ...layer.text, align };
    }
  } else if (key === "fit") {
    layer.fit = String(value) as FitMode;
  } else if (key === "provider") {
    layer.voice = { ...layer.voice, provider: parseTtsProvider(value) };
  } else if (key === "asset_id") {
    layer.assetId = String(value);
  } else if (key === "speaker") {
    layer.voice = { ...layer.voice, speaker: String(value) };
  } else if (key === "speed") {
    layer.voice = { ...layer.voice, speed: Number(value) };
  } else if (key === "pitch") {
    layer.voice = { ...layer.voice, pitch: Number(value) };
  } else if (key === "emotion") {
    layer.voice = { ...layer.voice, emotion: String(value) };
  } else if (key === "trim_start") {
    layer.trimStart = parseTrimValue(layer, value);
  } else if (key === "trim_end") {
    layer.trimEnd = parseTrimValue(layer, value);
  }
}

function parseTtsProvider(value: string | number): TtsProviderKind {
  const provider = String(value);
  if (provider === "voicevox" || provider === "aivis_speech" || provider === "coeiro_ink") {
    return provider;
  }
  return "voicevox";
}

function assignTextStroke(layer: TimelineLayer, key: string, value: string | number) {
  if (key === "color") {
    layer.text = { ...layer.text, stroke: { ...layer.text.stroke, color: String(value) } };
  } else if (key === "width") {
    layer.text = { ...layer.text, stroke: { ...layer.text.stroke, width: Number(value) } };
  }
}

function assignTextShadow(layer: TimelineLayer, key: string, value: string | number) {
  if (key === "color") {
    layer.text = { ...layer.text, shadow: { ...layer.text.shadow, color: String(value) } };
  } else if (key === "offset_x") {
    layer.text = { ...layer.text, shadow: { ...layer.text.shadow, offsetX: Number(value) } };
  } else if (key === "offset_y") {
    layer.text = { ...layer.text, shadow: { ...layer.text.shadow, offsetY: Number(value) } };
  } else if (key === "blur") {
    layer.text = { ...layer.text, shadow: { ...layer.text.shadow, blur: Number(value) } };
  }
}

function parseTrimValue(layer: TimelineLayer, value: string | number) {
  const number = Number(value);
  return layer.contentKind === "audio" ? number / 1000 : number;
}

function defaultLayer(trackId: string): TimelineLayer {
  return {
    id: "",
    trackId,
    label: "",
    contentKind: "image",
    assetId: null,
    start: 0,
    duration: 1,
    trimStart: 0,
    trimEnd: 0,
    zIndex: 0,
    transform: layerTransform(),
    crop: cropRect(),
    mask: "none",
    fit: "none",
    text: textLayerStyle(),
    voice: voiceLayerSettings(),
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
