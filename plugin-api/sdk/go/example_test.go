package mmsdk_test

import mmsdk "github.com/isksss/make-movie/plugin-api/sdk/go"

type MinimalPlugin struct{}

func (plugin MinimalPlugin) Metadata() mmsdk.Metadata {
	return mmsdk.Metadata{
		Name:     "minimal-plugin",
		Version:  "0.1.0",
		Category: mmsdk.CategoryUtility,
	}
}

func (plugin MinimalPlugin) Initialize() error {
	return nil
}

func (plugin MinimalPlugin) Shutdown() error {
	return nil
}

var _ mmsdk.Plugin = MinimalPlugin{}
