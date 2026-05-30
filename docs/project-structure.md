# Project 構成

## Repository

```text
crates/
apps/
plugin-api/
docs/
examples/
```

## 動画プロジェクト

```text
project/
  mm.toml
  mm.lock
  media/
    video/
    image/
    audio/
    subtitle/
    font/
    script/
    mask/
  output/
  cache/
```

## Asset 管理

デフォルトは copy mode です。

外部ファイルを import すると、`media/` 以下にコピーし、Asset として登録します。
