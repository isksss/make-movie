export type AssetKind = "video" | "image" | "audio" | "subtitle" | "font" | "mask";
export type MaskKind = "none" | "circle" | "rounded_rect" | "ellipse" | "svg";
export type FitMode = "none" | "contain" | "cover" | "stretch" | "blur_background";
export type TransitionKind = "none" | "crossfade" | "wipe" | "push" | "zoom" | "blur" | "flash";
export type WipeShapeKind = "circle" | "rounded_rect";
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
  | "height"
  | "crop_x"
  | "crop_y"
  | "crop_width"
  | "crop_height";
export type EasingKind =
  | "linear"
  | "ease_in"
  | "ease_out"
  | "ease_in_out"
  | "ease_out_back"
  | "bounce"
  | "elastic";
export type TextAlignKind = "left" | "center" | "right";
export type TextGradientDirection = "vertical" | "horizontal";
export type TtsProviderKind = "voicevox" | "aivis_speech" | "coeiro_ink";
export type AssetMode = "copy" | "link";
export type RenderBackend = "auto" | "cpu" | "skia" | "gpu";

export interface GpuProbeResult {
  available: boolean;
  adapterName: string | null;
}

export interface ProjectSettings {
  title: string;
  width: number;
  height: number;
  fps: number;
  sampleRate: number;
  duration: number;
  output: string;
  assetMode: AssetMode;
  ffmpeg: string | null;
}

export type PluginRepository = "github" | "gitlab" | "url" | "local";

export interface PluginDeclaration {
  repository: PluginRepository;
  owner?: string;
  repo?: string;
  version?: string;
  url?: string;
  path?: string;
}

export interface Asset {
  id: string;
  kind: AssetKind;
  path: string;
}

export interface TimelineLayer {
  id: string;
  trackId: string;
  groupId: string | null;
  label: string;
  contentKind: "video" | "image" | "audio" | "text" | "subtitle" | "voice";
  assetId: string | null;
  start: number;
  duration: number;
  trimStart: number;
  trimEnd: number;
  zIndex: number;
  transform: LayerTransform;
  crop: CropRect;
  mask: MaskKind;
  maskRadius?: number;
  maskPath?: string;
  fit: FitMode;
  text: TextLayerStyle;
  voice: VoiceLayerSettings;
  transition: LayerTransition;
  effects: LayerEffect[];
  animations: LayerAnimation[];
}

export interface TextLayerStyle {
  text: string;
  fontFamily: string;
  fontSize: number;
  color: string;
  letterSpacing: number;
  lineSpacing: number;
  align: TextAlignKind;
  stroke: TextStrokeStyle;
  shadow: TextShadowStyle;
  gradient: TextGradientStyle;
}

export interface TextGradientStyle {
  enabled: boolean;
  startColor: string;
  endColor: string;
  direction: TextGradientDirection;
}

export interface TextStrokeStyle {
  color: string;
  width: number;
}

export interface TextShadowStyle {
  color: string;
  offsetX: number;
  offsetY: number;
  blur: number;
}

export interface VoiceLayerSettings {
  provider: TtsProviderKind;
  speaker: string;
  text: string;
  speed: number;
  pitch: number;
  emotion: string;
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
  wipeShape: WipeShapeKind;
  wipeRadius: number;
  wipeBorderColor: string;
  wipeBorderWidth: number;
  wipeShadowColor: string;
  wipeShadowOffsetX: number;
  wipeShadowOffsetY: number;
  wipeShadowBlur: number;
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

export interface TimelineGroup {
  id: string;
  name: string;
}

export interface ProjectState {
  settings: ProjectSettings;
  assets: Asset[];
  tracks: Track[];
  layers: TimelineLayer[];
  scenes: Scene[];
  groups: TimelineGroup[];
  plugins: PluginDeclaration[];
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
    wipeShape: "circle",
    wipeRadius: 24,
    wipeBorderColor: "#ffffff",
    wipeBorderWidth: 0,
    wipeShadowColor: "#000000",
    wipeShadowOffsetX: 0,
    wipeShadowOffsetY: 0,
    wipeShadowBlur: 0,
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

export function textLayerStyle(overrides: Partial<TextLayerStyle> = {}): TextLayerStyle {
  return {
    text: "",
    fontFamily: "",
    fontSize: 48,
    color: "#ffffff",
    letterSpacing: 0,
    lineSpacing: 1.2,
    align: "center",
    stroke: { color: "#000000", width: 0 },
    shadow: { color: "#000000", offsetX: 0, offsetY: 0, blur: 0 },
    gradient: { enabled: false, startColor: "#ffffff", endColor: "#ffcc00", direction: "vertical" },
    ...overrides,
  };
}

export function voiceLayerSettings(
  overrides: Partial<VoiceLayerSettings> = {},
): VoiceLayerSettings {
  return {
    provider: "voicevox",
    speaker: "ずんだもん",
    text: "",
    speed: 1,
    pitch: 0,
    emotion: "neutral",
    ...overrides,
  };
}
