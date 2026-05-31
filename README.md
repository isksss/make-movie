# make-movie (mm)

make-movie は、宣言的動画生成エンジン兼デスクトップ動画編集ソフトです。

`mm.toml` を唯一の Source of Truth とし、GUI も CLI も同じプロジェクト定義を読み書きします。

```text
media/
mm.toml
mm.lock
↓
動画生成
```

## 目的

- YouTube Shorts、TikTok、Instagram Reels、YouTube 横動画を生成・編集できること
- 解説動画、商品紹介動画、ニュース動画、広告動画、ゲーム実況動画、リアクション動画を扱えること
- Plugin なしでも動画編集ソフトとして成立すること
- WASM Component Model による Plugin Platform として拡張できること

## 技術スタック

- Core: Rust
- GUI: React / TypeScript / Vite / Zustand / Tauri
- Graphics: Skia
- Video: FFmpeg
- Plugin: WASM Component Model / WASI Preview2 / WIT

## Repository 構成

```text
crates/
  mm-core/
  mm-render/
  mm-plugin-runtime/
  mm-cli/
apps/
  mm-gui/
plugin-api/
docs/
examples/
```

## 開発方針

初期実装順は Project、Asset、Timeline、Renderer、CLI、GUI、Plugin Runtime です。

詳細は [PLANS.md](PLANS.md) と [docs/architecture.md](docs/architecture.md) を参照してください。

## 開発環境

ランタイムは repository root の `.mise.toml` を正とします。

```bash
mise install
```

標準 Node.js は 26 系、pnpm は `10.24.0` です。
