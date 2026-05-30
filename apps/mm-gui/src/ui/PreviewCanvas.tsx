import { useEffect, useRef } from "react";
import type { PreviewState, ProjectState } from "../types";
import { drawPreviewFrame } from "./previewRenderer";

interface PreviewCanvasProps {
  ariaLabel: string;
  project: ProjectState;
  preview: PreviewState;
}

export function PreviewCanvas({ ariaLabel, project, preview }: PreviewCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }
    const parent = canvas.parentElement;
    const width = Math.max(320, parent?.clientWidth ?? 640);
    const height = Math.max(240, parent?.clientHeight ?? 360);
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext("2d");
    if (!context) {
      return;
    }
    drawPreviewFrame(context, project, preview);
  }, [project, preview]);

  return <canvas aria-label={ariaLabel} className="preview-canvas" ref={canvasRef} />;
}
