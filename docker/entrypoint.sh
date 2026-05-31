#!/usr/bin/env bash
set -euo pipefail

cd /workspace

if [[ -z "${XDG_RUNTIME_DIR:-}" ]]; then
  export XDG_RUNTIME_DIR="/tmp/runtime-$(id -u)"
  mkdir -p "$XDG_RUNTIME_DIR"
  chmod 700 "$XDG_RUNTIME_DIR"
fi

if [[ -z "${DISPLAY:-}" && "${MM_DOCKER_XVFB:-1}" == "1" ]]; then
  export DISPLAY=:99
  Xvfb "$DISPLAY" -screen 0 1280x720x24 >/tmp/mm-xvfb.log 2>&1 &
  xvfb_pid=$!
  trap 'kill "$xvfb_pid" >/dev/null 2>&1 || true' EXIT
fi

if [[ "${MM_DOCKER_BOOTSTRAP:-1}" == "1" ]]; then
  corepack pnpm config set store-dir /home/ubuntu/.local/share/pnpm/store

  if [[ -f apps/mm-gui/pnpm-lock.yaml ]]; then
    corepack pnpm --dir apps/mm-gui install --frozen-lockfile
    corepack pnpm --dir apps/mm-gui exec playwright install chromium
  fi

  if [[ -f plugin-api/sdk/ts/package.json ]]; then
    corepack pnpm --dir plugin-api/sdk/ts install --frozen-lockfile
  fi
fi

exec "$@"
