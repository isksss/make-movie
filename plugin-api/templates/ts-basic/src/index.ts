import { definePlugin } from "mm-sdk-ts";

export const plugin = definePlugin({
  metadata: () => ({
    name: "ts-basic",
    version: "0.1.0",
    category: "utility",
    displayName: "TypeScript Basic",
    description: "TypeScript plugin template",
  }),
  initialize: () => undefined,
  shutdown: () => undefined,
});

export default plugin;
