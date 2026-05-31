package gobasic

import mmsdk "github.com/isksss/make-movie/plugin-api/sdk/go"

type Plugin struct {
	initialized bool
}

var _ mmsdk.Plugin = (*Plugin)(nil)

func (plugin *Plugin) Metadata() mmsdk.Metadata {
	return mmsdk.Metadata{
		Name:        "go-basic",
		Version:     "0.1.0",
		Category:    mmsdk.CategoryUtility,
		DisplayName: "Go Basic",
		Description: "Go plugin template",
	}
}

func (plugin *Plugin) Initialize() error {
	plugin.initialized = true
	return nil
}

func (plugin *Plugin) Shutdown() error {
	plugin.initialized = false
	return nil
}

func (plugin *Plugin) Initialized() bool {
	return plugin.initialized
}
