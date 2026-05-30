package mmsdk

// Plugin mirrors the lifecycle defined in plugin-api/plugin.wit.
type Plugin interface {
	Metadata() string
	Initialize() error
	Shutdown() error
}

// NoopPlugin is useful for examples and tests.
type NoopPlugin struct {
	Value string
}

func (plugin NoopPlugin) Metadata() string {
	if plugin.Value == "" {
		return "{}"
	}
	return plugin.Value
}

func (plugin NoopPlugin) Initialize() error {
	return nil
}

func (plugin NoopPlugin) Shutdown() error {
	return nil
}
