import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e-tauri",
  fullyParallel: false,
  reporter: "list",
  timeout: 180_000,
  use: {
    mode: "tauri",
    trace: "on-first-retry",
  },
  workers: 1,
});
