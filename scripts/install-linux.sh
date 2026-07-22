#!/usr/bin/env bash
# Install MagicPad Companion Linux helpers (udev + optional remapper profile).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RULE_SRC="$ROOT/packaging/linux/99-magic-trackpad.rules"
RULE_DST="/etc/udev/rules.d/99-magic-trackpad.rules"
PROFILE_SRC="$ROOT/packaging/linux/input-remapper-profiles/MagicPad.json"

if [[ ! -f "$RULE_SRC" ]]; then
  echo "Missing $RULE_SRC" >&2
  exit 1
fi

echo "Installing udev rules to $RULE_DST (requires root)…"
sudo install -m 644 "$RULE_SRC" "$RULE_DST"
sudo udevadm control --reload-rules
sudo udevadm trigger
echo "udev rules installed."

# Ensure input group membership hint
if ! id -nG "$USER" | grep -qw input; then
  echo "Hint: add your user to the input group:"
  echo "  sudo usermod -aG input $USER && newgrp input"
fi

if [[ -f "$PROFILE_SRC" ]]; then
  DEST_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/input-remapper-2/presets/Magic Trackpad"
  mkdir -p "$DEST_DIR"
  cp "$PROFILE_SRC" "$DEST_DIR/MagicPad.json"
  echo "input-remapper profile staged at $DEST_DIR/MagicPad.json"
fi

CFG="${XDG_CONFIG_HOME:-$HOME/.config}/magicpad-companion"
mkdir -p "$CFG"
cp "$ROOT/packaging/linux/magicpad-companion.service" \
  "${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/" 2>/dev/null || true

echo "Done. Replug the Magic Trackpad, then launch MagicPad Companion."
