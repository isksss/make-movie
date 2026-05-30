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

  it("Text layer styleを更新しUndo/Redoできる", () => {
    useProjectStore.getState().updateLayerText("title", {
      text: "更新後タイトル",
      fontSize: 72,
      color: "#ffcc00",
      align: "left",
      stroke: { color: "#111111", width: 3 },
      shadow: { color: "#222222", offsetX: 4, offsetY: 5, blur: 6 },
      gradient: {
        enabled: true,
        startColor: "#ff0000",
        endColor: "#0000ff",
        direction: "horizontal",
      },
    });

    let layer = useProjectStore.getState().project.layers.find((item) => item.id === "title");
    expect(layer?.label).toBe("更新後タイトル");
    expect(layer?.text).toMatchObject({
      text: "更新後タイトル",
      fontSize: 72,
      color: "#ffcc00",
      align: "left",
      stroke: { color: "#111111", width: 3 },
      shadow: { color: "#222222", offsetX: 4, offsetY: 5, blur: 6 },
      gradient: {
        enabled: true,
        startColor: "#ff0000",
        endColor: "#0000ff",
        direction: "horizontal",
      },
    });

    useProjectStore.getState().undo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "title");
    expect(layer?.text.text).toBe("Title Text");

    useProjectStore.getState().redo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "title");
    expect(layer?.text.fontSize).toBe(72);
  });

  it("Voice layer TTS設定を更新しUndo/Redoできる", () => {
    useProjectStore.getState().updateLayerVoice("voice-main", {
      provider: "aivis_speech",
      speaker: "四国めたん",
      text: "更新後の文章",
      speed: 1.4,
      pitch: 0.2,
      emotion: "happy",
    });

    let layer = useProjectStore.getState().project.layers.find((item) => item.id === "voice-main");
    expect(layer?.label).toBe("更新後の文章");
    expect(layer?.voice).toMatchObject({
      provider: "aivis_speech",
      speaker: "四国めたん",
      text: "更新後の文章",
      speed: 1.4,
      pitch: 0.2,
      emotion: "happy",
    });

    useProjectStore.getState().undo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "voice-main");
    expect(layer?.voice.text).toBe("今日のニュースを解説します。");

    useProjectStore.getState().redo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "voice-main");
    expect(layer?.voice.speed).toBe(1.4);
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
      maskRadius: 24,
      fit: "blur_background",
    });

    let layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.crop).toEqual({ x: 10, y: 20, width: 320, height: 180 });
    expect(layer?.mask).toBe("rounded_rect");
    expect(layer?.maskRadius).toBe(24);
    expect(layer?.fit).toBe("blur_background");

    useProjectStore.getState().undo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.crop).toEqual({ x: 0, y: 0, width: 0, height: 0 });
    expect(layer?.mask).toBe("none");
    expect(layer?.maskRadius).toBeUndefined();
    expect(layer?.fit).toBe("contain");

    useProjectStore.getState().redo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.crop.width).toBe(320);
    expect(layer?.mask).toBe("rounded_rect");
    expect(layer?.maskRadius).toBe(24);
    expect(layer?.fit).toBe("blur_background");
  });

  it("SVG mask pathを更新しUndo/Redoできる", () => {
    useProjectStore.getState().updateLayerVisual("intro-image", {
      mask: "svg",
      maskPath: "media/mask/window.svg",
    });

    let layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.mask).toBe("svg");
    expect(layer?.maskPath).toBe("media/mask/window.svg");

    useProjectStore.getState().undo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.mask).toBe("none");
    expect(layer?.maskPath).toBeUndefined();

    useProjectStore.getState().redo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.mask).toBe("svg");
    expect(layer?.maskPath).toBe("media/mask/window.svg");
  });

  it("layer trimを更新しUndo/Redoできる", () => {
    useProjectStore.getState().updateLayerTrim("voice-audio", {
      trimStart: 0.25,
      trimEnd: 1.5,
    });

    let layer = useProjectStore.getState().project.layers.find((item) => item.id === "voice-audio");
    expect(layer?.trimStart).toBe(0.25);
    expect(layer?.trimEnd).toBe(1.5);

    useProjectStore.getState().undo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "voice-audio");
    expect(layer?.trimStart).toBe(0);
    expect(layer?.trimEnd).toBe(0);

    useProjectStore.getState().redo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "voice-audio");
    expect(layer?.trimStart).toBe(0.25);
    expect(layer?.trimEnd).toBe(1.5);
  });

  it("layer transition を更新しUndo/Redoできる", () => {
    useProjectStore.getState().updateLayerTransition("intro-image", {
      kind: "wipe",
      duration: 1.2,
      wipeShape: "rounded_rect",
      wipeRadius: 32,
      wipeBorderColor: "#ff0000",
      wipeBorderWidth: 3,
      wipeShadowColor: "#0000ff",
      wipeShadowOffsetX: 4,
      wipeShadowOffsetY: 5,
      wipeShadowBlur: 6,
    });

    let layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.transition.kind).toBe("wipe");
    expect(layer?.transition.duration).toBe(1.2);
    expect(layer?.transition.wipeShape).toBe("rounded_rect");
    expect(layer?.transition.wipeRadius).toBe(32);
    expect(layer?.transition.wipeBorderColor).toBe("#ff0000");
    expect(layer?.transition.wipeBorderWidth).toBe(3);
    expect(layer?.transition.wipeShadowColor).toBe("#0000ff");
    expect(layer?.transition.wipeShadowOffsetX).toBe(4);
    expect(layer?.transition.wipeShadowOffsetY).toBe(5);
    expect(layer?.transition.wipeShadowBlur).toBe(6);

    useProjectStore.getState().undo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.transition.kind).toBe("none");
    expect(layer?.transition.duration).toBe(0.5);
    expect(layer?.transition.wipeShape).toBe("circle");

    useProjectStore.getState().redo();
    layer = useProjectStore.getState().project.layers.find((item) => item.id === "intro-image");
    expect(layer?.transition.kind).toBe("wipe");
    expect(layer?.transition.duration).toBe(1.2);
    expect(layer?.transition.wipeShape).toBe("rounded_rect");
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
