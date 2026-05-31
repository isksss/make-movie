import { definePlugin, metadataToJson } from "../src/index.js";

export default definePlugin({
  metadata: () =>
    metadataToJson({
      name: "minimal-plugin",
      version: "0.1.0",
      category: "utility",
      displayName: "Minimal Plugin",
      description: "Minimal make-movie plugin example",
    }),
  initialize: () => undefined,
  shutdown: () => undefined,
});
