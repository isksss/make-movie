package gobasic

import (
	"strings"
	"testing"
)

func TestMetadata(t *testing.T) {
	plugin := &Plugin{}

	if !strings.Contains(plugin.Metadata(), `"name":"go-basic"`) {
		t.Fatalf("metadata should contain plugin name: %s", plugin.Metadata())
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
