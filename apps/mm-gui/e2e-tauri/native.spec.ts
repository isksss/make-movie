import { existsSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "./fixtures";

const projectPath = join(process.cwd(), "src-tauri", "mm.toml");

test.beforeEach(() => {
  rmSync(projectPath, { force: true });
});

test.afterEach(() => {
  rmSync(projectPath, { force: true });
});

test("Tauri実アプリで表示と保存IPCを検証できる", async ({ tauriPage }) => {
  await tauriPage.waitForFunction("document.body.innerText.includes('make-movie')", 30_000);
  await expect(tauriPage.locator(".brand")).toContainText("make-movie");
  await expect(tauriPage.locator(".assets-pane")).toContainText("アセット");

  await tauriPage.locator('button[title="プロジェクトを保存"]').click();
  await tauriPage.waitForFunction(
    "document.body.innerText.includes('プロジェクトを保存しました')",
    10_000,
  );

  expect(existsSync(projectPath)).toBe(true);
  const toml = readFileSync(projectPath, "utf8");
  expect(toml).toContain("[settings]");
  expect(toml).toContain("[[tracks.layers]]");
  expect(toml).toContain('title = "make-movie"');
});
