import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { AssetKind, GpuProbeResult, RenderBackend } from "../types";

export const commands = {
  loadProject: (path: string) => invoke<string>("load_project", { path }),
  saveProject: (path: string, toml: string) => invoke<void>("save_project", { path, toml }),
  buildProject: (path: string) => invoke<void>("build_project", { path }),
  buildProjectWithBackend: (path: string, backend: RenderBackend) =>
    invoke<void>("build_project_with_backend", { path, backend }),
  renderPreviewFrame: (path: string, time: number, backend: RenderBackend) =>
    invoke<string>("render_preview_frame", { path, time, backend }),
  probeGpuBackend: () => invoke<GpuProbeResult>("probe_gpu_backend"),
  listSystemFonts: () => invoke<string[]>("list_system_fonts"),
  importAsset: (projectRoot: string, sourcePath: string, kind: AssetKind) =>
    invoke<string>("import_asset", { projectRoot, sourcePath, kind }),
  importAssetIntoProject: (projectPath: string, sourcePath: string, kind: AssetKind) =>
    invoke<string>("import_asset_into_project", { projectPath, sourcePath, kind }),
  applyExternalAnalysisResult: (projectToml: string, layerId: string, analysisJson: string) =>
    invoke<string>("apply_external_analysis_result", { projectToml, layerId, analysisJson }),
  installPlugin: (name: string) => invoke<void>("install_plugin", { name }),
  installConfiguredPlugins: (projectPath: string, globalConfig?: string) =>
    invoke<string[]>(
      "install_configured_plugins",
      globalConfig ? { projectPath, globalConfig } : { projectPath },
    ),
  updatePlugin: (name: string) => invoke<void>("update_plugin", { name }),
  removePlugin: (name: string) => invoke<void>("remove_plugin", { name }),
};

export const dialogs = {
  openProject: async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "make-movie project", extensions: ["toml"] }],
    });
    return typeof selected === "string" ? selected : null;
  },
  saveProject: async () => {
    const selected = await save({
      filters: [{ name: "make-movie project", extensions: ["toml"] }],
      defaultPath: "mm.toml",
    });
    return typeof selected === "string" ? selected : null;
  },
  openAsset: async () => {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "media",
          extensions: [
            "mp4",
            "mov",
            "webm",
            "png",
            "jpg",
            "jpeg",
            "webp",
            "wav",
            "mp3",
            "ogg",
            "srt",
            "ass",
            "vtt",
            "ttf",
            "otf",
            "svg",
          ],
        },
      ],
    });
    return typeof selected === "string" ? selected : null;
  },
};
