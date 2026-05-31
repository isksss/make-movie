package gobasic

import (
	"strings"
	"testing"

	mmsdk "github.com/isksss/make-movie/plugin-api/sdk/go"
)

func TestMetadata(t *testing.T) {
	plugin := &Plugin{}
	metadata := mmsdk.MustMetadataJSON(plugin.Metadata())

	if !strings.Contains(metadata, `"name":"go-basic"`) {
		t.Fatalf("metadata should contain plugin name: %s", metadata)
	}
}

func TestLifecycle(t *testing.T) {
	plugin := &Plugin{}

	if err := plugin.Initialize(); err != nil {
		t.Fatalf("initialize failed: %v", err)
	}
	if !plugin.Initialized() {
		t.Fatal("plugin should be initialized")
	}

	if err := plugin.Shutdown(); err != nil {
		t.Fatalf("shutdown failed: %v", err)
	}
	if plugin.Initialized() {
		t.Fatal("plugin should be shutdown")
	}
}
