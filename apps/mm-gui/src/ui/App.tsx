import {
  Box,
  Copy,
  FolderOpen,
  Import,
  Pause,
  Play,
  Save,
  Scissors,
  Settings,
  SkipBack,
  SkipForward,
  Trash2,
  Redo2,
  Undo2,
  Wand2,
} from "lucide-react";
import { useState } from "react";
import { commands } from "../services/tauri";
import { parseProjectToml, serializeProjectToToml } from "../store/projectToml";
import { useProjectStore } from "../store/projectStore";
import { usePreviewStore } from "../store/previewStore";
import { cropRect, layerAnimation, layerEffect, layerTransform, layerTransition } from "../types";
import type { ProjectState } from "../types";
import { PreviewCanvas } from "./PreviewCanvas";

const defaultProjectPath = "mm.toml";
const defaultProjectRoot = ".";
const defaultImportPath = "media/image/import.png";

export function App() {
  const {
    project,
    selectedAssetId,
    selectedLayerId,
    tts,
    setProject,
    selectAsset,
    selectLayer,
    moveLayer,
    splitLayer,
    duplicateLayer,
    deleteLayer,
    updateLayerTransform,
    updateLayerVisual,
    updateLayerTransition,
    updateLayerEffect,
    updateLayerAnimation,
    updateTtsText,
    undo,
    redo,
    canUndo,
    canRedo,
  } = useProjectStore();
  const preview = usePreviewStore();
  const [commandStatus, setCommandStatus] = useState("Ready");
  const selectedLayer =
    project.layers.find((layer) => layer.id === selectedLayerId) ?? project.layers[0];
  const selectedTransform = selectedLayer ? layerTransform(selectedLayer.transform) : null;
  const selectedCrop = selectedLayer ? cropRect(selectedLayer.crop) : null;
  const selectedTransition = selectedLayer ? layerTransition(selectedLayer.transition) : null;
  const selectedEffect = selectedLayer ? layerEffect(selectedLayer.effects[0]) : null;
  const selectedAnimation = selectedLayer ? layerAnimation(selectedLayer.animations[0]) : null;
  const runCommand = async (action: () => Promise<unknown>, successMessage: string) => {
    try {
      await action();
      setCommandStatus(successMessage);
    } catch (error) {
      setCommandStatus(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div className="app-shell">
      <header className="menu-bar">
        <div className="brand">make-movie</div>
        <div className="toolbar" aria-label="Project toolbar">
          <button
            onClick={() =>
              runCommand(async () => {
                const toml = await commands.loadProject(defaultProjectPath);
                setProject(parseProjectToml(toml, project));
              }, "プロジェクトを開きました")
            }
            title="Open project"
          >
            <FolderOpen size={18} />
          </button>
          <button
            onClick={() =>
              runCommand(
                () => commands.saveProject(defaultProjectPath, serializeProjectToToml(project)),
                "プロジェクトを保存しました",
              )
            }
            title="Save project"
          >
            <Save size={18} />
          </button>
          <button
            onClick={() =>
              runCommand(
                () => commands.importAsset(defaultProjectRoot, defaultImportPath, "image"),
                "アセットを取り込みました",
              )
            }
            title="Import asset"
          >
            <Import size={18} />
          </button>
          <button
            onClick={() =>
              runCommand(() => commands.buildProject(defaultProjectPath), "動画を書き出しました")
            }
            title="Build movie"
          >
            <Wand2 size={18} />
          </button>
          <button disabled={!canUndo} onClick={undo} title="Undo">
            <Undo2 size={18} />
          </button>
          <button disabled={!canRedo} onClick={redo} title="Redo">
            <Redo2 size={18} />
          </button>
        </div>
        <div aria-live="polite" className="command-status">
          {commandStatus}
        </div>
      </header>

      <main className="workspace">
        <section className="assets-pane" aria-label="Assets">
          <div className="pane-heading">
            <Box size={16} />
            <span>Assets</span>
          </div>
          <div className="asset-list">
            {project.assets.map((asset) => (
              <button
                className={asset.id === selectedAssetId ? "asset-row selected" : "asset-row"}
                key={asset.id}
                onClick={() => selectAsset(asset.id)}
              >
                <span>{asset.id}</span>
                <small>{asset.kind}</small>
              </button>
            ))}
          </div>
        </section>

        <section className="preview-pane" aria-label="Preview">
          <div className="preview-surface">
            <PreviewCanvas project={project} preview={preview} />
          </div>
          <div className="preview-controls">
            <button
              title="Previous frame"
              onClick={() => preview.stepFrame(project.settings.fps, -1)}
            >
              <SkipBack size={18} />
            </button>
            <button
              title={preview.playing ? "Stop" : "Play"}
              onClick={preview.playing ? preview.stop : preview.play}
            >
              {preview.playing ? <Pause size={18} /> : <Play size={18} />}
            </button>
            <button title="Next frame" onClick={() => preview.stepFrame(project.settings.fps, 1)}>
              <SkipForward size={18} />
            </button>
            <input
              aria-label="Seek"
              max={project.settings.duration}
              min={0}
              onChange={(event) => preview.seek(Number(event.target.value))}
              step={1 / project.settings.fps}
              type="range"
              value={preview.currentTime}
            />
            <select
              aria-label="Playback rate"
              onChange={(event) => preview.setRate(Number(event.target.value))}
              value={preview.playbackRate}
            >
              <option value={0.5}>0.5x</option>
              <option value={1}>1x</option>
              <option value={1.5}>1.5x</option>
              <option value={2}>2x</option>
            </select>
          </div>
        </section>

        <section className="property-pane" aria-label="Property">
          <div className="pane-heading">
            <Settings size={16} />
            <span>Property</span>
          </div>
          {selectedLayer ? (
            <div className="property-grid">
              <label>
                Layer
                <input readOnly value={selectedLayer.label} />
              </label>
              <label>
                Start
                <input
                  min={0}
                  onChange={(event) => moveLayer(selectedLayer.id, Number(event.target.value))}
                  step={0.1}
                  type="number"
                  value={selectedLayer.start}
                />
              </label>
              <label>
                Duration
                <input readOnly value={selectedLayer.duration} />
              </label>
              <label>
                Z
                <input readOnly value={selectedLayer.zIndex} />
              </label>
              {selectedTransform ? (
                <>
                  <label>
                    X
                    <input
                      onChange={(event) =>
                        updateLayerTransform(selectedLayer.id, { x: Number(event.target.value) })
                      }
                      step={1}
                      type="number"
                      value={selectedTransform.x}
                    />
                  </label>
                  <label>
                    Y
                    <input
                      onChange={(event) =>
                        updateLayerTransform(selectedLayer.id, { y: Number(event.target.value) })
                      }
                      step={1}
                      type="number"
                      value={selectedTransform.y}
                    />
                  </label>
                  <label>
                    Width
                    <input
                      min={0}
                      onChange={(event) =>
                        updateLayerTransform(selectedLayer.id, {
                          width: Number(event.target.value),
                        })
                      }
                      step={1}
                      type="number"
                      value={selectedTransform.width}
                    />
                  </label>
                  <label>
                    Height
                    <input
                      min={0}
                      onChange={(event) =>
                        updateLayerTransform(selectedLayer.id, {
                          height: Number(event.target.value),
                        })
                      }
                      step={1}
                      type="number"
                      value={selectedTransform.height}
                    />
                  </label>
                  <label>
                    Scale
                    <input
                      min={0.01}
                      onChange={(event) =>
                        updateLayerTransform(selectedLayer.id, {
                          scale: Number(event.target.value),
                        })
                      }
                      step={0.01}
                      type="number"
                      value={selectedTransform.scale}
                    />
                  </label>
                  <label>
                    Rotation
                    <input
                      onChange={(event) =>
                        updateLayerTransform(selectedLayer.id, {
                          rotation: Number(event.target.value),
                        })
                      }
                      step={1}
                      type="number"
                      value={selectedTransform.rotation}
                    />
                  </label>
                  <label>
                    Opacity
                    <input
                      max={1}
                      min={0}
                      onChange={(event) =>
                        updateLayerTransform(selectedLayer.id, {
                          opacity: Number(event.target.value),
                        })
                      }
                      step={0.01}
                      type="number"
                      value={selectedTransform.opacity}
                    />
                  </label>
                  {selectedCrop ? (
                    <>
                      <label>
                        Crop X
                        <input
                          min={0}
                          onChange={(event) =>
                            updateLayerVisual(selectedLayer.id, {
                              crop: { x: Number(event.target.value) },
                            })
                          }
                          step={1}
                          type="number"
                          value={selectedCrop.x}
                        />
                      </label>
                      <label>
                        Crop Y
                        <input
                          min={0}
                          onChange={(event) =>
                            updateLayerVisual(selectedLayer.id, {
                              crop: { y: Number(event.target.value) },
                            })
                          }
                          step={1}
                          type="number"
                          value={selectedCrop.y}
                        />
                      </label>
                      <label>
                        Crop Width
                        <input
                          min={0}
                          onChange={(event) =>
                            updateLayerVisual(selectedLayer.id, {
                              crop: { width: Number(event.target.value) },
                            })
                          }
                          step={1}
                          type="number"
                          value={selectedCrop.width}
                        />
                      </label>
                      <label>
                        Crop Height
                        <input
                          min={0}
                          onChange={(event) =>
                            updateLayerVisual(selectedLayer.id, {
                              crop: { height: Number(event.target.value) },
                            })
                          }
                          step={1}
                          type="number"
                          value={selectedCrop.height}
                        />
                      </label>
                    </>
                  ) : null}
                  <label>
                    Mask
                    <select
                      onChange={(event) =>
                        updateLayerVisual(selectedLayer.id, {
                          mask: event.target.value as ProjectState["layers"][number]["mask"],
                        })
                      }
                      value={selectedLayer.mask}
                    >
                      <option value="none">none</option>
                      <option value="circle">circle</option>
                      <option value="rounded_rect">rounded_rect</option>
                      <option value="ellipse">ellipse</option>
                    </select>
                  </label>
                  <label>
                    Fit
                    <select
                      onChange={(event) =>
                        updateLayerVisual(selectedLayer.id, {
                          fit: event.target.value as ProjectState["layers"][number]["fit"],
                        })
                      }
                      value={selectedLayer.fit}
                    >
                      <option value="none">none</option>
                      <option value="contain">contain</option>
                      <option value="cover">cover</option>
                      <option value="stretch">stretch</option>
                      <option value="blur_background">blur_background</option>
                    </select>
                  </label>
                  {selectedTransition ? (
                    <>
                      <label>
                        Transition
                        <select
                          onChange={(event) =>
                            updateLayerTransition(selectedLayer.id, {
                              kind: event.target
                                .value as ProjectState["layers"][number]["transition"]["kind"],
                            })
                          }
                          value={selectedTransition.kind}
                        >
                          <option value="none">none</option>
                          <option value="crossfade">crossfade</option>
                          <option value="wipe">wipe</option>
                          <option value="push">push</option>
                          <option value="zoom">zoom</option>
                          <option value="blur">blur</option>
                          <option value="flash">flash</option>
                        </select>
                      </label>
                      <label>
                        Transition Duration
                        <input
                          min={0}
                          onChange={(event) =>
                            updateLayerTransition(selectedLayer.id, {
                              duration: Number(event.target.value),
                            })
                          }
                          step={0.1}
                          type="number"
                          value={selectedTransition.duration}
                        />
                      </label>
                    </>
                  ) : null}
                  {selectedEffect ? (
                    <>
                      <label>
                        Effect
                        <select
                          onChange={(event) =>
                            updateLayerEffect(selectedLayer.id, {
                              kind: event.target
                                .value as ProjectState["layers"][number]["effects"][number]["kind"],
                            })
                          }
                          value={selectedEffect.kind}
                        >
                          <option value="none">none</option>
                          <option value="fade_in">fade_in</option>
                          <option value="fade_out">fade_out</option>
                          <option value="blur">blur</option>
                          <option value="zoom">zoom</option>
                          <option value="slide">slide</option>
                          <option value="brightness">brightness</option>
                          <option value="contrast">contrast</option>
                          <option value="saturation">saturation</option>
                          <option value="pixelate">pixelate</option>
                          <option value="motion_blur">motion_blur</option>
                        </select>
                      </label>
                      <label>
                        Effect Duration
                        <input
                          min={0}
                          onChange={(event) =>
                            updateLayerEffect(selectedLayer.id, {
                              duration: Number(event.target.value),
                            })
                          }
                          step={0.1}
                          type="number"
                          value={selectedEffect.duration}
                        />
                      </label>
                      <label>
                        Effect Amount
                        <input
                          onChange={(event) =>
                            updateLayerEffect(selectedLayer.id, {
                              amount: Number(event.target.value),
                            })
                          }
                          step={0.1}
                          type="number"
                          value={selectedEffect.amount}
                        />
                      </label>
                      <label>
                        Effect X
                        <input
                          onChange={(event) =>
                            updateLayerEffect(selectedLayer.id, {
                              x: Number(event.target.value),
                            })
                          }
                          step={1}
                          type="number"
                          value={selectedEffect.x}
                        />
                      </label>
                      <label>
                        Effect Y
                        <input
                          onChange={(event) =>
                            updateLayerEffect(selectedLayer.id, {
                              y: Number(event.target.value),
                            })
                          }
                          step={1}
                          type="number"
                          value={selectedEffect.y}
                        />
                      </label>
                    </>
                  ) : null}
                  {selectedAnimation ? (
                    <>
                      <label>
                        Keyframe Property
                        <select
                          onChange={(event) =>
                            updateLayerAnimation(selectedLayer.id, {
                              property: event.target
                                .value as ProjectState["layers"][number]["animations"][number]["property"],
                            })
                          }
                          value={selectedAnimation.property}
                        >
                          <option value="none">none</option>
                          <option value="x">x</option>
                          <option value="y">y</option>
                          <option value="scale">scale</option>
                          <option value="rotation">rotation</option>
                          <option value="opacity">opacity</option>
                          <option value="width">width</option>
                          <option value="height">height</option>
                        </select>
                      </label>
                      <label>
                        Easing
                        <select
                          onChange={(event) =>
                            updateLayerAnimation(selectedLayer.id, {
                              easing: event.target
                                .value as ProjectState["layers"][number]["animations"][number]["easing"],
                            })
                          }
                          value={selectedAnimation.easing}
                        >
                          <option value="linear">linear</option>
                          <option value="ease_in">ease_in</option>
                          <option value="ease_out">ease_out</option>
                          <option value="ease_in_out">ease_in_out</option>
                          <option value="ease_out_back">ease_out_back</option>
                          <option value="bounce">bounce</option>
                          <option value="elastic">elastic</option>
                        </select>
                      </label>
                      <label>
                        Keyframe 1 Time
                        <input
                          min={0}
                          onChange={(event) =>
                            updateLayerAnimation(selectedLayer.id, {
                              keyframes: [
                                {
                                  ...selectedAnimation.keyframes[0],
                                  time: Number(event.target.value),
                                },
                                selectedAnimation.keyframes[1],
                              ],
                            })
                          }
                          step={0.1}
                          type="number"
                          value={selectedAnimation.keyframes[0].time}
                        />
                      </label>
                      <label>
                        Keyframe 1 Value
                        <input
                          onChange={(event) =>
                            updateLayerAnimation(selectedLayer.id, {
                              keyframes: [
                                {
                                  ...selectedAnimation.keyframes[0],
                                  value: Number(event.target.value),
                                },
                                selectedAnimation.keyframes[1],
                              ],
                            })
                          }
                          step={0.1}
                          type="number"
                          value={selectedAnimation.keyframes[0].value}
                        />
                      </label>
                      <label>
                        Keyframe 2 Time
                        <input
                          min={0}
                          onChange={(event) =>
                            updateLayerAnimation(selectedLayer.id, {
                              keyframes: [
                                selectedAnimation.keyframes[0],
                                {
                                  ...selectedAnimation.keyframes[1],
                                  time: Number(event.target.value),
                                },
                              ],
                            })
                          }
                          step={0.1}
                          type="number"
                          value={selectedAnimation.keyframes[1].time}
                        />
                      </label>
                      <label>
                        Keyframe 2 Value
                        <input
                          onChange={(event) =>
                            updateLayerAnimation(selectedLayer.id, {
                              keyframes: [
                                selectedAnimation.keyframes[0],
                                {
                                  ...selectedAnimation.keyframes[1],
                                  value: Number(event.target.value),
                                },
                              ],
                            })
                          }
                          step={0.1}
                          type="number"
                          value={selectedAnimation.keyframes[1].value}
                        />
                      </label>
                    </>
                  ) : null}
                </>
              ) : null}
            </div>
          ) : null}
          <div className="tts-editor">
            <label>
              Speaker
              <input readOnly value={tts.speaker} />
            </label>
            <label>
              Text
              <textarea onChange={(event) => updateTtsText(event.target.value)} value={tts.text} />
            </label>
          </div>
        </section>

        <section className="timeline-pane" aria-label="Timeline">
          <div className="timeline-toolbar">
            <button
              disabled={!selectedLayer}
              onClick={() => selectedLayer && splitLayer(selectedLayer.id, preview.currentTime)}
              title="Cut"
            >
              <Scissors size={18} />
            </button>
            <button
              disabled={!selectedLayer}
              onClick={() => selectedLayer && duplicateLayer(selectedLayer.id)}
              title="Duplicate"
            >
              <Copy size={18} />
            </button>
            <button
              disabled={!selectedLayer}
              onClick={() => selectedLayer && deleteLayer(selectedLayer.id)}
              title="Delete"
            >
              <Trash2 size={18} />
            </button>
            <span>{preview.currentTime.toFixed(2)}s</span>
          </div>
          <div className="timeline-grid">
            {project.tracks.map((track) => (
              <div className="track-row" key={track.id}>
                <div className="track-label">{track.name}</div>
                <div className="track-lane">
                  {project.layers
                    .filter((layer) => layer.trackId === track.id)
                    .map((layer) => (
                      <button
                        className={layer.id === selectedLayerId ? "clip selected" : "clip"}
                        key={layer.id}
                        onClick={() => selectLayer(layer.id)}
                        style={{
                          left: `${(layer.start / project.settings.duration) * 100}%`,
                          width: `${(layer.duration / project.settings.duration) * 100}%`,
                        }}
                      >
                        {layer.label}
                      </button>
                    ))}
                </div>
              </div>
            ))}
          </div>
        </section>

        <section className="plugin-pane" aria-label="Plugin Manager">
          <div className="pane-heading">
            <Wand2 size={16} />
            <span>Plugins</span>
          </div>
          <button>VOICEVOX</button>
          <button>AivisSpeech</button>
          <button>Template Pack</button>
        </section>
      </main>
    </div>
  );
}
