# Plugin Manifest

## 目的

Plugin manifest は Plugin Manager が resolve / download / verify / install / update / remove を行うための入力です。

JSON Schema は `plugin-api/schema/plugin-manifest.schema.json` に配置します。

## 基本形

```toml
name = "voicevox"
version = "1.0.0"
component = "voicevox.wasm"

[metadata]
display_name = "VOICEVOX"
category = "tts"
description = "VOICEVOX TTS plugin"

[source]
repository = "github"
owner = "make-movie"
repo = "voicevox"
version = "1.0.0"
```

## category

- `ai`
- `subtitle`
- `tts`
- `template`
- `export`
- `utility`

## repository

### GitHub

```toml
[source]
repository = "github"
owner = "isksss"
repo = "theme"
version = "1.0.0"
```

### GitLab

```toml
[source]
repository = "gitlab"
owner = "isksss"
repo = "theme"
version = "1.0.0"
```

### URL

```toml
[source]
repository = "url"
url = "https://example.com/plugin.wasm"
version = "1.0.0"
```

### Local

```toml
[source]
repository = "local"
path = "./plugins/foo"
```

## Lock

`mm.lock` の Plugin lock schema は `plugin-api/schema/plugin-lock.schema.json` に配置します。

```toml
[[plugin]]
name = "voicevox"
version = "1.0.0"
checksum = "sha256:..."
path = "plugins/voicevox/voicevox.wasm"
```
