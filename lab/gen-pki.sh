#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p pki out
if [[ ! -s pki/ta.key ]]; then
  docker run --rm --name glab-pki -v "$PWD/pki:/pki" -v "$PWD/server:/lab-tools:ro" glab-server bash /lab-tools/gen-pki.sh
fi
# Generate profiles inside a container: generated keys stay root-owned, mode 0600.
docker run --rm --name glab-profiles -v "$PWD/pki:/pki:ro" -v "$PWD/out:/out" -v "$PWD/server:/lab-tools:ro" glab-server python3 /lab-tools/make-profiles.py
