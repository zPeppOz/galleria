#!/bin/bash
# usage: run.sh <cargo args>   e.g. run.sh test --workspace --exclude galleria-e2e --no-fail-fast
# Runs cargo for x86_64-pc-windows-gnu in the container XWINE_CONTAINER_NAME (default galleria-xwine), from the image
# galleria-xbuild-wine (see README.md); the test binaries are run by wine64. Wine is not Windows (Dockerfile).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
. scripts/build-cache.sh
cache=$(build_cache_dir xwine)
mkdir -p "$HOME/.cache/galleria-xcargo/rustup"
docker run --rm --name "${XWINE_CONTAINER_NAME:-galleria-xwine}" \
  -e CARGO_HOME=/cargo -e RUSTUP_HOME=/cargo/rustup -e CARGO_TARGET_DIR=/src/target/xwine -e CARGO_BUILD_BUILD_DIR=/cache \
  -e CARGO_BUILD_TARGET=x86_64-pc-windows-gnu -e CARGO_TERM_COLOR=never \
  -e CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER=wine -e WINEARCH=win64 -e WINEDEBUG=-all -e WINEPREFIX=/tmp/wineprefix \
  -e RUST_TEST_THREADS=4 \
  -v "$HOME/.cache/galleria-xcargo:/cargo" -v "$cache:/cache" -v "$PWD:/src" -w /src galleria-xbuild-wine bash -euc '
    rustup target add x86_64-pc-windows-gnu >/dev/null 2>&1
    dll=$(dirname "$(find /usr/lib/gcc/x86_64-w64-mingw32 -name libgcc_s_seh-1.dll | grep posix | head -1)")
    wv=$(dirname "$(find /cargo/registry/src -path "*webview2-com-sys*/x64/WebView2Loader.dll" | head -1)"); export WINEPATH="$dll;/usr/x86_64-w64-mingw32/lib;$wv"; echo "WINEPATH=$WINEPATH" >&2
    cargo "$@"' bash "$@"
