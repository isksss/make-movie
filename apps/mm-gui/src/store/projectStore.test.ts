import { beforeEach, describe, expect, it } from "vitest";
import { initialProject, useProjectStore } from "./projectStore";

describe("projectStore", () => {
  beforeEach(() => {
    useProjectStore.setState({
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
    });
  });

  it("layer の start を更新できる", () => {
    useProjectStore.getState().moveLayer("title", 2.5);

    const layer = useProjectStore.getState().project.layers.find((item) => item.id === "title");
    expect(layer?.start).toBe(2.5);
  });

  it("asset を追加すると選択状態も更新する", () => {
    useProjectStore.getState().addAsset({
      id: "mask",
      kind: "mask",
      path: "media/mask/circle.svg",
    });

    expect(useProjectStore.getState().selectedAssetId).toBe("mask");
    expect(useProjectStore.getState().project.assets).toHaveLength(4);
  });

  it("layer 移動をUndo/Redoできる", () => {
    useProjectStore.getState().moveLayer("title", 2.5);

    expect(useProjectStore.getState().canUndo).toBe(true);
    useProjectStore.getState().undo();
    expect(useProjectStore.getState().canRedo).toBe(true);
    expect(
      useProjectStore.getState().project.layers.find((item) => item.id === "title")?.start,
    ).toBe(0);

    useProjectStore.getState().redo();
    expect(
      useProjectStore.getState().project.layers.find((item) => item.id === "title")?.start,
    ).toBe(2.5);
  });

  it("TTS text 更新をUndo/Redoできる", () => {
    useProjectStore.getState().updateTtsText("更新後の文章");

    expect(useProjectStore.getState().tts.text).toBe("更新後の文章");
    useProjectStore.getState().undo();
    expect(useProjectStore.getState().tts.text).toBe("今日のニュースを解説します。");
    useProjectStore.getState().redo();
    expect(useProjectStore.getState().tts.text).toBe("更新後の文章");
  });
});
