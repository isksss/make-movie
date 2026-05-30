# リリースプロセス

## ブランチ

- `develop` は開発統合ブランチとして保持する。
- `main` は本番ブランチとして扱う。
- feature branch は PR マージ後に削除してよい。
- `develop` を `main` へ反映するときは、`develop` ブランチを削除しない。

## リリース前検証

`main` へマージする前に、変更範囲に応じて `docs/verification.md` の検証を通します。

最低限の確認:

- Rust workspace の format / clippy / test
- GUI の lint / unit test / E2E / build
- Tauri backend の test / check
- FFmpeg が利用可能な環境での CLI build または preview

## 配布方針

- CLI 配布物は `mm` のみとし、FFmpeg は同梱しない。
- GUI 配布物は `mm-gui`、`ffmpeg`、`ffprobe` を含める。
- 本体ライセンスは MIT とする。
- FFmpeg は LGPL 条件を満たす形で同梱する。

## GUI sidecar 準備

GUI bundle 前に、`apps/mm-gui/src-tauri/binaries/` へ target triple 付きの
`ffmpeg` / `ffprobe` を配置します。

```text
apps/mm-gui/src-tauri/binaries/ffmpeg-<target-triple>
apps/mm-gui/src-tauri/binaries/ffprobe-<target-triple>
```

Windows target では `.exe` を付けます。

```text
apps/mm-gui/src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe
apps/mm-gui/src-tauri/binaries/ffprobe-x86_64-pc-windows-msvc.exe
```

`tauri.conf.json` の `bundle.externalBin` で `binaries/ffmpeg` と
`binaries/ffprobe` を指定しているため、Tauri bundle 時に同梱されます。

## リリース記録

リリース PR には以下を記載します。

- 対象 Issue / PR
- 変更概要
- 実行した検証
- 既知の問題
- 配布物の対象プラットフォーム
