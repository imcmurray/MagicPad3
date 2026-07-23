#!/usr/bin/env bash
# Compatibility wrapper → scripts/install-endeavouros.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec "$ROOT/scripts/install-endeavouros.sh" "$@"
