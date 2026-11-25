#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../.."
BOUNDARY_TREE=$'galleriad v0.1.0\ngalleria-ipc v0.1.0' scripts/check-boundaries.sh >/dev/null
for forbidden in galleria-profile galleria-gui; do
  if BOUNDARY_TREE=$'galleriad v0.1.0\n'"$forbidden v0.1.0" scripts/check-boundaries.sh >/dev/null 2>&1; then
    echo "boundary check accepted $forbidden" >&2; exit 1
  fi
done
echo "boundary check rejects forbidden dependencies"
