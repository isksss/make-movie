import { createTauriTest } from "@srsholmes/tauri-playwright";

export const { test, expect } = createTauriTest({
  devUrl: "",
  startTimeout: 180,
  tauriCommand: "corepack pnpm tauri:dev",
  tauriCwd: ".",
  tauriFeatures: ["e2e-testing"],
});
