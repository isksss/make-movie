export type AssetKind = "video" | "image" | "audio" | "subtitle" | "font" | "mask";

export interface ProjectSettings {
  title: string;
  width: number;
  height: number;
  fps: number;
  sampleRate: number;
  duration: number;
  output: string;
}

export interface Asset {
  id: string;
  kind: AssetKind;
  path: string;
}

export interface TimelineLayer {
  id: string;
  trackId: string;
  label: string;
  contentKind: "video" | "image" | "audio" | "text" | "subtitle" | "voice";
  start: number;
  duration: number;
  zIndex: number;
  transform: LayerTransform;
}

export interface LayerTransform {
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
  rotation: number;
  opacity: number;
}

export interface Track {
  id: string;
  name: string;
  kind: "video" | "audio";
}

export interface ProjectState {
  settings: ProjectSettings;
  assets: Asset[];
  tracks: Track[];
  layers: TimelineLayer[];
}

export interface PreviewState {
  playing: boolean;
  currentTime: number;
  playbackRate: number;
  loop: boolean;
}

export interface TtsState {
  speaker: string;
  text: string;
  speed: number;
  pitch: number;
  emotion: string;
}

export function layerTransform(overrides: Partial<LayerTransform> = {}): LayerTransform {
  return {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
    scale: 1,
    rotation: 0,
    opacity: 1,
    ...overrides,
  };
}
