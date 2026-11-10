#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "$0")/.."

# Inspect metadata first so a broken dependency tree cannot look like an absent daemon.
packages=$(cargo metadata --no-deps --format-version 1)
if ! python3 -c 'import json, sys; sys.exit(not any(p["name"] == "galleriad" for p in json.load(sys.stdin)["packages"]))' <<< "$packages"; then
    echo "galleriad does not exist yet; boundary check skipped"
    exit 0
fi

tree=$(cargo tree -p galleriad -e normal --prefix none)
if [[ "$tree" == *galleria-profile* ]]; then
    echo "boundary violation: galleriad depends on galleria-profile" >&2
    exit 1
fi
echo "galleriad does not depend on galleria-profile"
