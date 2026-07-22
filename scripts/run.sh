#!/usr/bin/env bash
# Launch the release MagicPad Companion GUI (no Vite / no localhost:1420).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/src-tauri/target/release/magicpad-companion"

# Wayland + WebKitGTK mitigation (Arch/EndeavourOS)
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"

if [[ ! -x "$BIN" ]]; then
  echo "Release binary missing. Building…"
  cd "$ROOT"
  npm run build
  npm run tauri -- build --bundles deb
fi

exec "$BIN" "$@"
