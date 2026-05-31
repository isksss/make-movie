import { existsSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "./fixtures";

const projectPath = join(process.cwd(), "src-tauri", "mm.toml");
const windowNotReadyMessage = "window 'main' not found after retries";

test.beforeEach(() => {
  rmSync(projectPath, { force: true });
});

test.afterEach(() => {
  rmSync(projectPath, { force: true });
});

async function waitForTauriWindow(tauriPage: {
  waitForFunction: (expression: string, timeout?: number) => Promise<void>;
}) {
  const deadline = Date.now() + 60_000;
  let lastError: unknown;

  while (Date.now() < deadline) {
    try {
      await tauriPage.waitForFunction("document.body.innerText.includes('make-movie')", 10_000);
      return;
    } catch (error) {
      lastError = error;
      if (!String(error).includes(windowNotReadyMessage)) {
        throw error;
      }
      await new Promise((resolve) => setTimeout(resolve, 1_000));
    }
  }

  throw lastError;
}

test("Tauri実アプリで表示と保存IPCを検証できる", async ({ tauriPage }) => {
  await waitForTauriWindow(tauriPage);
  await tauriPage.locator(".language-select select").selectOption("ja");
  await expect(tauriPage.locator(".brand")).toContainText("make-movie");
  await expect(tauriPage.locator(".assets-pane")).toContainText("アセット");

  await tauriPage.evaluate(
    `(() => {
      const toml = [
        "[settings]",
        'title = "make-movie"',
        "width = 1080",
        "height = 1920",
        "fps = 30",
        "sample_rate = 48000",
        "duration = 1",
        'output = "output/movie.mp4"',
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
        "start = 0",
        "duration = 1",
        "z_index = 1",
        "",
        "[tracks.layers.content]",
        'type = "image"',
        'asset_id = "hero"',
        "",
      ].join("\\n");
      return window.__TAURI_INTERNALS__.invoke("save_project", { path: "mm.toml", toml });
    })()`,
  );

  expect(existsSync(projectPath)).toBe(true);
  const toml = readFileSync(projectPath, "utf8");
  expect(toml).toContain("[settings]");
  expect(toml).toContain("[[tracks.layers]]");
  expect(toml).toContain('title = "make-movie"');
});
