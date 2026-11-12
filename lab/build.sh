#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
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
  # Use the installed global toolchain read-only, without copying it into the worktree.
  docker run --rm --user "$(id -u):$(id -g)" --name glab-build -v "$PWD:/work" -w /work \
    -v "${RUSTUP_HOME:-$HOME/.rustup}:/opt/rustup:ro" \
    -v "$HOME/.cargo/bin:/opt/cargo/bin:ro" -v "$HOME/.cargo/registry:/cargo-registry" \
    -e HOME=/work/target/lab-cargo-home -e RUSTUP_HOME=/opt/rustup -e CARGO_HOME=/work/target/lab-cargo-home \
    -e CARGO_TARGET_DIR=/work/target/lab-build -e PATH=/opt/cargo/bin:/usr/bin:/bin \
    glab-build bash -euc 'mkdir -p "$CARGO_HOME"; ln -sfn /cargo-registry "$CARGO_HOME/registry"; cargo build --workspace; cargo test -p galleria-e2e --no-run --message-format=json > lab/out/artifacts.json'
  build_target=target/lab-build
else
  cargo build --workspace
  cargo test -p galleria-e2e --no-run --message-format=json > lab/out/artifacts.json
  build_target=${CARGO_TARGET_DIR:-target}
fi
cp "$build_target/debug/galleriad" "$build_target/debug/ipc-probe" lab/out/bin/
python3 - <<'PY'
import json, pathlib, shutil
out=pathlib.Path('lab/out/tests')
for old in out.iterdir(): old.unlink()
for line in pathlib.Path('lab/out/artifacts.json').read_text().splitlines():
    event=json.loads(line)
    if event.get('reason')=='compiler-artifact' and event.get('executable') and event['target']['name'].startswith('t'):
        source=pathlib.Path(event['executable'])
        if str(source).startswith('/work/'): source=pathlib.Path(str(source)[6:])
        shutil.copy2(source,out/event['target']['name'])
assert list(out.iterdir()), 'no integration test executables built'
PY
