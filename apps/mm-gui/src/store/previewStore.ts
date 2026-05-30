import { create } from "zustand";
import type { PreviewState } from "../types";

interface PreviewStore extends PreviewState {
  play: () => void;
  stop: () => void;
  seek: (time: number) => void;
  stepFrame: (fps: number, direction: 1 | -1) => void;
  setRate: (playbackRate: number) => void;
  toggleLoop: () => void;
}

export const usePreviewStore = create<PreviewStore>((set) => ({
  playing: false,
  currentTime: 0,
  playbackRate: 1,
  loop: false,
  play: () => set({ playing: true }),
  stop: () => set({ playing: false }),
  seek: (currentTime) => set({ currentTime: Math.max(0, currentTime) }),
  stepFrame: (fps, direction) =>
    set((state) => ({
      currentTime: Math.max(0, state.currentTime + direction * (1 / fps)),
    })),
  setRate: (playbackRate) => set({ playbackRate }),
  toggleLoop: () => set((state) => ({ loop: !state.loop })),
}));
