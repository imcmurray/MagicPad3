#!/usr/bin/env bash
# Compatibility wrapper — prefer the EndeavourOS/Arch installer.
# For helpers-only:  ./scripts/install-endeavouros.sh --helpers
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec "$ROOT/scripts/install-endeavouros.sh" "$@"
