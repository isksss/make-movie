import { create } from "zustand";
import {
  cropRect,
  layerAnimation,
  layerEffect,
  layerTransform,
  layerTransition,
  textLayerStyle,
  voiceLayerSettings,
} from "../types";
import type {
  Asset,
  CropRect,
  FitMode,
  LayerAnimation,
  LayerEffect,
  LayerTransition,
  LayerTransform,
  MaskKind,
  ProjectState,
  TextLayerStyle,
  TimelineLayer,
  TtsState,
  VoiceLayerSettings,
} from "../types";

interface ProjectSnapshot {
  project: ProjectState;
  tts: TtsState;
}

interface ProjectStore {
  project: ProjectState;
  selectedLayerId: string | null;
  selectedAssetId: string | null;
  tts: TtsState;
  past: ProjectSnapshot[];
  future: ProjectSnapshot[];
  canUndo: boolean;
  canRedo: boolean;
  setProject: (project: ProjectState) => void;
  selectLayer: (id: string) => void;
  selectAsset: (id: string) => void;
  moveLayer: (id: string, start: number) => void;
  splitLayer: (id: string, time: number) => void;
  duplicateLayer: (id: string) => void;
  deleteLayer: (id: string) => void;
  updateLayerTrim: (
    id: string,
    trim: Partial<Pick<TimelineLayer, "trimStart" | "trimEnd">>,
  ) => void;
  updateLayerTransform: (id: string, transform: Partial<LayerTransform>) => void;
  updateLayerVisual: (
    id: string,
    visual: Partial<{ crop: Partial<CropRect>; mask: MaskKind; fit: FitMode }>,
  ) => void;
  updateLayerText: (id: string, text: Partial<TextLayerStyle>) => void;
  updateLayerVoice: (id: string, voice: Partial<VoiceLayerSettings>) => void;
  updateLayerTransition: (id: string, transition: Partial<LayerTransition>) => void;
  updateLayerEffect: (id: string, effect: Partial<LayerEffect>) => void;
  updateLayerAnimation: (id: string, animation: Partial<LayerAnimation>) => void;
  addAsset: (asset: Asset) => void;
  updateTts: (tts: Partial<TtsState>) => void;
  undo: () => void;
  redo: () => void;
}

export const initialProject: ProjectState = {
  settings: {
    title: "make-movie",
    width: 1080,
    height: 1920,
    fps: 30,
    sampleRate: 48000,
    duration: 30,
    output: "output/movie.mp4",
  },
  assets: [
    { id: "intro", kind: "image", path: "media/image/intro.png" },
    { id: "voice", kind: "audio", path: "media/audio/voice.wav" },
    { id: "subtitle", kind: "subtitle", path: "media/subtitle/main.srt" },
  ],
  tracks: [
    { id: "v1", name: "V1 Main Video", kind: "video" },
    { id: "v2", name: "V2 Overlay", kind: "video" },
    { id: "v3", name: "V3 Text", kind: "video" },
    { id: "v4", name: "V4 Subtitle", kind: "video" },
    { id: "a1", name: "A1 Voice", kind: "audio" },
    { id: "a2", name: "A2 BGM", kind: "audio" },
  ],
  scenes: [{ id: "intro", name: "Intro", start: 0, duration: 12 }],
  plugins: [],
  layers: [
    {
      id: "title",
      trackId: "v3",
      label: "Title Text",
      contentKind: "text",
      start: 0,
      duration: 4,
      trimStart: 0,
      trimEnd: 0,
      zIndex: 10,
      transform: layerTransform({ x: 140, y: 120, width: 800, height: 120 }),
      crop: cropRect(),
      mask: "none",
      fit: "none",
      text: textLayerStyle({ text: "Title Text", fontSize: 64 }),
      voice: voiceLayerSettings(),
      transition: layerTransition(),
      effects: [],
      animations: [],
    },
    {
      id: "intro-image",
      trackId: "v2",
      label: "Intro Image",
      contentKind: "image",
      start: 0.5,
      duration: 7,
      trimStart: 0,
      trimEnd: 0,
      zIndex: 2,
      transform: layerTransform({ x: 120, y: 300, width: 840, height: 480 }),
      crop: cropRect(),
      mask: "none",
      fit: "contain",
      text: textLayerStyle(),
      voice: voiceLayerSettings(),
      transition: layerTransition(),
      effects: [],
      animations: [],
    },
    {
      id: "subtitle-main",
      trackId: "v4",
      label: "Subtitle",
      contentKind: "subtitle",
      start: 0,
      duration: 9,
      trimStart: 0,
      trimEnd: 0,
      zIndex: 12,
      transform: layerTransform({ x: 120, y: 1600, width: 840, height: 120 }),
      crop: cropRect(),
      mask: "none",
      fit: "none",
      text: textLayerStyle(),
      voice: voiceLayerSettings(),
      transition: layerTransition(),
      effects: [],
      animations: [],
    },
    {
      id: "voice-main",
      trackId: "a1",
      label: "Narration",
      contentKind: "voice",
      start: 0,
      duration: 12,
      trimStart: 0,
      trimEnd: 0,
      zIndex: 0,
      transform: layerTransform({ x: 120, y: 1760, width: 840, height: 80 }),
      crop: cropRect(),
      mask: "none",
      fit: "none",
      text: textLayerStyle(),
      voice: voiceLayerSettings({
        text: "今日のニュースを解説します。",
      }),
      transition: layerTransition(),
      effects: [],
      animations: [],
    },
    {
      id: "voice-audio",
      trackId: "a2",
      label: "Voice Audio",
      contentKind: "audio",
      start: 0,
      duration: 12,
      trimStart: 0,
      trimEnd: 0,
      zIndex: 0,
      transform: layerTransform({ x: 120, y: 1840, width: 840, height: 60 }),
      crop: cropRect(),
      mask: "none",
      fit: "none",
      text: textLayerStyle(),
      voice: voiceLayerSettings(),
      transition: layerTransition(),
      effects: [],
      animations: [],
    },
  ],
};

const snapshot = (state: Pick<ProjectStore, "project" | "tts">): ProjectSnapshot => ({
  project: structuredClone(state.project),
  tts: structuredClone(state.tts),
});

function withHistory(
  state: ProjectStore,
  update: (state: ProjectStore) => Pick<ProjectStore, "project"> | Pick<ProjectStore, "tts">,
) {
  const next = update(state);
  return {
    ...next,
    past: [...state.past, snapshot(state)],
    future: [],
    canUndo: true,
    canRedo: false,
  };
}

export const useProjectStore = create<ProjectStore>((set) => ({
  project: initialProject,
  selectedLayerId: "title",
  selectedAssetId: "intro",
  tts: {
    speaker: "ずんだもん",
    text: "今日のニュースを解説します。",
    speed: 1,
    pitch: 0,
    emotion: "neutral",
  },
  past: [],
  future: [],
  canUndo: false,
  canRedo: false,
  setProject: (project) =>
    set((state) =>
      withHistory(state, () => ({
        project,
      })),
    ),
  selectLayer: (id) => set({ selectedLayerId: id }),
  selectAsset: (id) => set({ selectedAssetId: id }),
  moveLayer: (id, start) =>
    set((state) =>
      withHistory(state, () => ({
        project: {
          ...state.project,
          layers: state.project.layers.map(
            (layer): TimelineLayer => (layer.id === id ? { ...layer, start } : layer),
          ),
        },
      })),
    ),
  splitLayer: (id, time) =>
    set((state) => {
      const layerIndex = state.project.layers.findIndex((layer) => layer.id === id);
      if (layerIndex < 0) {
        return {};
      }
      const layer = state.project.layers[layerIndex];
      const end = layer.start + layer.duration;
      if (time <= layer.start || time >= end) {
        return {};
      }
      const nextId = uniqueLayerId(state.project.layers, `${layer.id}-split`);
      const nextLabel = uniqueLayerLabel(state.project.layers, `${layer.label} (2)`);
      const first: TimelineLayer = {
        ...layer,
        duration: time - layer.start,
      };
      const second: TimelineLayer = {
        ...layer,
        id: nextId,
        label: nextLabel,
        start: time,
        duration: end - time,
      };
      const layers = [...state.project.layers];
      layers.splice(layerIndex, 1, first, second);
      return {
        ...withHistory(state, () => ({
          project: { ...state.project, layers },
        })),
        selectedLayerId: nextId,
      };
    }),
  duplicateLayer: (id) =>
    set((state) => {
      const layerIndex = state.project.layers.findIndex((layer) => layer.id === id);
      if (layerIndex < 0) {
        return {};
      }
      const layer = state.project.layers[layerIndex];
      const nextId = uniqueLayerId(state.project.layers, `${layer.id}-copy`);
      const nextLabel = uniqueLayerLabel(state.project.layers, `${layer.label} Copy`);
      const nextStart =
        layer.start + layer.duration <= state.project.settings.duration
          ? layer.start + layer.duration
          : layer.start;
      const duplicate: TimelineLayer = {
        ...layer,
        id: nextId,
        label: nextLabel,
        start: nextStart,
      };
      const layers = [...state.project.layers];
      layers.splice(layerIndex + 1, 0, duplicate);
      return {
        ...withHistory(state, () => ({
          project: { ...state.project, layers },
        })),
        selectedLayerId: nextId,
      };
    }),
  deleteLayer: (id) =>
    set((state) => {
      const layerIndex = state.project.layers.findIndex((layer) => layer.id === id);
      if (layerIndex < 0) {
        return {};
      }
      const layers = state.project.layers.filter((layer) => layer.id !== id);
      const nextSelection = layers[layerIndex]?.id ?? layers[layerIndex - 1]?.id ?? null;
      return {
        ...withHistory(state, () => ({
          project: { ...state.project, layers },
        })),
        selectedLayerId: nextSelection,
      };
    }),
  updateLayerTrim: (id, trim) =>
    set((state) =>
      withHistory(state, () => ({
        project: {
          ...state.project,
          layers: state.project.layers.map(
            (layer): TimelineLayer => (layer.id === id ? { ...layer, ...trim } : layer),
          ),
        },
      })),
    ),
  updateLayerTransform: (id, transform) =>
    set((state) =>
      withHistory(state, () => ({
        project: {
          ...state.project,
          layers: state.project.layers.map((layer): TimelineLayer => {
            if (layer.id !== id) {
              return layer;
            }
            return {
              ...layer,
              transform: {
                ...layerTransform(layer.transform),
                ...transform,
              },
            };
          }),
        },
      })),
    ),
  updateLayerVisual: (id, visual) =>
    set((state) =>
      withHistory(state, () => ({
        project: {
          ...state.project,
          layers: state.project.layers.map((layer): TimelineLayer => {
            if (layer.id !== id) {
              return layer;
            }
            return {
              ...layer,
              crop: visual.crop
                ? {
                    ...cropRect(layer.crop),
                    ...visual.crop,
                  }
                : layer.crop,
              mask: visual.mask ?? layer.mask,
              fit: visual.fit ?? layer.fit,
            };
          }),
        },
      })),
    ),
  updateLayerText: (id, text) =>
    set((state) =>
      withHistory(state, () => ({
        project: {
          ...state.project,
          layers: state.project.layers.map((layer): TimelineLayer => {
            if (layer.id !== id) {
              return layer;
            }
            return {
              ...layer,
              label: text.text ?? layer.label,
              text: {
                ...layer.text,
                ...text,
                stroke: {
                  ...layer.text.stroke,
                  ...text.stroke,
                },
                shadow: {
                  ...layer.text.shadow,
                  ...text.shadow,
                },
              },
            };
          }),
        },
      })),
    ),
  updateLayerVoice: (id, voice) =>
    set((state) =>
      withHistory(state, () => ({
        project: {
          ...state.project,
          layers: state.project.layers.map((layer): TimelineLayer => {
            if (layer.id !== id) {
              return layer;
            }
            return {
              ...layer,
              label: voice.text ?? layer.label,
              voice: {
                ...layer.voice,
                ...voice,
              },
            };
          }),
        },
      })),
    ),
  updateLayerTransition: (id, transition) =>
    set((state) =>
      withHistory(state, () => ({
        project: {
          ...state.project,
          layers: state.project.layers.map((layer): TimelineLayer => {
            if (layer.id !== id) {
              return layer;
            }
            return {
              ...layer,
              transition: {
                ...layerTransition(layer.transition),
                ...transition,
              },
            };
          }),
        },
      })),
    ),
  updateLayerEffect: (id, effect) =>
    set((state) =>
      withHistory(state, () => ({
        project: {
          ...state.project,
          layers: state.project.layers.map((layer): TimelineLayer => {
            if (layer.id !== id) {
              return layer;
            }
            const nextEffect = {
              ...layerEffect(layer.effects[0]),
              ...effect,
            };
            return {
              ...layer,
              effects: nextEffect.kind === "none" ? [] : [nextEffect],
            };
          }),
        },
      })),
    ),
  updateLayerAnimation: (id, animation) =>
    set((state) =>
      withHistory(state, () => ({
        project: {
          ...state.project,
          layers: state.project.layers.map((layer): TimelineLayer => {
            if (layer.id !== id) {
              return layer;
            }
            const current = layerAnimation(layer.animations[0]);
            const nextAnimation = {
              ...current,
              ...animation,
              keyframes: animation.keyframes ?? current.keyframes,
            };
            return {
              ...layer,
              animations: nextAnimation.property === "none" ? [] : [nextAnimation],
            };
          }),
        },
      })),
    ),
  addAsset: (asset) =>
    set((state) => ({
      ...withHistory(state, () => ({
        project: { ...state.project, assets: [...state.project.assets, asset] },
      })),
      selectedAssetId: asset.id,
    })),
  updateTts: (tts) =>
    set((state) =>
      withHistory(state, () => ({
        tts: { ...state.tts, ...tts },
      })),
    ),
  undo: () =>
    set((state) => {
      const previous = state.past.at(-1);
      if (!previous) {
        return {};
      }
      const past = state.past.slice(0, -1);
      return {
        project: previous.project,
        tts: previous.tts,
        past,
        future: [snapshot(state), ...state.future],
        canUndo: past.length > 0,
        canRedo: true,
      };
    }),
  redo: () =>
    set((state) => {
      const next = state.future[0];
      if (!next) {
        return {};
      }
      const future = state.future.slice(1);
      return {
        project: next.project,
        tts: next.tts,
        past: [...state.past, snapshot(state)],
        future,
        canUndo: true,
        canRedo: future.length > 0,
      };
    }),
}));

function uniqueLayerId(layers: TimelineLayer[], base: string) {
  if (!layers.some((layer) => layer.id === base)) {
    return base;
  }
  for (let index = 2; ; index += 1) {
    const candidate = `${base}-${index}`;
    if (!layers.some((layer) => layer.id === candidate)) {
      return candidate;
    }
  }
}

function uniqueLayerLabel(layers: TimelineLayer[], base: string) {
  if (!layers.some((layer) => layer.label === base)) {
    return base;
  }
  for (let index = 3; ; index += 1) {
    const candidate = base.replace(/\(\d+\)$/, `(${index})`);
    if (!layers.some((layer) => layer.label === candidate)) {
      return candidate;
    }
  }
}
