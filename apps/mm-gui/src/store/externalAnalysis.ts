import { commands } from "../services/tauri";
import type { ProjectState } from "../types";
import { parseProjectToml, serializeProjectToToml } from "./projectToml";

type ApplyExternalAnalysisCommand = (
  projectToml: string,
  layerId: string,
  analysisJson: string,
) => Promise<string>;

export const sampleExternalAnalysisJson = JSON.stringify(
  {
    source_width: 1920,
    source_height: 1080,
    targets: [
      {
        id: "face-1",
        kind: "face",
        frames: [
          {
            time: 0,
            bbox: { x: 640, y: 240, width: 320, height: 320 },
            confidence: 0.98,
          },
          {
            time: 1,
            bbox: { x: 700, y: 260, width: 340, height: 340 },
            confidence: 0.96,
          },
        ],
      },
    ],
  },
  null,
  2,
);

export async function applyExternalAnalysisJson(
  project: ProjectState,
  layerId: string,
  analysisJson: string,
  applyCommand: ApplyExternalAnalysisCommand = commands.applyExternalAnalysisResult,
): Promise<ProjectState> {
  const projectToml = serializeProjectToToml(project);
  const updatedToml = await applyCommand(projectToml, layerId, analysisJson);
  return parseProjectToml(updatedToml, project);
}
