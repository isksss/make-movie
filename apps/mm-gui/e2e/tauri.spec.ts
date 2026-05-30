import { expect, test } from "@playwright/test";

type TauriCall = {
  cmd: string;
  args: Record<string, unknown>;
};

declare global {
  interface Window {
    __TAURI_TEST_CALLS__: TauriCall[];
    __TAURI_INTERNALS__: {
      invoke: (cmd: string, args: Record<string, unknown>) => Promise<string | null>;
      transformCallback: () => number;
      unregisterCallback: () => undefined;
    };
  }
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const calls: TauriCall[] = [];
    const importedProjectToml = (assetId: string, assetKind: string, assetPath: string) =>
      [
        "[settings]",
        'title = "E2E"',
        "width = 1080",
        "height = 1920",
        "fps = 30",
        "sample_rate = 48000",
        "duration = 1",
        'output = "output/movie.mp4"',
        'asset_mode = "copy"',
        "",
        "[[assets]]",
        'id = "hero"',
        'kind = "image"',
        'path = "media/image/hero.png"',
        "",
        "[[assets]]",
        `id = "${assetId}"`,
        `kind = "${assetKind}"`,
        `path = "${assetPath}"`,
        "",
        "[[tracks]]",
        'id = "v1"',
        'name = "V1 Main Video"',
        'kind = "video"',
        "",
        "[[tracks]]",
        'id = "a1"',
        'name = "A1 Voice"',
        'kind = "audio"',
        "",
        "[[tracks.layers]]",
        'id = "hero-layer"',
        'label = "Hero Layer"',
        "start = 0",
        "duration = 1",
        "z_index = 1",
        "",
        "[tracks.layers.content]",
        'type = "image"',
        'asset_id = "hero"',
        "",
        "[tracks.layers.transform]",
        "x = 0",
        "y = 0",
        "width = 320",
        "height = 180",
        "scale = 1",
        "rotation = 0",
        "opacity = 1",
        "",
        "[[tracks.layers]]",
        `id = "${assetId}-layer"`,
        `label = "${assetId === "import" ? "Import Layer" : "Drop Layer"}"`,
        "start = 0",
        "duration = 1",
        "z_index = 2",
        "",
        "[tracks.layers.content]",
        `type = "${assetKind}"`,
        `asset_id = "${assetId}"`,
        "",
        "[tracks.layers.transform]",
        "x = 0",
        "y = 0",
        "width = 320",
        "height = 180",
        "scale = 1",
        "rotation = 0",
        "opacity = 1",
        "",
      ].join("\n");
    window.__TAURI_TEST_CALLS__ = calls;
    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd: string, args: Record<string, unknown>) => {
        calls.push({ cmd, args });
        if (cmd === "load_project") {
          return [
            "[settings]",
            'title = "E2E"',
            "width = 1080",
            "height = 1920",
            "fps = 30",
            "sample_rate = 48000",
            "duration = 1",
            'output = "output/movie.mp4"',
            'asset_mode = "link"',
            'ffmpeg = "/opt/mm/ffmpeg"',
            "",
            "[[assets]]",
            'id = "hero"',
            'kind = "image"',
            'path = "media/image/hero.png"',
            "",
            "[[plugin]]",
            'repository = "github"',
            'owner = "isksss"',
            'repo = "gui-theme"',
            'version = "1.0.0"',
            "",
            "[[tracks]]",
            'id = "v1"',
            'name = "V1 Main Video"',
            'kind = "video"',
            "",
            "[[tracks.layers]]",
            'id = "hero-layer"',
            'label = "Hero Layer"',
            "start = 0",
            "duration = 1",
            "z_index = 1",
            "",
            "[tracks.layers.content]",
            'type = "image"',
            'asset_id = "hero"',
            "",
            "[tracks.layers.transform]",
            "x = 0",
            "y = 0",
            "width = 320",
            "height = 180",
            "scale = 1",
            "rotation = 0",
            "opacity = 1",
            "",
          ].join("\n");
        }
        if (cmd === "import_asset_into_project") {
          const sourcePath = String(args.sourcePath);
          if (sourcePath.endsWith("drop.wav")) {
            return importedProjectToml("drop", "audio", "media/audio/drop.wav");
          }
          return importedProjectToml("import", "image", "media/image/import.png");
        }
        return null;
      },
      transformCallback: () => 1,
      unregisterCallback: () => undefined,
    };
  });
});

test("toolbarからTauriコマンドを呼び出せる", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: "プロジェクトを開く" }).click();
  await expect(page.getByText("プロジェクトを開きました")).toBeVisible();
  await expect(page.getByRole("button", { name: "Hero Layer" })).toBeVisible();

  await page.getByRole("button", { name: "プロジェクトを保存" }).click();
  await expect(page.getByText("プロジェクトを保存しました")).toBeVisible();

  await page.getByRole("button", { name: "アセット取り込み" }).click();
  await expect(page.getByText("アセットを取り込みました")).toBeVisible();
  await expect(page.getByRole("button", { name: "import image", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Import Layer" })).toBeVisible();

  await page.getByRole("button", { name: "プロジェクトを保存" }).click();
  await expect(page.getByText("プロジェクトを保存しました")).toBeVisible();

  await page.getByRole("button", { name: "動画書き出し" }).click();
  await expect(page.getByText("動画を書き出しました")).toBeVisible();

  const calls = await page.evaluate(() => window.__TAURI_TEST_CALLS__);
  expect(calls).toEqual([
    { cmd: "load_project", args: { path: "mm.toml" } },
    {
      cmd: "save_project",
      args: expect.objectContaining({
        path: "mm.toml",
        toml: expect.stringContaining("[settings]"),
      }),
    },
    {
      cmd: "import_asset_into_project",
      args: { projectPath: "mm.toml", sourcePath: "media/image/import.png", kind: "image" },
    },
    {
      cmd: "save_project",
      args: expect.objectContaining({
        path: "mm.toml",
        toml: expect.stringContaining('id = "import-layer"'),
      }),
    },
    { cmd: "build_project", args: { path: "mm.toml" } },
  ]);
  expect(calls[1].args.toml).toEqual(expect.stringContaining("[[tracks.layers]]"));
  expect(calls[1].args.toml).toEqual(expect.stringContaining('label = "Hero Layer"'));
  expect(calls[1].args.toml).toEqual(expect.stringContaining('asset_id = "hero"'));
  expect(calls[1].args.toml).toEqual(expect.stringContaining('asset_mode = "link"'));
  expect(calls[1].args.toml).toEqual(expect.stringContaining('ffmpeg = "/opt/mm/ffmpeg"'));
  expect(calls[1].args.toml).toEqual(expect.stringContaining("[[plugin]]"));
  expect(calls[1].args.toml).toEqual(expect.stringContaining('repo = "gui-theme"'));
  expect(calls[3].args.toml).toEqual(expect.stringContaining('path = "media/image/import.png"'));
  expect(calls[3].args.toml).toEqual(expect.stringContaining('asset_id = "import"'));
});

test("Assetsペインへのdropでassetとlayerを取り込める", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: "プロジェクトを開く" }).click();
  await page.getByRole("region", { name: "アセット" }).dispatchEvent("dragover", {
    dataTransfer: await page.evaluateHandle(() => new DataTransfer()),
  });
  await page.getByRole("region", { name: "アセット" }).dispatchEvent("drop", {
    dataTransfer: await page.evaluateHandle(() => {
      const dataTransfer = new DataTransfer();
      dataTransfer.setData("text/plain", "/tmp/drop.wav");
      return dataTransfer;
    }),
  });

  await expect(page.getByText("アセットを取り込みました")).toBeVisible();
  await expect(page.getByRole("button", { name: "drop audio", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Drop Layer" })).toBeVisible();

  const calls = await page.evaluate(() => window.__TAURI_TEST_CALLS__);
  expect(calls).toEqual([
    { cmd: "load_project", args: { path: "mm.toml" } },
    {
      cmd: "import_asset_into_project",
      args: { projectPath: "mm.toml", sourcePath: "/tmp/drop.wav", kind: "audio" },
    },
  ]);
});

test("TTS編集は保存TOMLのVoiceレイヤーに反映される", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("言語").selectOption("en");

  const ttsEditor = page.locator(".tts-editor");
  await ttsEditor.getByLabel("Speaker").fill("四国めたん");
  await ttsEditor.locator("textarea").fill("保存される文章");
  await ttsEditor.getByRole("spinbutton", { name: "Speed", exact: true }).fill("1.4");
  await ttsEditor.getByRole("spinbutton", { name: "Pitch", exact: true }).fill("0.2");
  await ttsEditor.getByRole("textbox", { name: "Emotion", exact: true }).fill("happy");

  await page.getByRole("button", { name: "Save project" }).click();
  await expect(page.getByText("Project saved")).toBeVisible();

  const calls = await page.evaluate(() => window.__TAURI_TEST_CALLS__);
  expect(calls).toEqual([
    {
      cmd: "save_project",
      args: expect.objectContaining({
        path: "mm.toml",
        toml: expect.stringContaining('speaker = "四国めたん"'),
      }),
    },
  ]);
  expect(calls[0].args.toml).toEqual(expect.stringContaining('type = "voice"'));
  expect(calls[0].args.toml).toEqual(expect.stringContaining('text = "保存される文章"'));
  expect(calls[0].args.toml).toEqual(expect.stringContaining("speed = 1.4"));
  expect(calls[0].args.toml).toEqual(expect.stringContaining("pitch = 0.2"));
  expect(calls[0].args.toml).toEqual(expect.stringContaining('emotion = "happy"'));
});

test("Plugin Managerからplugin操作を呼び出せる", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("region", { name: "プラグイン管理" })).toBeVisible();
  await page.getByRole("button", { name: "インストール VOICEVOX" }).click();
  await expect(page.getByText("プラグインをインストールしました")).toBeVisible();

  await page.getByRole("button", { name: "更新 AivisSpeech" }).click();
  await expect(page.getByText("プラグインを更新しました")).toBeVisible();

  await page.getByRole("button", { name: "削除 Template Pack" }).click();
  await expect(page.getByText("プラグインを削除しました")).toBeVisible();

  const calls = await page.evaluate(() => window.__TAURI_TEST_CALLS__);
  expect(calls).toEqual([
    { cmd: "install_plugin", args: { name: "VOICEVOX" } },
    { cmd: "update_plugin", args: { name: "AivisSpeech" } },
    { cmd: "remove_plugin", args: { name: "Template Pack" } },
  ]);
});

test("Plugin ManagerはProject plugin宣言を表示して操作できる", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: "プロジェクトを開く" }).click();
  await expect(page.getByText("gui-theme")).toBeVisible();

  await page.getByRole("button", { name: "インストール gui-theme" }).click();
  await expect(page.getByText("プラグインをインストールしました")).toBeVisible();

  const calls = await page.evaluate(() => window.__TAURI_TEST_CALLS__);
  expect(calls).toContainEqual({ cmd: "install_plugin", args: { name: "gui-theme" } });
});

test("Plugin Managerは英語表示でもplugin操作を呼び出せる", async ({ page }) => {
  await page.goto("/");

  await page.getByLabel("言語").selectOption("en");
  await expect(page.getByRole("region", { name: "Plugin Manager" })).toBeVisible();

  await page.getByRole("button", { name: "Install VOICEVOX" }).click();
  await expect(page.getByText("Plugin installed")).toBeVisible();

  const calls = await page.evaluate(() => window.__TAURI_TEST_CALLS__);
  expect(calls).toEqual([{ cmd: "install_plugin", args: { name: "VOICEVOX" } }]);
});
