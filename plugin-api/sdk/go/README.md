# mm-sdk-go

Go SDK for make-movie plugins.

## Install

```bash
go get github.com/isksss/make-movie/plugin-api/sdk/go
```

## Example

```go
package main

import mmsdk "github.com/isksss/make-movie/plugin-api/sdk/go"

type MyPlugin struct{}

func (plugin MyPlugin) Metadata() mmsdk.Metadata {
	return mmsdk.Metadata{
		Name:     "my-plugin",
		Version:  "0.1.0",
		Category: mmsdk.CategoryUtility,
	}
}

func (plugin MyPlugin) Initialize() error {
	return nil
}

func (plugin MyPlugin) Shutdown() error {
	return nil
}

var _ mmsdk.Plugin = MyPlugin{}
```

## Release Tag

Go は GitHub module path から取得します。tag は submodule path を prefix にします。

```bash
git tag plugin-api/sdk/go/v0.1.0
git push origin plugin-api/sdk/go/v0.1.0
```
