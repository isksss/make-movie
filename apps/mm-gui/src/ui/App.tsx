import {
  Box,
  FolderOpen,
  Import,
  Pause,
  Play,
  Save,
  Scissors,
  Settings,
  SkipBack,
  SkipForward,
  Wand2,
} from "lucide-react";
import { useProjectStore } from "../store/projectStore";
import { usePreviewStore } from "../store/previewStore";
import { PreviewCanvas } from "./PreviewCanvas";

export function App() {
  const {
    project,
    selectedAssetId,
    selectedLayerId,
    tts,
    selectAsset,
    selectLayer,
    moveLayer,
    updateTtsText,
  } = useProjectStore();
  const preview = usePreviewStore();
  const selectedLayer =
    project.layers.find((layer) => layer.id === selectedLayerId) ?? project.layers[0];

  return (
    <div className="app-shell">
      <header className="menu-bar">
        <div className="brand">make-movie</div>
        <div className="toolbar" aria-label="Project toolbar">
          <button title="Open project">
            <FolderOpen size={18} />
          </button>
          <button title="Save project">
            <Save size={18} />
          </button>
          <button title="Import asset">
            <Import size={18} />
          </button>
          <button title="Build movie">
            <Wand2 size={18} />
          </button>
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
            <button title="Cut">
              <Scissors size={18} />
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
