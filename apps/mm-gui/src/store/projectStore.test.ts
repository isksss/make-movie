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
});
