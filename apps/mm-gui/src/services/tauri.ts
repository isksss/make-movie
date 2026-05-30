import { invoke } from "@tauri-apps/api/core";
import type { AssetKind } from "../types";

export const commands = {
  loadProject: (path: string) => invoke<string>("load_project", { path }),
  saveProject: (path: string, toml: string) => invoke<void>("save_project", { path, toml }),
  buildProject: (path: string) => invoke<void>("build_project", { path }),
  importAsset: (projectRoot: string, sourcePath: string, kind: AssetKind) =>
    invoke<string>("import_asset", { projectRoot, sourcePath, kind }),
  importAssetIntoProject: (projectPath: string, sourcePath: string, kind: AssetKind) =>
    invoke<string>("import_asset_into_project", { projectPath, sourcePath, kind }),
  installPlugin: (name: string) => invoke<void>("install_plugin", { name }),
  installConfiguredPlugins: (projectPath: string, globalConfig?: string) =>
    invoke<string[]>(
      "install_configured_plugins",
      globalConfig ? { projectPath, globalConfig } : { projectPath },
    ),
  updatePlugin: (name: string) => invoke<void>("update_plugin", { name }),
  removePlugin: (name: string) => invoke<void>("remove_plugin", { name }),
};
