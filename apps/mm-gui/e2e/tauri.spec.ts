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
            'asset_mode = "copy"',
            "",
            "[[assets]]",
            'id = "hero"',
            'kind = "image"',
            'path = "media/image/hero.png"',
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
          return [
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
            'id = "import"',
            'kind = "image"',
            'path = "media/image/import.png"',
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
            "[[tracks.layers]]",
            'id = "import-layer"',
            'label = "Import Layer"',
            "start = 0",
            "duration = 1",
            "z_index = 2",
            "",
            "[tracks.layers.content]",
            'type = "image"',
            'asset_id = "import"',
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
  expect(calls[3].args.toml).toEqual(expect.stringContaining('path = "media/image/import.png"'));
});
