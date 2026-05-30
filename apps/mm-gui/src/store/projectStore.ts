import { create } from "zustand";
import type { Asset, ProjectState, TimelineLayer, TtsState } from "../types";

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
  addAsset: (asset: Asset) => void;
  updateTtsText: (text: string) => void;
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
  layers: [
    {
      id: "title",
      trackId: "v3",
      label: "Title Text",
      contentKind: "text",
      start: 0,
      duration: 4,
      zIndex: 10,
    },
    {
      id: "intro-image",
      trackId: "v2",
      label: "Intro Image",
      contentKind: "image",
      start: 0.5,
      duration: 7,
      zIndex: 2,
    },
    {
      id: "subtitle-main",
      trackId: "v4",
      label: "Subtitle",
      contentKind: "subtitle",
      start: 0,
      duration: 9,
      zIndex: 12,
    },
    {
      id: "voice-main",
      trackId: "a1",
      label: "Narration",
      contentKind: "voice",
      start: 0,
      duration: 12,
      zIndex: 0,
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
  addAsset: (asset) =>
    set((state) => ({
      ...withHistory(state, () => ({
        project: { ...state.project, assets: [...state.project.assets, asset] },
      })),
      selectedAssetId: asset.id,
    })),
  updateTtsText: (text) =>
    set((state) =>
      withHistory(state, () => ({
        tts: { ...state.tts, text },
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
