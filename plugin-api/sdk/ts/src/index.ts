// Generated from plugin-api/plugin.wit.

export type PluginCategory =
  | "ai"
  | "subtitle"
  | "tts"
  | "template"
  | "export"
  | "utility";

export interface PluginMetadata {
  name: string;
  version: string;
  category: PluginCategory;
  displayName?: string;
  description?: string;
}

export interface MmPlugin {
  metadata(): PluginMetadata;
  initialize(): void | Promise<void>;
  shutdown(): void | Promise<void>;
}

const categories = new Set<PluginCategory>([
  "ai",
  "subtitle",
  "tts",
  "template",
  "export",
  "utility",
]);

export function definePlugin(plugin: MmPlugin): MmPlugin {
  return plugin;
}

export function metadataToJson(metadata: PluginMetadata): string {
  validateMetadata(metadata);
  return JSON.stringify({
    name: metadata.name,
    version: metadata.version,
    category: metadata.category,
    ...(metadata.displayName ? { display_name: metadata.displayName } : {}),
    ...(metadata.description ? { description: metadata.description } : {}),
  });
}

export function validateMetadata(metadata: PluginMetadata): void {
  if (metadata.name.trim() === "") {
    throw new Error("metadata name is required");
  }
  if (metadata.version.trim() === "") {
    throw new Error("metadata version is required");
  }
  if (!categories.has(metadata.category)) {
    throw new Error(`metadata category is invalid: ${metadata.category}`);
  }
}

export const noopPlugin: MmPlugin = definePlugin({
  metadata: () => ({
    name: "noop-plugin",
    version: "0.1.0",
    category: "utility",
  }),
  initialize: () => undefined,
  shutdown: () => undefined,
});
