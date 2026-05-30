import { describe, expect, it } from "vitest";
import { initialProject } from "../store/projectStore";
import { activePreviewItems, colorForLayer, createPreviewViewport } from "./previewRenderer";

describe("previewRenderer", () => {
  it("project aspect ratio に収まる viewport を計算する", () => {
    const viewport = createPreviewViewport(800, 600, 1080, 1920);

    expect(viewport.height).toBe(600);
    expect(viewport.width).toBeCloseTo(337.5);
    expect(viewport.offsetX).toBeCloseTo(231.25);
  });

  it("現在時刻に active な layer を z_index 順で返す", () => {
    const items = activePreviewItems(initialProject, 1);

    expect(items.map((item) => item.layer.id)).toEqual([
      "voice-main",
      "voice-audio",
      "intro-image",
      "title",
      "subtitle-main",
    ]);
  });

  it("layer kind ごとに描画色を変える", () => {
    expect(colorForLayer(initialProject.layers[0])).toBe("#58a9b8");
    expect(colorForLayer(initialProject.layers[1])).toBe("#79b97a");
  });
});
