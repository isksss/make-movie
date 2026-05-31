# Known Issues

- Skia backend は raster surface の frame 生成、Layer 合成、Image layer の基本 mask / blur / brightness / contrast / saturation / pixelate / motion blur effect ネイティブ描画、シンプルな Text layer と stroke / shadow / gradient / rotation / letter spacing 付き Text layer のネイティブ描画に対応済みです。
- GPU backend は wgpu による frame clear と、mask / rotation / fit / transition を使わない Image layer の矩形 texture 合成に対応済みです。未対応 layer は既存 CPU 合成へフォールバックします。
- 動画 layer は preview 用 frame decode と FFmpeg encode 合成に対応済みです。
- 音声 layer / Voice layer は FFmpeg mux に対応済みです。
- Text / Subtitle は CPU glyph rendering と srt / vtt / ass の最小 parse に対応済みです。
