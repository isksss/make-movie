import { beforeEach, describe, expect, it } from "vitest";
import { usePreviewStore } from "./previewStore";

describe("previewStore", () => {
  beforeEach(() => {
    usePreviewStore.setState({
      playing: false,
      currentTime: 0,
      playbackRate: 1,
      loop: false,
    });
  });

  it("再生状態を切り替えられる", () => {
    usePreviewStore.getState().play();
    expect(usePreviewStore.getState().playing).toBe(true);

    usePreviewStore.getState().stop();
    expect(usePreviewStore.getState().playing).toBe(false);
  });

  it("frame 単位で移動できる", () => {
    usePreviewStore.getState().seek(1);
    usePreviewStore.getState().stepFrame(25, 1);

    expect(usePreviewStore.getState().currentTime).toBeCloseTo(1.04);
  });

  it("loop を切り替えられる", () => {
    usePreviewStore.getState().toggleLoop();
    expect(usePreviewStore.getState().loop).toBe(true);

    usePreviewStore.getState().toggleLoop();
    expect(usePreviewStore.getState().loop).toBe(false);
  });
});
