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
          return '[settings]\ntitle = "E2E"\nwidth = 1080\nheight = 1920\nfps = 30\nsample_rate = 48000\nduration = 1\noutput = "output/movie.mp4"\nasset_mode = "copy"\n';
        }
        if (cmd === "import_asset") {
          return 'id = "import"\nkind = "image"\npath = "media/image/import.png"\n';
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

  await page.getByRole("button", { name: "Open project" }).click();
  await expect(page.getByText("プロジェクトを開きました")).toBeVisible();

  await page.getByRole("button", { name: "Save project" }).click();
  await expect(page.getByText("プロジェクトを保存しました")).toBeVisible();

  await page.getByRole("button", { name: "Import asset" }).click();
  await expect(page.getByText("アセットを取り込みました")).toBeVisible();

  await page.getByRole("button", { name: "Build movie" }).click();
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
      cmd: "import_asset",
      args: { projectRoot: ".", sourcePath: "media/image/import.png", kind: "image" },
    },
    { cmd: "build_project", args: { path: "mm.toml" } },
  ]);
});
