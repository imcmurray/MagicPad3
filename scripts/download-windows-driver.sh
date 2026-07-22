#!/usr/bin/env bash
# Download notes helper — opens / prints the recommended Windows driver release page.
# Binary download is intentional opt-in (GPL, architecture-specific, signed packages).
set -euo pipefail

URL="https://github.com/vitoplantamura/MagicTrackpad2ForWindows/releases"
FALLBACK="https://github.com/imbushuo/mac-precision-touchpad/releases"

echo "Recommended Precision driver releases:"
echo "  $URL"
echo "Fallback (imbushuo):"
echo "  $FALLBACK"
echo
echo "After downloading, extract AMD64 or ARM64 INF tree to:"
echo "  %LOCALAPPDATA%\\MagicPadCompanion\\drivers\\"
echo "Then use MagicPad Companion → Driver → Install driver."

if command -v xdg-open >/dev/null 2>&1; then
  xdg-open "$URL" || true
elif command -v open >/dev/null 2>&1; then
  open "$URL" || true
fi
