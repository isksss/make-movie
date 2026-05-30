# make-movie Plugin SDK

このディレクトリは `plugin-api/plugin.wit` を正本として利用する最小 SDK 雛形を管理します。

## 契約

唯一の ABI 契約は `../plugin.wit` です。SDK は Plugin 実装者が各言語で同じ export を実装しやすくするための薄い補助層です。

必須 export:

- `metadata() -> string`
- `initialize()`
- `shutdown()`

## SDK

- `rust/`: `mm-sdk-rust`
- `go/`: `mm-sdk-go`
- `ts/`: `mm-sdk-ts`
- `csharp/`: `mm-sdk-csharp`

## 検証

```bash
bash plugin-api/sdk/verify.sh
```

この検証は `plugin.wit` の必須 export と、各 SDK の最小ファイルが揃っていることを確認します。
