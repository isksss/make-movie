# PLANS.md

## 初期実装順

1. Repository foundation
2. Core project model and TOML I/O
3. Renderer and FFmpeg export
4. CLI commands
5. Plugin runtime and manager
6. Desktop GUI

## MVP 完了条件

- `mm.toml` を読み込める。
- Asset、Timeline、Scene を Core で表現できる。
- CLI から validate と build を実行できる。
- Renderer が frame を生成し、FFmpeg で mp4 を出力できる。
- GUI が `mm.toml` を読み書きする編集画面を持つ。
- Plugin Runtime が WIT ABI を唯一の契約として plugin lifecycle を扱える。

## 最終目標

動画編集ソフト、宣言的動画生成エンジン、WASM Plugin Platform を 1 つの製品として成立させます。
