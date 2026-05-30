export interface MmPlugin {
  metadata(): string;
  initialize(): void | Promise<void>;
  shutdown(): void | Promise<void>;
}

export function definePlugin(plugin: MmPlugin): MmPlugin {
  return plugin;
}

export const noopPlugin: MmPlugin = definePlugin({
  metadata: () => "{}",
  initialize: () => undefined,
  shutdown: () => undefined,
});
