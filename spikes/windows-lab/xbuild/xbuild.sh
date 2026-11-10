#!/bin/bash
# Cross-compila un crate per Windows (x86_64-pc-windows-gnu) dentro Docker: xbuild.sh <cartella-crate> [argomenti cargo]
set -euo pipefail
crate="$(cd "$1" && pwd)"; shift
mkdir -p "$HOME/.cache/galleria-xcargo"
docker run --rm -u "$(id -u):$(id -g)" -e CARGO_HOME=/cargo -e HOME=/tmp \
  -v "$HOME/.cache/galleria-xcargo:/cargo" -v "$crate:/src" -w /src \
  galleria-xbuild cargo build --release --target x86_64-pc-windows-gnu "$@"
