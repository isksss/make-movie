import { describe, expect, it } from "vitest";
import { initialProject } from "./projectStore";
import { parseProjectToml, serializeProjectToToml } from "./projectToml";

describe("projectToml", () => {
  it("ProjectStateをCore互換TOMLへserializeできる", () => {
    const toml = serializeProjectToToml({
      ...initialProject,
      settings: {
        ...initialProject.settings,
        assetMode: "link",
        ffmpeg: "/usr/bin/ffmpeg",
      },
      layers: [
        {
          ...initialProject.layers[1],
          contentKind: "video",
          trimStart: 0.2,
          trimEnd: 0.8,
          transition: {
            kind: "wipe",
            duration: 1.2,
            wipeShape: "rounded_rect",
            wipeRadius: 28,
            wipeBorderColor: "#ff0000",
            wipeBorderWidth: 3,
            wipeShadowColor: "#0000ff",
            wipeShadowOffsetX: 4,
            wipeShadowOffsetY: 5,
            wipeShadowBlur: 6,
          },
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
    expect(toml).toContain('asset_mode = "link"');
    expect(toml).toContain('ffmpeg = "/usr/bin/ffmpeg"');
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
    expect(toml).toContain("[tracks.layers.transition.shape]");
    expect(toml).toContain('type = "rounded_rect"');
    expect(toml).toContain("radius = 28");
    expect(toml).toContain("[tracks.layers.transition.shape.border]");
    expect(toml).toContain('color = "#ff0000"');
    expect(toml).toContain("width = 3");
    expect(toml).toContain("[tracks.layers.transition.shape.shadow]");
    expect(toml).toContain('color = "#0000ff"');
    expect(toml).toContain("offset_x = 4");
    expect(toml).toContain("offset_y = 5");
    expect(toml).toContain("blur = 6");
    expect(toml).toContain("[[tracks.layers.effects]]");
    expect(toml).toContain("[[tracks.layers.animations.keyframes]]");
  });

  it("Text layer styleをTOMLへserializeできる", () => {
    const toml = serializeProjectToToml({
      ...initialProject,
      layers: [
        {
          ...initialProject.layers[0],
          text: {
            text: "複数行\nタイトル",
            fontSize: 72,
            color: "#ffcc00",
            letterSpacing: 1.5,
            lineSpacing: 1.4,
            align: "left",
            stroke: { color: "#111111", width: 3 },
            shadow: { color: "#222222", offsetX: 4, offsetY: 5, blur: 6 },
          },
        },
      ],
    });

    expect(toml).toContain('text = "複数行\\nタイトル"');
    expect(toml).toContain("font_size = 72");
    expect(toml).toContain('color = "#ffcc00"');
    expect(toml).toContain("letter_spacing = 1.5");
    expect(toml).toContain('align = "left"');
    expect(toml).toContain("[tracks.layers.content.stroke]");
    expect(toml).toContain("[tracks.layers.content.shadow]");
  });

  it("Image maskのradiusとSVG pathをTOMLへserializeできる", () => {
    const roundedToml = serializeProjectToToml({
      ...initialProject,
      layers: [
        {
          ...initialProject.layers[1],
          mask: "rounded_rect",
          maskRadius: 28,
        },
      ],
    });

    expect(roundedToml).toContain("[tracks.layers.content.mask]");
    expect(roundedToml).toContain('type = "rounded_rect"');
    expect(roundedToml).toContain("radius = 28");

    const svgToml = serializeProjectToToml({
      ...initialProject,
      layers: [
        {
          ...initialProject.layers[1],
          mask: "svg",
          maskPath: "media/mask/window.svg",
        },
      ],
    });

    expect(svgToml).toContain("[tracks.layers.content.mask]");
    expect(svgToml).toContain('type = "svg"');
    expect(svgToml).toContain('path = "media/mask/window.svg"');
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
asset_mode = "link"
ffmpeg = "/opt/mm/ffmpeg"

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
    expect(parsed.settings.assetMode).toBe("link");
    expect(parsed.settings.ffmpeg).toBe("/opt/mm/ffmpeg");
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
      assetId: "hero",
      fit: "cover",
      trimStart: 2,
      trimEnd: 4,
      start: 1,
      duration: 5,
      zIndex: 2,
    });
    expect(parsed.layers[0].transform.width).toBe(320);
  });

  it("Image maskとcropをTOMLからparseできる", () => {
    const parsed = parseProjectToml(`
[settings]
title = "Mask"
width = 1280
height = 720
fps = 30
sample_rate = 48000
duration = 3
output = "output/mask.mp4"

[[assets]]
id = "hero"
kind = "image"
path = "media/image/hero.png"

[[tracks]]
id = "v2"
name = "V2 Overlay"
kind = "video"

[[tracks.layers]]
id = "hero-layer"
label = "Hero"
start = 0
duration = 3
z_index = 2

[tracks.layers.content]
type = "image"
asset_id = "hero"
fit = "contain"

[tracks.layers.content.crop]
x = 10
y = 20
width = 320
height = 180

[tracks.layers.content.mask]
type = "svg"
path = "media/mask/window.svg"

[tracks.layers.transform]
x = 0
y = 0
width = 320
height = 180
scale = 1
rotation = 0
opacity = 1
`);

    expect(parsed.layers[0]).toMatchObject({
      crop: { x: 10, y: 20, width: 320, height: 180 },
      mask: "svg",
      maskPath: "media/mask/window.svg",
    });
  });

  it("RoundedRect mask radiusをTOMLからparseできる", () => {
    const parsed = parseProjectToml(`
[settings]
title = "Mask Radius"
width = 1280
height = 720
fps = 30
sample_rate = 48000
duration = 3
output = "output/mask-radius.mp4"

[[assets]]
id = "hero"
kind = "image"
path = "media/image/hero.png"

[[tracks]]
id = "v2"
name = "V2 Overlay"
kind = "video"

[[tracks.layers]]
id = "hero-layer"
label = "Hero"
start = 0
duration = 3
z_index = 2

[tracks.layers.content]
type = "image"
asset_id = "hero"

[tracks.layers.content.mask]
type = "rounded_rect"
radius = 32

[tracks.layers.transform]
x = 0
y = 0
width = 320
height = 180
scale = 1
rotation = 0
opacity = 1
`);

    expect(parsed.layers[0]).toMatchObject({
      mask: "rounded_rect",
      maskRadius: 32,
    });
  });

  it("同種assetが複数あってもlayerごとのasset_idを保持してserializeできる", () => {
    const parsed = parseProjectToml(`
[settings]
title = "Asset Ref"
width = 1280
height = 720
fps = 30
sample_rate = 48000
duration = 3
output = "output/asset-ref.mp4"

[[assets]]
id = "first"
kind = "image"
path = "media/image/first.png"

[[assets]]
id = "second"
kind = "image"
path = "media/image/second.png"

[[tracks]]
id = "v1"
name = "V1"
kind = "video"

[[tracks.layers]]
id = "image-layer"
label = "Image"
start = 0
duration = 3
z_index = 1

[tracks.layers.content]
type = "image"
asset_id = "second"

[tracks.layers.transform]
x = 0
y = 0
width = 320
height = 180
scale = 1
rotation = 0
opacity = 1
`);

    expect(parsed.layers[0].assetId).toBe("second");

    const toml = serializeProjectToToml(parsed);
    expect(toml).toContain('asset_id = "second"');
    expect(toml).not.toContain('asset_id = "first"');
  });

  it("Text layer styleをTOMLからparseできる", () => {
    const parsed = parseProjectToml(`
[settings]
title = "Text"
width = 1280
height = 720
fps = 30
sample_rate = 48000
duration = 3
output = "output/text.mp4"

[[tracks]]
id = "v3"
name = "V3 Text"
kind = "video"

[[tracks.layers]]
id = "title"
label = "Title"
start = 0
duration = 3
z_index = 10

[tracks.layers.content]
type = "text"
text = "複数行\\nタイトル"
font_size = 72
color = "#ffcc00"
letter_spacing = 1.5
line_spacing = 1.4
align = "right"

[tracks.layers.content.stroke]
color = "#111111"
width = 3

[tracks.layers.content.shadow]
color = "#222222"
offset_x = 4
offset_y = 5
blur = 6

[tracks.layers.transform]
x = 0
y = 0
width = 320
height = 180
scale = 1
rotation = 0
opacity = 1
`);

    expect(parsed.layers[0].text).toMatchObject({
      text: "複数行\nタイトル",
      fontSize: 72,
      color: "#ffcc00",
      letterSpacing: 1.5,
      lineSpacing: 1.4,
      align: "right",
      stroke: { color: "#111111", width: 3 },
      shadow: { color: "#222222", offsetX: 4, offsetY: 5, blur: 6 },
    });
  });

  it("Wipe transitionの角丸、枠線、影をTOMLからparseできる", () => {
    const parsed = parseProjectToml(`
[settings]
title = "Transition"
width = 1280
height = 720
fps = 30
sample_rate = 48000
duration = 3
output = "output/transition.mp4"

[[tracks]]
id = "v1"
name = "V1"
kind = "video"

[[tracks.layers]]
id = "intro"
label = "Intro"
start = 0
duration = 3
z_index = 1

[tracks.layers.content]
type = "image"

[tracks.layers.transform]
x = 0
y = 0
width = 320
height = 180
scale = 1
rotation = 0
opacity = 1

[tracks.layers.transition]
type = "wipe"
duration = 1.2

[tracks.layers.transition.shape]
type = "rounded_rect"
radius = 28

[tracks.layers.transition.shape.border]
color = "#ff0000"
width = 3

[tracks.layers.transition.shape.shadow]
color = "#0000ff"
offset_x = 4
offset_y = 5
blur = 6
`);

    expect(parsed.layers[0].transition).toMatchObject({
      kind: "wipe",
      duration: 1.2,
      wipeShape: "rounded_rect",
      wipeRadius: 28,
      wipeBorderColor: "#ff0000",
      wipeBorderWidth: 3,
      wipeShadowColor: "#0000ff",
      wipeShadowOffsetX: 4,
      wipeShadowOffsetY: 5,
      wipeShadowBlur: 6,
    });
  });

  it("Voice layer TTS設定をTOMLとして往復できる", () => {
    const toml = serializeProjectToToml({
      ...initialProject,
      layers: [
        {
          ...initialProject.layers.find((layer) => layer.id === "voice-main")!,
          voice: {
            provider: "coeiro_ink",
            speaker: "四国めたん",
            text: "更新後の文章",
            speed: 1.4,
            pitch: 0.2,
            emotion: "happy",
          },
        },
      ],
    });

    expect(toml).toContain('provider = "coeiro_ink"');
    expect(toml).toContain('speaker = "四国めたん"');
    expect(toml).toContain('text = "更新後の文章"');
    expect(toml).toContain("speed = 1.4");
    expect(toml).toContain("pitch = 0.2");
    expect(toml).toContain('emotion = "happy"');

    const parsed = parseProjectToml(toml);
    expect(parsed.layers[0]).toMatchObject({
      contentKind: "voice",
      label: "更新後の文章",
      voice: {
        provider: "coeiro_ink",
        speaker: "四国めたん",
        text: "更新後の文章",
        speed: 1.4,
        pitch: 0.2,
        emotion: "happy",
      },
    });
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

  it("ffmpeg未指定ではffmpeg行をserializeしない", () => {
    const toml = serializeProjectToToml({
      ...initialProject,
      settings: {
        ...initialProject.settings,
        ffmpeg: null,
      },
    });

    expect(toml).not.toContain("\nffmpeg = ");
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
