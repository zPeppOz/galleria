#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/build-cache.sh
client=${1:-26}
case "$client" in 26|27|79) ;; *) echo 'client must be 26, 27 or 79' >&2; exit 2;; esac
free_kb=$(df -Pk "$PWD" | awk 'NR==2 {print $4}')
(( free_kb >= 8388608 )) || { echo 'less than 8 GiB free; stopping' >&2; exit 1; }
mkdir -p lab/out/bin lab/out/tests
# Existing laboratory images are reused. LAB_REBUILD=1 explicitly refreshes them.
image() {
  local tag=$1 file=$2
  shift 2
  if [[ ${LAB_REBUILD:-0} == 1 ]] || ! docker image inspect "$tag" >/dev/null 2>&1; then
    docker build -t "$tag" -f "$file" "$@" lab
  fi
}
image glab-server lab/images/server.Dockerfile
if [[ $client == 79 ]]; then
  image glab-client79 lab/images/client-fedora.Dockerfile
else
  base=ubuntu:24.04
  [[ $client != 27 ]] || base=ubuntu:26.04
  image "glab-client$client" lab/images/client.Dockerfile --build-arg "BASE=$base"
fi
if [[ ${LAB_BUILD:-container} != host && ${GLAB_BUILD_INSIDE:-0} != 1 ]]; then
  image glab-build lab/images/build.Dockerfile
  # Build as the invoking user; rootless Docker already maps container root to that user.
  build_user="$(id -u):$(id -g)"
  if docker info --format '{{.SecurityOptions}}' 2>/dev/null | grep -q rootless; then build_user=0:0; fi
  # The intermediate artifacts of cargo (build-dir) live in a host folder shared by the worktrees (scripts/build-cache.sh);
  # the binaries and artifacts.json stay in target/lab-build. The test executables are in the build-dir: see the copy below.
  build_cache=$(build_cache_dir lab-build)
  # Use the installed global toolchain read-only, without copying it into the worktree.
  docker run --rm --user "$build_user" --name glab-build -v "$PWD:/work" -v "$build_cache:/cargo-build" -w /work \
    -v "${RUSTUP_HOME:-$HOME/.rustup}:/opt/rustup:ro" \
    -v "$HOME/.cargo/bin:/opt/cargo/bin:ro" -v "$HOME/.cargo/registry:/cargo-registry" \
    -e HOME=/work/target/lab-cargo-home -e RUSTUP_HOME=/opt/rustup -e CARGO_HOME=/work/target/lab-cargo-home \
    -e CARGO_TARGET_DIR=/work/target/lab-build -e CARGO_BUILD_BUILD_DIR=/cargo-build -e PATH=/opt/cargo/bin:/usr/bin:/bin \
    glab-build bash -euc 'mkdir -p "$CARGO_HOME"; ln -sfn /cargo-registry "$CARGO_HOME/registry"; cargo build --workspace --exclude galleria-gui; cargo test -p galleria-e2e --no-run --message-format=json > lab/out/artifacts.json'
  build_target=target/lab-build
else
  cargo build --workspace --exclude galleria-gui
  cargo test -p galleria-e2e --no-run --message-format=json > lab/out/artifacts.json
  build_target=${CARGO_TARGET_DIR:-target}
fi
cp "$build_target/debug/galleriad" "$build_target/debug/ipc-probe" "$build_target/debug/galleria" lab/out/bin/
python3 - "${build_cache:-}" <<'PY'
import json, pathlib, shutil, sys
cache=sys.argv[1]
out=pathlib.Path('lab/out/tests')
for old in out.iterdir(): old.unlink()
for line in pathlib.Path('lab/out/artifacts.json').read_text().splitlines():
    event=json.loads(line)
    if event.get('reason')=='compiler-artifact' and event.get('executable') and event['target']['name'].startswith('t'):
        source=pathlib.Path(event['executable'])
        if str(source).startswith('/work/'): source=pathlib.Path(str(source)[6:])
        elif cache and str(source).startswith('/cargo-build/'): source=pathlib.Path(cache)/str(source)[13:]
        shutil.copy2(source,out/event['target']['name'])
assert list(out.iterdir()), 'no integration test executables built'
PY
