# ロードマップ

## Phase 1: Core / CLI

- Project / Asset / Timeline / Scene のモデルを安定させる。
- `mm.toml` / `mm.lock` の読み書きと検証を強化する。
- CLI の `build` / `validate` / `preview` / `doctor` を実運用できる品質にする。
- FFmpeg locator と mp4 出力を安定させる。

## Phase 2: Renderer

- Text / Subtitle / Animation / Mask / Wipe / Effects / Transition を拡充する。
- CPU renderer と GPU renderer の差分をテストで確認できるようにする。
- FFmpeg による decode / encode / audio processing を拡張する。

## Phase 3: GUI

- Timeline / Preview / Property / Asset Browser を編集用途として完成させる。
- Cut / Trim / Crop / Resize / Rotate / Wipe / Keyframe の操作性を高める。
- `mm.toml` との同期を維持しながら Undo / Redo を拡充する。
- 日本語・英語 UI を継続的に維持する。

## Phase 4: Plugin Platform

- WASM Component Model / WASI Preview2 / WIT ABI を拡張する。
- GitHub / GitLab / URL / Local plugin の解決、検証、インストールを強化する。
- AI / Subtitle / TTS / Template / Export / Utility plugin を追加できる SDK を整備する。

## Phase 5: 配布

- CLI と GUI のリリース手順を自動化する。
- Windows / macOS / Linux の配布物を検証する。
- FFmpeg / ffprobe 同梱方針とライセンス表記を整備する。
