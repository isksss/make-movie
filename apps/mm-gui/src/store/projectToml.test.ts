import { describe, expect, it } from "vitest";
import { initialProject } from "./projectStore";
import { parseProjectToml, serializeProjectToToml } from "./projectToml";

describe("projectToml", () => {
  it("ProjectStateをCore互換TOMLへserializeできる", () => {
    const toml = serializeProjectToToml({
      ...initialProject,
      layers: [
        {
          ...initialProject.layers[1],
          contentKind: "video",
          trimStart: 0.2,
          trimEnd: 0.8,
          transition: { kind: "wipe", duration: 1.2 },
          effects: [{ kind: "blur", duration: 0.5, amount: 2, x: 0, y: 0 }],
          animations: [
            {
              property: "opacity",
              easing: "ease_in_out",
              keyframes: [
                { time: 0, value: 0 },
                { time: 1, value: 1 },
              ],
            },
          ],
        },
      ],
    });

    expect(toml).toContain("[[assets]]");
    expect(toml).toContain("[[scenes]]");
    expect(toml).toContain('name = "Intro"');
    expect(toml).toContain("[[tracks]]");
    expect(toml).toContain("[[tracks.layers]]");
    expect(toml).toContain('[tracks.layers.content]\ntype = "video"');
    expect(toml).toContain("trim_start = 0.2");
    expect(toml).toContain("trim_end = 0.8");
    expect(toml).toContain("[tracks.layers.transform]");
    expect(toml).toContain("[tracks.layers.transition]");
    expect(toml).toContain('type = "wipe"');
    expect(toml).toContain("[[tracks.layers.effects]]");
    expect(toml).toContain("[[tracks.layers.animations.keyframes]]");
  });

  it("plugin宣言をserializeできる", () => {
    const toml = serializeProjectToToml({
      ...initialProject,
      plugins: [
        {
          repository: "github",
          owner: "isksss",
          repo: "gui-theme",
          version: "1.0.0",
        },
        {
          repository: "url",
          url: "https://example.com/plugin.wasm",
          version: "2.0.0",
        },
        {
          repository: "local",
          path: "./plugins/local-theme",
        },
      ],
    });

    expect(toml).toContain("[[plugin]]");
    expect(toml).toContain('repository = "github"');
    expect(toml).toContain('owner = "isksss"');
    expect(toml).toContain('repo = "gui-theme"');
    expect(toml).toContain('url = "https://example.com/plugin.wasm"');
    expect(toml).toContain('path = "./plugins/local-theme"');
  });

  it("TOMLからProjectStateへparseできる", () => {
    const parsed = parseProjectToml(`
[settings]
title = "Loaded"
width = 1280
height = 720
fps = 60
sample_rate = 44100
duration = 12
output = "output/loaded.mp4"

[[assets]]
id = "hero"
kind = "video"
path = "media/video/hero.mp4"

[[scenes]]
id = "scene-1"
name = "Opening"
start = 0
duration = 6

[[plugin]]
repository = "github"
owner = "isksss"
repo = "gui-theme"
version = "1.0.0"

[[plugin]]
repository = "local"
path = "./plugins/local-theme"

[[tracks]]
id = "v1"
name = "V1 Main Video"
kind = "video"

[[tracks.layers]]
id = "hero-layer"
label = "Hero"
start = 1
duration = 5
z_index = 2

[tracks.layers.content]
type = "video"
asset_id = "hero"
fit = "cover"
trim_start = 2
trim_end = 4

[tracks.layers.transform]
x = 10
y = 20
width = 320
height = 180
scale = 1
rotation = 0
opacity = 1
`);

    expect(parsed.settings.title).toBe("Loaded");
    expect(parsed.settings.sampleRate).toBe(44100);
    expect(parsed.assets[0]).toEqual({
      id: "hero",
      kind: "video",
      path: "media/video/hero.mp4",
    });
    expect(parsed.scenes[0]).toEqual({
      id: "scene-1",
      name: "Opening",
      start: 0,
      duration: 6,
    });
    expect(parsed.plugins).toEqual([
      {
        repository: "github",
        owner: "isksss",
        repo: "gui-theme",
        version: "1.0.0",
      },
      {
        repository: "local",
        path: "./plugins/local-theme",
      },
    ]);
    expect(parsed.tracks[0].id).toBe("v1");
    expect(parsed.layers[0]).toMatchObject({
      id: "hero-layer",
      trackId: "v1",
      label: "Hero",
      contentKind: "video",
      fit: "cover",
      trimStart: 2,
      trimEnd: 4,
      start: 1,
      duration: 5,
      zIndex: 2,
    });
    expect(parsed.layers[0].transform.width).toBe(320);
  });

  it("scenes未定義のTOMLではfallbackのscenesを維持する", () => {
    const parsed = parseProjectToml(`
[settings]
title = "No Scenes"
width = 1280
height = 720
fps = 30
sample_rate = 48000
duration = 3
output = "output/no-scenes.mp4"
`);

    expect(parsed.scenes).toEqual(initialProject.scenes);
    expect(parsed.plugins).toEqual([]);
  });

  it("audio trimはCore互換のミリ秒TOMLとして往復できる", () => {
    const toml = serializeProjectToToml({
      ...initialProject,
      layers: [
        {
          ...initialProject.layers.find((layer) => layer.id === "voice-audio")!,
          trimStart: 0.25,
          trimEnd: 1.5,
        },
      ],
    });

    expect(toml).toContain("trim_start = 250");
    expect(toml).toContain("trim_end = 1500");

    const parsed = parseProjectToml(toml);
    expect(parsed.layers[0]).toMatchObject({
      contentKind: "audio",
      trimStart: 0.25,
      trimEnd: 1.5,
    });
  });
});
