# Known Issues

- 完全な Skia backend は段階的に拡張します。
- 動画 layer は preview 用 frame decode と FFmpeg encode 合成に対応済みです。
- 音声 layer / Voice layer は FFmpeg mux に対応済みです。
- Text / Subtitle は CPU glyph rendering と srt / vtt / ass の最小 parse に対応済みです。
