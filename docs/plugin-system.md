# Plugin System

## 方針

Plugin は追加機能専用です。Core 機能は置き換えません。

## ABI

`plugin-api/plugin.wit` を唯一の契約とします。

詳細は `plugin-api/docs/abi.md` を参照します。

## Manifest / Lock

Plugin manifest と lock の schema は `plugin-api/schema/` に配置します。

- `plugin-api/schema/plugin-manifest.schema.json`
- `plugin-api/schema/plugin-lock.schema.json`

manifest の記述例は `plugin-api/docs/manifest.md` を参照します。

## 対応 repository

- GitHub
- GitLab
- URL
- Local

## 解決順

```text
Global Config
↓
Project Config
↓
Merge
↓
Resolve
↓
Download
↓
Load
```

Project config を優先します。
