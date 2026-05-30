# FFmpeg sidecar binaries

Tauri bundle は `tauri.conf.json` の `bundle.externalBin` で次の sidecar を同梱する。

- `binaries/ffmpeg`
- `binaries/ffprobe`

実ファイルは Git 管理しない。bundle 前に Tauri の target triple を付けた名前で配置する。

例:

```text
apps/mm-gui/src-tauri/binaries/ffmpeg-x86_64-unknown-linux-gnu
apps/mm-gui/src-tauri/binaries/ffprobe-x86_64-unknown-linux-gnu
```

Windows の場合は `.exe` を含める。

```text
apps/mm-gui/src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe
apps/mm-gui/src-tauri/binaries/ffprobe-x86_64-pc-windows-msvc.exe
```
