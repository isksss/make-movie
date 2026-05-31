# Plugin ABI

## 契約

`plugin-api/plugin.wit` が Core、GUI、CLI、Plugin の唯一の ABI 契約です。

GUI や CLI は Plugin の内部実装に依存せず、Runtime は WIT で定義された export だけを呼び出します。

## 最小インターフェイス

```wit
package mm:plugin;

interface plugin {
  enum plugin-category {
    ai,
    subtitle,
    tts,
    template,
    export,
    utility,
  }

  record plugin-metadata {
    name: string,
    version: string,
    category: plugin-category,
    display-name: option<string>,
    description: option<string>,
  }

  metadata: func() -> plugin-metadata;
  initialize: func();
  shutdown: func();
}
```

## ライフサイクル

1. Plugin Manager が manifest を解決する。
2. component wasm を install directory へ配置する。
3. Plugin Runtime が wasm をロードする。
4. `initialize` を呼び出す。
5. 終了時に `shutdown` を呼び出す。

`metadata` は Plugin 側の追加メタデータを `plugin-metadata` record として返すための関数です。Core の必須編集機能は Plugin で置き換えません。

## 互換性

- ABI 変更は `plugin.wit` の変更として扱う。
- ABI 変更時は SDK 生成物と Plugin Runtime の両方を更新する。
- 破壊的変更は release note に明記する。
