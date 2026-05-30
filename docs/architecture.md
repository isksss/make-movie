# アーキテクチャ

## Source of Truth

`mm.toml` が唯一のプロジェクト定義です。

GUI は独自形式を持たず、ProjectState を `mm.toml` と相互変換します。

```text
GUI -> ProjectState -> mm.toml
mm.toml -> ProjectState -> GUI
```

## Core

Core は動画編集ソフトとして成立する基本機能を持ちます。

- Project
- Asset
- Timeline
- Scene
- Text
- Subtitle
- TTS
- Effects
- Transition

Plugin は Core 機能を置き換えず、追加機能だけを提供します。

## Renderer

Skia が描画、FFmpeg が decode / encode / audio processing を担当します。

## GUI

GUI は表示と操作のみを担当し、Timeline Engine、Renderer、TTS、Asset Logic は Core へ委譲します。

## Plugin

`plugin-api/plugin.wit` を唯一の ABI 契約とします。
