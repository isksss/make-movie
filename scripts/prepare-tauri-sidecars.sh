#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_triple="${TAURI_TARGET_TRIPLE:-$(rustc -vV | sed -n 's/^host: //p')}"
binary_dir="$root/apps/mm-gui/src-tauri/binaries"

if [[ -z "$target_triple" ]]; then
  echo "target triple を取得できません" >&2
  exit 1
fi

mkdir -p "$binary_dir"

copy_sidecar() {
  local name="$1"
  local source
  source="$(command -v "$name" || true)"
  if [[ -z "$source" ]]; then
    echo "$name が PATH に見つかりません" >&2
    exit 1
  fi

  local extension=""
  if [[ "$target_triple" == *"windows"* ]]; then
    extension=".exe"
  fi

  local target="$binary_dir/$name-$target_triple$extension"
  cp "$source" "$target"
  chmod 755 "$target"
  printf '%s\n' "$target"
}

copy_sidecar ffmpeg
copy_sidecar ffprobe
