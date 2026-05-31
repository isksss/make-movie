import {
  definePlugin,
  metadataToJson,
  noopPlugin,
  type MmPlugin,
  type PluginMetadata,
} from "./index.js";

const metadata = {
  name: "ts-plugin",
  version: "0.1.0",
  category: "utility",
  displayName: "TS Plugin",
} satisfies PluginMetadata;

const metadataJson: string = metadataToJson(metadata);

const plugin: MmPlugin = definePlugin({
  metadata: () => metadataJson,
  initialize: async () => undefined,
  shutdown: () => undefined,
});

const noopMetadata: string = noopPlugin.metadata();

void plugin;
void noopMetadata;
