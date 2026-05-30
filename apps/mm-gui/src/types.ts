export type AssetKind = "video" | "image" | "audio" | "subtitle" | "font" | "mask";
export type MaskKind = "none" | "circle" | "rounded_rect" | "ellipse";
export type FitMode = "none" | "contain" | "cover" | "stretch" | "blur_background";
export type TransitionKind = "none" | "crossfade" | "wipe" | "push" | "zoom" | "blur" | "flash";
export type EffectKind =
  | "none"
  | "fade_in"
  | "fade_out"
  | "blur"
  | "zoom"
  | "slide"
  | "brightness"
  | "contrast"
  | "saturation"
  | "pixelate"
  | "motion_blur";
export type AnimatedProperty =
  | "none"
  | "x"
  | "y"
  | "scale"
  | "rotation"
  | "opacity"
  | "width"
  | "height";
export type EasingKind =
  | "linear"
  | "ease_in"
  | "ease_out"
  | "ease_in_out"
  | "ease_out_back"
  | "bounce"
  | "elastic";

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
  crop: CropRect;
  mask: MaskKind;
  fit: FitMode;
  transition: LayerTransition;
  effects: LayerEffect[];
  animations: LayerAnimation[];
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

export interface CropRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface LayerTransition {
  kind: TransitionKind;
  duration: number;
}

export interface LayerEffect {
  kind: EffectKind;
  duration: number;
  amount: number;
  x: number;
  y: number;
}

export interface LayerAnimation {
  property: AnimatedProperty;
  easing: EasingKind;
  keyframes: Keyframe[];
}

export interface Keyframe {
  time: number;
  value: number;
}

export interface Track {
  id: string;
  name: string;
  kind: "video" | "audio";
}

export interface Scene {
  id: string;
  name: string;
  start: number;
  duration: number;
}

export interface ProjectState {
  settings: ProjectSettings;
  assets: Asset[];
  tracks: Track[];
  layers: TimelineLayer[];
  scenes: Scene[];
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

export function cropRect(overrides: Partial<CropRect> = {}): CropRect {
  return {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
    ...overrides,
  };
}

export function layerTransition(overrides: Partial<LayerTransition> = {}): LayerTransition {
  return {
    kind: "none",
    duration: 0.5,
    ...overrides,
  };
}

export function layerEffect(overrides: Partial<LayerEffect> = {}): LayerEffect {
  return {
    kind: "none",
    duration: 0.5,
    amount: 1,
    x: 0,
    y: 0,
    ...overrides,
  };
}

export function layerAnimation(overrides: Partial<LayerAnimation> = {}): LayerAnimation {
  return {
    property: "none",
    easing: "linear",
    keyframes: [
      { time: 0, value: 0 },
      { time: 1, value: 1 },
    ],
    ...overrides,
  };
}
