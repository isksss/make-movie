package mmsdk

import (
	"strings"
	"testing"
)

func TestMetadataJSON(t *testing.T) {
	metadata := Metadata{
		Name:        "go-plugin",
		Version:     "0.1.0",
		Category:    CategoryUtility,
		DisplayName: "Go Plugin",
	}

	value, err := metadata.JSON()
	if err != nil {
		t.Fatalf("metadata JSON should be valid: %v", err)
	}

	if !strings.Contains(value, `"name":"go-plugin"`) {
		t.Fatalf("metadata JSON does not contain name: %s", value)
	}
}

func TestMetadataValidationRejectsInvalidCategory(t *testing.T) {
	metadata := Metadata{Name: "go-plugin", Version: "0.1.0", Category: "invalid"}

	if err := metadata.Validate(); err == nil {
		t.Fatal("invalid category should fail")
	}
}

func TestNoopPluginMetadataIsValid(t *testing.T) {
	metadata := NoopPlugin{}.Metadata()

	if err := metadata.Validate(); err != nil {
		t.Fatalf("metadata should be valid: %v", err)
	}

	if metadata.Category != CategoryUtility {
		t.Fatalf("noop metadata should contain category")
	}
}
