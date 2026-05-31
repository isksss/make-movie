import { describe, expect, it } from "vitest";
import { initialProject } from "./projectStore";
import { applyExternalAnalysisJson, sampleExternalAnalysisJson } from "./externalAnalysis";

describe("externalAnalysis", () => {
  it("ProjectStateをTOML経由でexternal analysis commandへ渡して更新結果をparseする", async () => {
    const updated = await applyExternalAnalysisJson(
      initialProject,
      "intro-image",
      sampleExternalAnalysisJson,
      async (projectToml, layerId, analysisJson) => {
        expect(layerId).toBe("intro-image");
        expect(projectToml).toContain("[[tracks.layers]]");
        expect(analysisJson).toContain("source_width");
        return `${projectToml}
[[tracks.layers.animations]]
property = "x"
easing = "linear"

[[tracks.layers.animations.keyframes]]
time = 0
value = 100
`;
      },
    );

    expect(updated.layers.at(-1)?.animations[0]).toMatchObject({
      property: "x",
      easing: "linear",
      keyframes: [{ time: 0, value: 100 }],
    });
  });
});
