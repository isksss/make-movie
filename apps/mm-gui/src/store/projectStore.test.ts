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

  it("TTS設定を更新しUndo/Redoできる", () => {
    useProjectStore.getState().updateTts({
      speaker: "四国めたん",
      text: "更新後の文章",
      speed: 1.4,
      pitch: 0.2,
      emotion: "happy",
    });

    expect(useProjectStore.getState().tts).toMatchObject({
      speaker: "四国めたん",
      text: "更新後の文章",
      speed: 1.4,
      pitch: 0.2,
      emotion: "happy",
    });
    useProjectStore.getState().undo();
    expect(useProjectStore.getState().tts).toMatchObject({
      speaker: "ずんだもん",
      text: "今日のニュースを解説します。",
      speed: 1,
      pitch: 0,
      emotion: "neutral",
    });
    useProjectStore.getState().redo();
    expect(useProjectStore.getState().tts).toMatchObject({
      speaker: "四国めたん",
      text: "更新後の文章",
      speed: 1.4,
      pitch: 0.2,
      emotion: "happy",
    });
  });

  it("layer を指定時刻でCutしUndo/Redoできる", () => {
    useProjectStore.getState().splitLayer("intro-image", 2.5);

    let layers = useProjectStore.getState().project.layers;
    expect(layers.find((item) => item.id === "intro-image")?.duration).toBe(2);
    expect(layers.find((item) => item.id === "intro-image-split")?.start).toBe(2.5);
    expect(layers.find((item) => item.id === "intro-image-split")?.duration).toBe(5);
    expect(useProjectStore.getState().selectedLayerId).toBe("intro-image-split");

    useProjectStore.getState().undo();
    layers = useProjectStore.getState().project.layers;
    expect(layers.find((item) => item.id === "intro-image")?.duration).toBe(7);
    expect(layers.some((item) => item.id === "intro-image-split")).toBe(false);

    useProjectStore.getState().redo();
    layers = useProjectStore.getState().project.layers;
    expect(layers.some((item) => item.id === "intro-image-split")).toBe(true);
  });

  it("layer 範囲外のCutでは履歴を追加しない", () => {
    useProjectStore.getState().splitLayer("intro-image", 0.25);

    expect(useProjectStore.getState().project.layers).toHaveLength(initialProject.layers.length);
    expect(useProjectStore.getState().canUndo).toBe(false);
  });

  it("layer を複製しUndo/Redoできる", () => {
    useProjectStore.getState().duplicateLayer("intro-image");

    let layers = useProjectStore.getState().project.layers;
    const copiedLayer = layers.find((item) => item.id === "intro-image-copy");
    expect(copiedLayer?.label).toBe("Intro Image Copy");
    expect(copiedLayer?.start).toBe(7.5);
    expect(copiedLayer?.duration).toBe(7);
    expect(useProjectStore.getState().selectedLayerId).toBe("intro-image-copy");

    useProjectStore.getState().undo();
    layers = useProjectStore.getState().project.layers;
    expect(layers.some((item) => item.id === "intro-image-copy")).toBe(false);

    useProjectStore.getState().redo();
    layers = useProjectStore.getState().project.layers;
    expect(layers.some((item) => item.id === "intro-image-copy")).toBe(true);
  });

  it("layer を削除しUndo/Redoできる", () => {
    useProjectStore.getState().deleteLayer("intro-image");

    let layers = useProjectStore.getState().project.layers;
    expect(layers.some((item) => item.id === "intro-image")).toBe(false);
    expect(useProjectStore.getState().selectedLayerId).toBe("subtitle-main");

    useProjectStore.getState().undo();
    layers = useProjectStore.getState().project.layers;
    expect(layers.some((item) => item.id === "intro-image")).toBe(true);

    useProjectStore.getState().redo();
    layers = useProjectStore.getState().project.layers;
    expect(layers.some((item) => item.id === "intro-image")).toBe(false);
  });

  it("layer transform を更新しUndo/Redoできる", () => {
    useProjectStore.getState().updateLayerTransform("intro-image", {
      width: 420,
      height: 240,
      rotation: 15,
      opacity: 0.5,
    });

    let layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.transform.width).toBe(420);
    expect(layer?.transform.height).toBe(240);
    expect(layer?.transform.rotation).toBe(15);
    expect(layer?.transform.opacity).toBe(0.5);

    useProjectStore.getState().undo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.transform.width).toBe(840);
    expect(layer?.transform.height).toBe(480);
    expect(layer?.transform.rotation).toBe(0);
    expect(layer?.transform.opacity).toBe(1);

    useProjectStore.getState().redo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.transform.width).toBe(420);
    expect(layer?.transform.height).toBe(240);
  });

  it("layer のCrop/Mask/Fitを更新しUndo/Redoできる", () => {
    useProjectStore.getState().updateLayerVisual("intro-image", {
      crop: { x: 10, y: 20, width: 320, height: 180 },
      mask: "rounded_rect",
      fit: "blur_background",
    });

    let layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.crop).toEqual({ x: 10, y: 20, width: 320, height: 180 });
    expect(layer?.mask).toBe("rounded_rect");
    expect(layer?.fit).toBe("blur_background");

    useProjectStore.getState().undo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.crop).toEqual({ x: 0, y: 0, width: 0, height: 0 });
    expect(layer?.mask).toBe("none");
    expect(layer?.fit).toBe("contain");

    useProjectStore.getState().redo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.crop.width).toBe(320);
    expect(layer?.mask).toBe("rounded_rect");
    expect(layer?.fit).toBe("blur_background");
  });

  it("layer transition を更新しUndo/Redoできる", () => {
    useProjectStore.getState().updateLayerTransition("intro-image", {
      kind: "wipe",
      duration: 1.2,
    });

    let layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.transition.kind).toBe("wipe");
    expect(layer?.transition.duration).toBe(1.2);

    useProjectStore.getState().undo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.transition.kind).toBe("none");
    expect(layer?.transition.duration).toBe(0.5);

    useProjectStore.getState().redo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.transition.kind).toBe("wipe");
    expect(layer?.transition.duration).toBe(1.2);
  });

  it("layer effect を更新しUndo/Redoできる", () => {
    useProjectStore.getState().updateLayerEffect("intro-image", {
      kind: "blur",
      amount: 2.5,
      duration: 1.2,
    });

    let layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.effects[0]).toMatchObject({
      kind: "blur",
      amount: 2.5,
      duration: 1.2,
    });

    useProjectStore.getState().undo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.effects).toEqual([]);

    useProjectStore.getState().redo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.effects[0]?.kind).toBe("blur");

    useProjectStore.getState().updateLayerEffect("intro-image", { kind: "none" });
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.effects).toEqual([]);
  });

  it("layer animation keyframe を更新しUndo/Redoできる", () => {
    useProjectStore.getState().updateLayerAnimation("intro-image", {
      property: "opacity",
      easing: "ease_in_out",
      keyframes: [
        { time: 0, value: 0 },
        { time: 1.5, value: 1 },
      ],
    });

    let layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.animations[0]).toMatchObject({
      property: "opacity",
      easing: "ease_in_out",
    });
    expect(layer?.animations[0]?.keyframes[1]).toEqual({ time: 1.5, value: 1 });

    useProjectStore.getState().undo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.animations).toEqual([]);

    useProjectStore.getState().redo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.animations[0]?.property).toBe("opacity");

    useProjectStore.getState().updateLayerAnimation("intro-image", { property: "none" });
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.animations).toEqual([]);
  });
});
