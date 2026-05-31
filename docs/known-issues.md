# Known Issues

- Skia backend は raster surface の frame 生成、Layer 合成、Image layer の基本 mask / blur effect / brightness effect / saturation effect ネイティブ描画、シンプルな Text layer と stroke / shadow / gradient / rotation / letter spacing 付き Text layer のネイティブ描画に対応済みです。contrast / pixelate / motion blur effect の Skia ネイティブ描画は段階的に拡張します。
- 動画 layer は preview 用 frame decode と FFmpeg encode 合成に対応済みです。
- 音声 layer / Voice layer は FFmpeg mux に対応済みです。
- Text / Subtitle は CPU glyph rendering と srt / vtt / ass の最小 parse に対応済みです。
