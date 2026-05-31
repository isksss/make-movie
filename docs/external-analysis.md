# External Analysis

外部解析結果を make-movie の既存 timeline model へ適用するための汎用仕様です。

## 方針

- `ai-tracking` 専用 API は追加しない。
- `plugin-api/plugin.wit` は v1 では変更しない。
- 外部解析は GUI Worker や外部Pluginで実行し、Core には結果だけを渡す。
- Core は解析結果を既存の `Animation` と `Crop` 系 property へ変換する。

## Input Shape

```json
{
  "source_width": 1920,
  "source_height": 1080,
  "targets": [
    {
      "id": "person-1",
      "kind": "person",
      "frames": [
        {
          "time": 0.0,
          "bbox": {
            "x": 100.0,
            "y": 120.0,
            "width": 320.0,
            "height": 480.0
          },
          "confidence": 0.92
        }
      ]
    }
  ]
}
```

`kind` は `face`、`person`、`object`、`pose`、`hand`、`scene`、`other` を使います。将来の姿勢推定、手検出、物体検出、シーン解析でも同じ形を使います。

## Conversion

`bbox` は source media のpixel座標です。

- `x`: `bbox.x + bbox.width / 2`
- `y`: `bbox.y + bbox.height / 2`
- `crop_x`: `bbox.x`
- `crop_y`: `bbox.y`
- `crop_width`: `bbox.width`
- `crop_height`: `bbox.height`

Core は必要に応じて bounds clamp、interval sampling、moving average smoothing を適用します。

## Tauri Command

GUI からは、生成済みのJSON文字列を `apply_external_analysis_result` に渡します。

```ts
await invoke<string>("apply_external_analysis_result", {
  projectToml,
  layerId: "video-main",
  analysisJson,
});
```

戻り値は更新済み `mm.toml` 文字列です。失敗時はProjectStateを更新せず、呼び出し側でエラー表示します。
