# Go Plugin Template

Go で make-movie Plugin を作る最小テンプレートです。

## Install

外部 repository で使う場合は SDK を GitHub module path から追加します。

```bash
go get github.com/isksss/make-movie/plugin-api/sdk/go
```

この repository 内の検証では `replace` で `../../sdk/go` を参照します。

## Test

```bash
go test ./...
go vet ./...
```

Go Plugin を Runtime で読み込む場合は、最終成果物を `plugin-api/plugin.wit` と互換の WASM Component に変換します。TinyGo / WASI Preview2 / Component Model の toolchain を採用する場合は、SDK、docs、CIを同じチケットで更新します。
