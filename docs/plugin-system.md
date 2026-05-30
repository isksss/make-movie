# Plugin System

## 方針

Plugin は追加機能専用です。Core 機能は置き換えません。

## ABI

`plugin-api/plugin.wit` を唯一の契約とします。

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
