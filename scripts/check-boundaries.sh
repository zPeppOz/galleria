#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "$0")/.."

# BOUNDARY_TREE replaces the real dependency tree of galleriad; scripts/tests/check-boundaries.sh sets it to prove
# that each forbidden dependency is rejected.
if [[ -z ${BOUNDARY_TREE+x} ]]; then
    # Inspect metadata first so a broken dependency tree cannot look like an absent daemon.
    packages=$(cargo metadata --no-deps --format-version 1)
    if ! python3 -c 'import json, sys; sys.exit(not any(p["name"] == "galleriad" for p in json.load(sys.stdin)["packages"]))' <<< "$packages"; then
        echo "galleriad does not exist yet; boundary check skipped"
        exit 0
    fi
    tree=$(cargo tree -p galleriad -e normal --prefix none)
else
    tree=$BOUNDARY_TREE
fi

for forbidden in galleria-profile galleria-gui; do
    if [[ "$tree" == *"$forbidden"* ]]; then
        echo "boundary violation: galleriad depends on $forbidden" >&2
        exit 1
    fi
done
echo "galleriad does not depend on galleria-profile or galleria-gui"
