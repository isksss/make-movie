import {
  Box,
  Copy,
  Download,
  FolderOpen,
  Import,
  Pause,
  Play,
  Repeat,
  Save,
  Scissors,
  Settings,
  SkipBack,
  SkipForward,
  Trash2,
  RefreshCw,
  Redo2,
  Undo2,
  Wand2,
  X,
} from "lucide-react";
import { useState } from "react";
import { commands } from "../services/tauri";
import { parseProjectToml, serializeProjectToToml } from "../store/projectToml";
import { useProjectStore } from "../store/projectStore";
import { usePreviewStore } from "../store/previewStore";
import { cropRect, layerAnimation, layerEffect, layerTransform, layerTransition } from "../types";
import type { AssetKind, PluginDeclaration, ProjectState, TtsProviderKind } from "../types";
import { messages, optionLabels } from "./i18n";
import type { Locale } from "./i18n";
import { PreviewCanvas } from "./PreviewCanvas";

const defaultProjectPath = "mm.toml";
const defaultImportPath = "media/image/import.png";
const fallbackPlugins = ["VOICEVOX", "AivisSpeech", "Template Pack"] as const;
const textAlignOptions = ["left", "center", "right"] as const;
const gradientDirectionOptions = ["vertical", "horizontal"] as const;
const maskOptions = ["none", "circle", "rounded_rect", "ellipse", "svg"] as const;
const fitOptions = ["none", "contain", "cover", "stretch", "blur_background"] as const;
const transitionOptions = ["none", "crossfade", "wipe", "push", "zoom", "blur", "flash"] as const;
const wipeShapeOptions = ["circle", "rounded_rect"] as const;
const effectOptions = [
  "none",
  "fade_in",
  "fade_out",
  "blur",
  "zoom",
  "slide",
  "brightness",
  "contrast",
  "saturation",
  "pixelate",
  "motion_blur",
] as const;
const animatedPropertyOptions = [
  "none",
  "x",
  "y",
  "scale",
  "rotation",
  "opacity",
  "width",
  "height",
] as const;
const easingOptions = [
  "linear",
  "ease_in",
  "ease_out",
  "ease_in_out",
  "ease_out_back",
  "bounce",
  "elastic",
] as const;
const ttsProviderOptions = ["voicevox", "aivis_speech", "coeiro_ink"] as const;

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
    updateLayerTrim,
    updateLayerTransform,
    updateLayerVisual,
    updateLayerText,
    updateLayerVoice,
    updateLayerTransition,
    updateLayerEffect,
    updateLayerAnimation,
    undo,
    redo,
    canUndo,
    canRedo,
  } = useProjectStore();
  const preview = usePreviewStore();
  const [locale, setLocale] = useState<Locale>("ja");
  const t = messages[locale];
  const labels = optionLabels[locale];
  const [commandStatus, setCommandStatus] = useState<string>(t.ready);
  const selectedLayer =
    project.layers.find((layer) => layer.id === selectedLayerId) ?? project.layers[0];
  const selectedTransform = selectedLayer ? layerTransform(selectedLayer.transform) : null;
  const selectedCrop = selectedLayer ? cropRect(selectedLayer.crop) : null;
  const selectedText = selectedLayer?.contentKind === "text" ? selectedLayer.text : null;
  const voiceLayer =
    (selectedLayer?.contentKind === "voice" ? selectedLayer : null) ??
    project.layers.find((layer) => layer.contentKind === "voice") ??
    null;
  const selectedTransition = selectedLayer ? layerTransition(selectedLayer.transition) : null;
  const selectedEffect = selectedLayer ? layerEffect(selectedLayer.effects[0]) : null;
  const selectedAnimation = selectedLayer ? layerAnimation(selectedLayer.animations[0]) : null;
  const pluginNames =
    project.plugins.length > 0 ? project.plugins.map(pluginDisplayName) : [...fallbackPlugins];
  const runCommand = async (action: () => Promise<unknown>, successMessage: string) => {
    try {
      await action();
      setCommandStatus(successMessage);
    } catch (error) {
      setCommandStatus(error instanceof Error ? error.message : String(error));
    }
  };
  const importAssetFromPath = (sourcePath: string) =>
    runCommand(async () => {
      const toml = await commands.importAssetIntoProject(
        defaultProjectPath,
        sourcePath,
        inferAssetKind(sourcePath),
      );
      setProject(parseProjectToml(toml, project));
    }, t.imported);

  return (
    <div className="app-shell">
      <header className="menu-bar">
        <div className="brand">make-movie</div>
        <div className="toolbar" aria-label={t.projectToolbar}>
          <button
            onClick={() =>
              runCommand(async () => {
                const toml = await commands.loadProject(defaultProjectPath);
                setProject(parseProjectToml(toml, project));
              }, t.opened)
            }
            title={t.openProject}
          >
            <FolderOpen size={18} />
          </button>
          <button
            onClick={() =>
              runCommand(
                () => commands.saveProject(defaultProjectPath, serializeProjectToToml(project)),
                t.saved,
              )
            }
            title={t.saveProject}
          >
            <Save size={18} />
          </button>
          <button onClick={() => importAssetFromPath(defaultImportPath)} title={t.importAsset}>
            <Import size={18} />
          </button>
          <button
            onClick={() => runCommand(() => commands.buildProject(defaultProjectPath), t.built)}
            title={t.buildMovie}
          >
            <Wand2 size={18} />
          </button>
          <button disabled={!canUndo} onClick={undo} title={t.undo}>
            <Undo2 size={18} />
          </button>
          <button disabled={!canRedo} onClick={redo} title={t.redo}>
            <Redo2 size={18} />
          </button>
        </div>
        <div aria-live="polite" className="command-status">
          {commandStatus}
        </div>
        <label className="language-select">
          {t.language}
          <select
            aria-label={t.language}
            onChange={(event) => {
              const nextLocale = event.target.value as Locale;
              setLocale(nextLocale);
              setCommandStatus(messages[nextLocale].ready);
            }}
            value={locale}
          >
            <option value="ja">{t.japanese}</option>
            <option value="en">{t.english}</option>
          </select>
        </label>
      </header>

      <main className="workspace">
        <section
          className="assets-pane"
          aria-label={t.assets}
          onDragOver={(event) => event.preventDefault()}
          onDrop={(event) => {
            event.preventDefault();
            const sourcePath = droppedSourcePath(event.dataTransfer);
            if (sourcePath) {
              void importAssetFromPath(sourcePath);
            }
          }}
        >
          <div className="pane-heading">
            <Box size={16} />
            <span>{t.assets}</span>
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

        <section className="preview-pane" aria-label={t.preview}>
          <div className="preview-surface">
            <PreviewCanvas ariaLabel={t.renderedPreview} project={project} preview={preview} />
          </div>
          <div className="preview-controls">
            <button
              title={t.previousFrame}
              onClick={() => preview.stepFrame(project.settings.fps, -1)}
            >
              <SkipBack size={18} />
            </button>
            <button
              title={preview.playing ? t.stop : t.play}
              onClick={preview.playing ? preview.stop : preview.play}
            >
              {preview.playing ? <Pause size={18} /> : <Play size={18} />}
            </button>
            <button title={t.nextFrame} onClick={() => preview.stepFrame(project.settings.fps, 1)}>
              <SkipForward size={18} />
            </button>
            <button
              aria-pressed={preview.loop}
              className={preview.loop ? "selected" : undefined}
              onClick={preview.toggleLoop}
              title={t.loop}
            >
              <Repeat size={18} />
            </button>
            <input
              aria-label={t.seek}
              max={project.settings.duration}
              min={0}
              onChange={(event) => preview.seek(Number(event.target.value))}
              step={1 / project.settings.fps}
              type="range"
              value={preview.currentTime}
            />
            <select
              aria-label={t.playbackRate}
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

        <section className="property-pane" aria-label={t.property}>
          <div className="pane-heading">
            <Settings size={16} />
            <span>{t.property}</span>
          </div>
          {selectedLayer ? (
            <div className="property-grid">
              <label>
                {t.layer}
                <input readOnly value={selectedLayer.label} />
              </label>
              <label>
                {t.start}
                <input
                  min={0}
                  onChange={(event) => moveLayer(selectedLayer.id, Number(event.target.value))}
                  step={0.1}
                  type="number"
                  value={selectedLayer.start}
                />
              </label>
              <label>
                {t.duration}
                <input readOnly value={selectedLayer.duration} />
              </label>
              <label>
                Z
                <input readOnly value={selectedLayer.zIndex} />
              </label>
              {selectedLayer.contentKind === "video" || selectedLayer.contentKind === "audio" ? (
                <>
                  <label>
                    {t.trimStart}
                    <input
                      min={0}
                      onChange={(event) =>
                        updateLayerTrim(selectedLayer.id, {
                          trimStart: Number(event.target.value),
                        })
                      }
                      step={0.1}
                      type="number"
                      value={selectedLayer.trimStart}
                    />
                  </label>
                  <label>
                    {t.trimEnd}
                    <input
                      min={0}
                      onChange={(event) =>
                        updateLayerTrim(selectedLayer.id, {
                          trimEnd: Number(event.target.value),
                        })
                      }
                      step={0.1}
                      type="number"
                      value={selectedLayer.trimEnd}
                    />
                  </label>
                </>
              ) : null}
              {selectedText ? (
                <>
                  <label>
                    {t.text}
                    <textarea
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, { text: event.target.value })
                      }
                      value={selectedText.text}
                    />
                  </label>
                  <label>
                    {t.fontSize}
                    <input
                      min={1}
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, { fontSize: Number(event.target.value) })
                      }
                      step={1}
                      type="number"
                      value={selectedText.fontSize}
                    />
                  </label>
                  <label>
                    {t.textColor}
                    <input
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, { color: event.target.value })
                      }
                      type="text"
                      value={selectedText.color}
                    />
                  </label>
                  <label>
                    {t.textGradient}
                    <input
                      checked={selectedText.gradient.enabled}
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, {
                          gradient: {
                            ...selectedText.gradient,
                            enabled: event.target.checked,
                          },
                        })
                      }
                      type="checkbox"
                    />
                  </label>
                  {selectedText.gradient.enabled ? (
                    <>
                      <label>
                        {t.gradientStartColor}
                        <input
                          onChange={(event) =>
                            updateLayerText(selectedLayer.id, {
                              gradient: {
                                ...selectedText.gradient,
                                startColor: event.target.value,
                              },
                            })
                          }
                          value={selectedText.gradient.startColor}
                        />
                      </label>
                      <label>
                        {t.gradientEndColor}
                        <input
                          onChange={(event) =>
                            updateLayerText(selectedLayer.id, {
                              gradient: {
                                ...selectedText.gradient,
                                endColor: event.target.value,
                              },
                            })
                          }
                          value={selectedText.gradient.endColor}
                        />
                      </label>
                      <label>
                        {t.gradientDirection}
                        <select
                          onChange={(event) =>
                            updateLayerText(selectedLayer.id, {
                              gradient: {
                                ...selectedText.gradient,
                                direction: event.target
                                  .value as ProjectState["layers"][number]["text"]["gradient"]["direction"],
                              },
                            })
                          }
                          value={selectedText.gradient.direction}
                        >
                          {gradientDirectionOptions.map((value) => (
                            <option key={value} value={value}>
                              {labels.gradientDirection[value]}
                            </option>
                          ))}
                        </select>
                      </label>
                    </>
                  ) : null}
                  <label>
                    {t.letterSpacing}
                    <input
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, {
                          letterSpacing: Number(event.target.value),
                        })
                      }
                      step={0.1}
                      type="number"
                      value={selectedText.letterSpacing}
                    />
                  </label>
                  <label>
                    {t.lineSpacing}
                    <input
                      min={0}
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, {
                          lineSpacing: Number(event.target.value),
                        })
                      }
                      step={0.1}
                      type="number"
                      value={selectedText.lineSpacing}
                    />
                  </label>
                  <label>
                    {t.textAlign}
                    <select
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, {
                          align: event.target
                            .value as ProjectState["layers"][number]["text"]["align"],
                        })
                      }
                      value={selectedText.align}
                    >
                      {textAlignOptions.map((value) => (
                        <option key={value} value={value}>
                          {labels.textAlign[value]}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    {t.strokeColor}
                    <input
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, {
                          stroke: { ...selectedText.stroke, color: event.target.value },
                        })
                      }
                      value={selectedText.stroke.color}
                    />
                  </label>
                  <label>
                    {t.strokeWidth}
                    <input
                      min={0}
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, {
                          stroke: { ...selectedText.stroke, width: Number(event.target.value) },
                        })
                      }
                      step={0.1}
                      type="number"
                      value={selectedText.stroke.width}
                    />
                  </label>
                  <label>
                    {t.shadowColor}
                    <input
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, {
                          shadow: { ...selectedText.shadow, color: event.target.value },
                        })
                      }
                      value={selectedText.shadow.color}
                    />
                  </label>
                  <label>
                    {t.shadowOffsetX}
                    <input
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, {
                          shadow: { ...selectedText.shadow, offsetX: Number(event.target.value) },
                        })
                      }
                      step={1}
                      type="number"
                      value={selectedText.shadow.offsetX}
                    />
                  </label>
                  <label>
                    {t.shadowOffsetY}
                    <input
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, {
                          shadow: { ...selectedText.shadow, offsetY: Number(event.target.value) },
                        })
                      }
                      step={1}
                      type="number"
                      value={selectedText.shadow.offsetY}
                    />
                  </label>
                  <label>
                    {t.shadowBlur}
                    <input
                      min={0}
                      onChange={(event) =>
                        updateLayerText(selectedLayer.id, {
                          shadow: { ...selectedText.shadow, blur: Number(event.target.value) },
                        })
                      }
                      step={0.1}
                      type="number"
                      value={selectedText.shadow.blur}
                    />
                  </label>
                </>
              ) : null}
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
                    {t.width}
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
                    {t.height}
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
                    {t.scale}
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
                    {t.rotation}
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
                    {t.opacity}
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
                        {t.cropX}
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
                        {t.cropY}
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
                        {t.cropWidth}
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
                        {t.cropHeight}
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
                    {t.mask}
                    <select
                      onChange={(event) =>
                        updateLayerVisual(selectedLayer.id, {
                          mask: event.target.value as ProjectState["layers"][number]["mask"],
                        })
                      }
                      value={selectedLayer.mask}
                    >
                      {maskOptions.map((value) => (
                        <option key={value} value={value}>
                          {labels.mask[value]}
                        </option>
                      ))}
                    </select>
                  </label>
                  {selectedLayer.mask === "rounded_rect" ? (
                    <label>
                      {t.maskRadius}
                      <input
                        min={0}
                        onChange={(event) =>
                          updateLayerVisual(selectedLayer.id, {
                            maskRadius: Number(event.target.value),
                          })
                        }
                        step={1}
                        type="number"
                        value={selectedLayer.maskRadius ?? 16}
                      />
                    </label>
                  ) : null}
                  {selectedLayer.mask === "svg" ? (
                    <label>
                      {t.maskPath}
                      <input
                        onChange={(event) =>
                          updateLayerVisual(selectedLayer.id, {
                            maskPath: event.target.value,
                          })
                        }
                        value={selectedLayer.maskPath ?? ""}
                      />
                    </label>
                  ) : null}
                  <label>
                    {t.fit}
                    <select
                      onChange={(event) =>
                        updateLayerVisual(selectedLayer.id, {
                          fit: event.target.value as ProjectState["layers"][number]["fit"],
                        })
                      }
                      value={selectedLayer.fit}
                    >
                      {fitOptions.map((value) => (
                        <option key={value} value={value}>
                          {labels.fit[value]}
                        </option>
                      ))}
                    </select>
                  </label>
                  {selectedTransition ? (
                    <>
                      <label>
                        {t.transition}
                        <select
                          onChange={(event) =>
                            updateLayerTransition(selectedLayer.id, {
                              kind: event.target
                                .value as ProjectState["layers"][number]["transition"]["kind"],
                            })
                          }
                          value={selectedTransition.kind}
                        >
                          {transitionOptions.map((value) => (
                            <option key={value} value={value}>
                              {labels.transition[value]}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label>
                        {t.transitionDuration}
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
                      {selectedTransition.kind === "wipe" ? (
                        <>
                          <label>
                            {t.wipeShape}
                            <select
                              onChange={(event) =>
                                updateLayerTransition(selectedLayer.id, {
                                  wipeShape: event.target
                                    .value as ProjectState["layers"][number]["transition"]["wipeShape"],
                                })
                              }
                              value={selectedTransition.wipeShape}
                            >
                              {wipeShapeOptions.map((value) => (
                                <option key={value} value={value}>
                                  {labels.wipeShape[value]}
                                </option>
                              ))}
                            </select>
                          </label>
                          {selectedTransition.wipeShape === "rounded_rect" ? (
                            <>
                              <label>
                                {t.wipeRadius}
                                <input
                                  min={0}
                                  onChange={(event) =>
                                    updateLayerTransition(selectedLayer.id, {
                                      wipeRadius: Number(event.target.value),
                                    })
                                  }
                                  step={1}
                                  type="number"
                                  value={selectedTransition.wipeRadius}
                                />
                              </label>
                              <label>
                                {t.wipeBorderColor}
                                <input
                                  onChange={(event) =>
                                    updateLayerTransition(selectedLayer.id, {
                                      wipeBorderColor: event.target.value,
                                    })
                                  }
                                  type="color"
                                  value={selectedTransition.wipeBorderColor}
                                />
                              </label>
                              <label>
                                {t.wipeBorderWidth}
                                <input
                                  min={0}
                                  onChange={(event) =>
                                    updateLayerTransition(selectedLayer.id, {
                                      wipeBorderWidth: Number(event.target.value),
                                    })
                                  }
                                  step={1}
                                  type="number"
                                  value={selectedTransition.wipeBorderWidth}
                                />
                              </label>
                              <label>
                                {t.wipeShadowColor}
                                <input
                                  onChange={(event) =>
                                    updateLayerTransition(selectedLayer.id, {
                                      wipeShadowColor: event.target.value,
                                    })
                                  }
                                  type="color"
                                  value={selectedTransition.wipeShadowColor}
                                />
                              </label>
                              <label>
                                {t.wipeShadowOffsetX}
                                <input
                                  onChange={(event) =>
                                    updateLayerTransition(selectedLayer.id, {
                                      wipeShadowOffsetX: Number(event.target.value),
                                    })
                                  }
                                  step={1}
                                  type="number"
                                  value={selectedTransition.wipeShadowOffsetX}
                                />
                              </label>
                              <label>
                                {t.wipeShadowOffsetY}
                                <input
                                  onChange={(event) =>
                                    updateLayerTransition(selectedLayer.id, {
                                      wipeShadowOffsetY: Number(event.target.value),
                                    })
                                  }
                                  step={1}
                                  type="number"
                                  value={selectedTransition.wipeShadowOffsetY}
                                />
                              </label>
                              <label>
                                {t.wipeShadowBlur}
                                <input
                                  min={0}
                                  onChange={(event) =>
                                    updateLayerTransition(selectedLayer.id, {
                                      wipeShadowBlur: Number(event.target.value),
                                    })
                                  }
                                  step={1}
                                  type="number"
                                  value={selectedTransition.wipeShadowBlur}
                                />
                              </label>
                            </>
                          ) : null}
                        </>
                      ) : null}
                    </>
                  ) : null}
                  {selectedEffect ? (
                    <>
                      <label>
                        {t.effect}
                        <select
                          onChange={(event) =>
                            updateLayerEffect(selectedLayer.id, {
                              kind: event.target
                                .value as ProjectState["layers"][number]["effects"][number]["kind"],
                            })
                          }
                          value={selectedEffect.kind}
                        >
                          {effectOptions.map((value) => (
                            <option key={value} value={value}>
                              {labels.effect[value]}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label>
                        {t.effectDuration}
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
                        {t.effectAmount}
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
                        {t.effectX}
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
                        {t.effectY}
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
                        {t.keyframeProperty}
                        <select
                          onChange={(event) =>
                            updateLayerAnimation(selectedLayer.id, {
                              property: event.target
                                .value as ProjectState["layers"][number]["animations"][number]["property"],
                            })
                          }
                          value={selectedAnimation.property}
                        >
                          {animatedPropertyOptions.map((value) => (
                            <option key={value} value={value}>
                              {labels.animatedProperty[value]}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label>
                        {t.easing}
                        <select
                          onChange={(event) =>
                            updateLayerAnimation(selectedLayer.id, {
                              easing: event.target
                                .value as ProjectState["layers"][number]["animations"][number]["easing"],
                            })
                          }
                          value={selectedAnimation.easing}
                        >
                          {easingOptions.map((value) => (
                            <option key={value} value={value}>
                              {labels.easing[value]}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label>
                        {t.keyframe1Time}
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
                        {t.keyframe1Value}
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
                        {t.keyframe2Time}
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
                        {t.keyframe2Value}
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
              {t.provider}
              <select
                disabled={!voiceLayer}
                onChange={(event) =>
                  voiceLayer &&
                  updateLayerVoice(voiceLayer.id, {
                    provider: event.target.value as TtsProviderKind,
                  })
                }
                value={voiceLayer?.voice.provider ?? "voicevox"}
              >
                {ttsProviderOptions.map((provider) => (
                  <option key={provider} value={provider}>
                    {labels.ttsProvider[provider]}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t.speaker}
              <input
                disabled={!voiceLayer}
                onChange={(event) =>
                  voiceLayer && updateLayerVoice(voiceLayer.id, { speaker: event.target.value })
                }
                value={voiceLayer?.voice.speaker ?? tts.speaker}
              />
            </label>
            <label>
              {t.text}
              <textarea
                disabled={!voiceLayer}
                onChange={(event) =>
                  voiceLayer && updateLayerVoice(voiceLayer.id, { text: event.target.value })
                }
                value={voiceLayer?.voice.text ?? tts.text}
              />
            </label>
            <label>
              {t.speed}
              <input
                disabled={!voiceLayer}
                min={0.5}
                onChange={(event) =>
                  voiceLayer &&
                  updateLayerVoice(voiceLayer.id, { speed: Number(event.target.value) })
                }
                step={0.1}
                type="number"
                value={voiceLayer?.voice.speed ?? tts.speed}
              />
            </label>
            <label>
              {t.pitch}
              <input
                disabled={!voiceLayer}
                onChange={(event) =>
                  voiceLayer &&
                  updateLayerVoice(voiceLayer.id, { pitch: Number(event.target.value) })
                }
                step={0.1}
                type="number"
                value={voiceLayer?.voice.pitch ?? tts.pitch}
              />
            </label>
            <label>
              {t.emotion}
              <input
                disabled={!voiceLayer}
                onChange={(event) =>
                  voiceLayer && updateLayerVoice(voiceLayer.id, { emotion: event.target.value })
                }
                value={voiceLayer?.voice.emotion ?? tts.emotion}
              />
            </label>
          </div>
        </section>

        <section className="timeline-pane" aria-label={t.timeline}>
          <div className="timeline-toolbar">
            <button
              disabled={!selectedLayer}
              onClick={() => selectedLayer && splitLayer(selectedLayer.id, preview.currentTime)}
              title={t.cut}
            >
              <Scissors size={18} />
            </button>
            <button
              disabled={!selectedLayer}
              onClick={() => selectedLayer && duplicateLayer(selectedLayer.id)}
              title={t.duplicate}
            >
              <Copy size={18} />
            </button>
            <button
              disabled={!selectedLayer}
              onClick={() => selectedLayer && deleteLayer(selectedLayer.id)}
              title={t.delete}
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

        <section className="plugin-pane" aria-label={t.pluginManager}>
          <div className="pane-heading">
            <Wand2 size={16} />
            <span>{t.plugins}</span>
            <button
              aria-label={t.installConfiguredPlugins}
              onClick={() =>
                runCommand(
                  () => commands.installConfiguredPlugins(defaultProjectPath),
                  t.configuredPluginsInstalled,
                )
              }
              title={t.installConfiguredPlugins}
            >
              <Download size={16} />
            </button>
          </div>
          <div className="plugin-list">
            {pluginNames.map((pluginName) => (
              <div className="plugin-row" key={pluginName}>
                <span>{pluginName}</span>
                <div className="plugin-actions">
                  <button
                    aria-label={`${t.installPlugin} ${pluginName}`}
                    onClick={() =>
                      runCommand(() => commands.installPlugin(pluginName), t.pluginInstalled)
                    }
                    title={`${t.installPlugin} ${pluginName}`}
                  >
                    <Download size={16} />
                  </button>
                  <button
                    aria-label={`${t.updatePlugin} ${pluginName}`}
                    onClick={() =>
                      runCommand(() => commands.updatePlugin(pluginName), t.pluginUpdated)
                    }
                    title={`${t.updatePlugin} ${pluginName}`}
                  >
                    <RefreshCw size={16} />
                  </button>
                  <button
                    aria-label={`${t.removePlugin} ${pluginName}`}
                    onClick={() =>
                      runCommand(() => commands.removePlugin(pluginName), t.pluginRemoved)
                    }
                    title={`${t.removePlugin} ${pluginName}`}
                  >
                    <X size={16} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      </main>
    </div>
  );
}

function droppedSourcePath(dataTransfer: DataTransfer): string | null {
  const file = dataTransfer.files.item(0);
  const filePath = file ? pathFromFile(file) : null;
  if (filePath) {
    return filePath;
  }
  const uriList = dataTransfer.getData("text/uri-list");
  if (uriList) {
    const uri = uriList
      .split(/\r?\n/)
      .map((line) => line.trim())
      .find((line) => line && !line.startsWith("#"));
    if (uri) {
      return decodeDroppedPath(uri);
    }
  }
  const text = dataTransfer.getData("text/plain").trim();
  return text ? decodeDroppedPath(text) : null;
}

function pathFromFile(file: File): string | null {
  const candidate = file as File & { path?: string };
  return candidate.path && candidate.path.trim() ? candidate.path : null;
}

function decodeDroppedPath(value: string): string {
  if (!value.startsWith("file://")) {
    return value;
  }
  try {
    return decodeURIComponent(new URL(value).pathname);
  } catch {
    return value.replace(/^file:\/\//, "");
  }
}

function inferAssetKind(sourcePath: string): AssetKind {
  const extension = sourcePath.split(/[?#]/)[0]?.split(".").pop()?.toLowerCase();
  if (extension && ["mp4", "mov", "mkv", "webm"].includes(extension)) return "video";
  if (extension && ["mp3", "wav", "m4a", "aac", "ogg", "flac"].includes(extension)) return "audio";
  if (extension && ["srt", "ass", "vtt"].includes(extension)) return "subtitle";
  if (extension && ["ttf", "otf", "woff", "woff2"].includes(extension)) return "font";
  if (extension && ["svg"].includes(extension)) return "mask";
  return "image";
}

function pluginDisplayName(plugin: PluginDeclaration): string {
  if ((plugin.repository === "github" || plugin.repository === "gitlab") && plugin.repo) {
    return plugin.repo;
  }
  if (plugin.repository === "url" && plugin.url) {
    return (
      plugin.url
        .split(/[?#]/)[0]
        ?.split("/")
        .pop()
        ?.replace(/\.wasm$/i, "") || plugin.url
    );
  }
  if (plugin.repository === "local" && plugin.path) {
    return (
      plugin.path
        .split(/[\\/]/)
        .filter(Boolean)
        .pop()
        ?.replace(/\.wasm$/i, "") || plugin.path
    );
  }
  return plugin.repository;
}
