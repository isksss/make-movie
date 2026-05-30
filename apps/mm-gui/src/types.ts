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
  start: number;
  duration: number;
  zIndex: number;
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
