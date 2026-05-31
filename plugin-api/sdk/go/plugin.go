package mmsdk

// Generated from plugin-api/plugin.wit.

import (
	"encoding/json"
	"errors"
	"strings"
)

// Category is the plugin category used by make-movie.
type Category string

const (
	CategoryAI       Category = "ai"
	CategorySubtitle Category = "subtitle"
	CategoryTTS      Category = "tts"
	CategoryTemplate Category = "template"
	CategoryExport   Category = "export"
	CategoryUtility  Category = "utility"
)

// Metadata is the structured metadata returned by Plugin.Metadata.
type Metadata struct {
	Name        string   `json:"name"`
	Version     string   `json:"version"`
	Category    Category `json:"category"`
	DisplayName string   `json:"display_name,omitempty"`
	Description string   `json:"description,omitempty"`
}

func (metadata Metadata) Validate() error {
	if strings.TrimSpace(metadata.Name) == "" {
		return errors.New("metadata name is required")
	}
	if strings.TrimSpace(metadata.Version) == "" {
		return errors.New("metadata version is required")
	}
	switch metadata.Category {
	case CategoryAI, CategorySubtitle, CategoryTTS, CategoryTemplate, CategoryExport, CategoryUtility:
		return nil
	default:
		return errors.New("metadata category is invalid")
	}
}

func (metadata Metadata) JSON() (string, error) {
	if err := metadata.Validate(); err != nil {
		return "", err
	}
	value, err := json.Marshal(metadata)
	if err != nil {
		return "", err
	}
	return string(value), nil
}

func MustMetadataJSON(metadata Metadata) string {
	value, err := metadata.JSON()
	if err != nil {
		panic(err)
	}
	return value
}

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
		return MustMetadataJSON(Metadata{
			Name:     "noop-plugin",
			Version:  "0.1.0",
			Category: CategoryUtility,
		})
	}
	return plugin.Value
}

func (plugin NoopPlugin) Initialize() error {
	return nil
}

func (plugin NoopPlugin) Shutdown() error {
	return nil
}
