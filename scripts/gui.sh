#!/usr/bin/env bash
# Build, test and run the Galleria GUI in the `galleria-gui-dev` image, so the host needs no WebKitGTK, Node or pnpm.
#
#   scripts/gui.sh image          build the image (rebuilt on its own when the Dockerfile, .nvmrc or pnpm change)
#   scripts/gui.sh run CMD...     run CMD in the image, repo mounted at /work, as the invoking user
#   scripts/gui.sh check          the gui checks: ui tests, ui build, clippy and tests of galleria-gui
#   scripts/gui.sh check-here     the same checks on this machine (used inside the image and by the `gui` CI job)
#   scripts/gui.sh build          build ui/ and the app as for a release, without bundles: target/gui-image/release/galleria-gui
#   scripts/gui.sh smoke          build the app and the fake daemon, then the WebDriver smoke test (lab/tests/gui-smoke.sh)
#   scripts/gui.sh pnpm-version   print the pnpm version pinned in package.json (for CI)
#
# Like lab/build.sh: the host Rust toolchain is mounted read-only, the build runs as the invoking user (rootless
# Docker maps container root to that user), and nothing is written outside the repository and the cargo registry.
set -euo pipefail
cd -- "$(dirname -- "$0")/.."

IMAGE=galleria-gui-dev
DOCKERFILE=lab/images/gui.Dockerfile

node_version() { tr -d '[:space:]' < .nvmrc | sed 's/^v//'; }
pnpm_version() { sed -n 's/.*"packageManager": *"pnpm@\([^"]*\)".*/\1/p' package.json; }

# Everything that goes into the image; the image carries it as a label so a stale image is noticed.
image_stamp() { { cat "$DOCKERFILE" .nvmrc; pnpm_version; } | sha256sum | cut -d' ' -f1; }

ensure_image() {
  local stamp have
  stamp=$(image_stamp)
  have=$(docker image inspect "$IMAGE" --format '{{ index .Config.Labels "galleria.stamp" }}' 2>/dev/null || true)
  if [[ ${GUI_REBUILD:-0} == 1 || $have != "$stamp" ]]; then
    docker build -t "$IMAGE" -f "$DOCKERFILE" \
      --build-arg "NODE_VERSION=$(node_version)" --build-arg "PNPM_VERSION=$(pnpm_version)" \
      --label "galleria.stamp=$stamp" lab/images
  fi
}

run_in_image() {
  ensure_image
  # Build as the invoking user; rootless Docker already maps container root to that user.
  local user
  user="$(id -u):$(id -g)"
  if docker info --format '{{.SecurityOptions}}' 2>/dev/null | grep -q rootless; then user=0:0; fi
  mkdir -p "$HOME/.cargo/registry" target/gui-home
  local -a opts=(--rm --user "$user" -v "$PWD:/work" -w /work
    -v "${RUSTUP_HOME:-$HOME/.rustup}:/opt/rustup:ro"
    -v "$HOME/.cargo/bin:/opt/cargo/bin:ro" -v "$HOME/.cargo/registry:/cargo-registry"
    -e HOME=/work/target/gui-home -e RUSTUP_HOME=/opt/rustup -e CARGO_HOME=/work/target/gui-home/cargo
    -e CARGO_TARGET_DIR=/work/target/gui-image -e GUI_INSIDE_IMAGE=1
    -e PATH=/opt/cargo/bin:/opt/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin)
  [[ -z ${GUI_CONTAINER_NAME:-} ]] || opts+=(--name "$GUI_CONTAINER_NAME")
  docker run "${opts[@]}" "$IMAGE" bash -euc \
    'mkdir -p "$CARGO_HOME"; ln -sfn /cargo-registry "$CARGO_HOME/registry"; exec "$@"' gui-run "$@"
}

check_here() {
  pnpm install --frozen-lockfile
  pnpm --dir ui test
  pnpm --dir ui typecheck
  pnpm --dir ui build
  cargo clippy -p galleria-gui --all-targets -- -D warnings
  cargo test -p galleria-gui
}

# The Tauri CLI sets the `custom-protocol` feature of `tauri` for release builds. It must run from the repository
# root (the CLI finds crates/galleria-gui/tauri.conf.json from there) and needs ui/dist to exist first.
build_here() {
  pnpm install --frozen-lockfile
  pnpm --dir ui build
  pnpm tauri build --no-bundle
}

# Level 3 (spec §14): the app built as for a release and the real daemon, driven through WebDriver by
# lab/tests/gui-smoke.sh. tauri-driver is installed once under target/gui-tools (it is not part of the workspace).
smoke_here() {
  local target=${CARGO_TARGET_DIR:-$PWD/target}
  build_here
  cargo build -p galleria-gui --example fake-daemon
  [[ -x target/gui-tools/bin/tauri-driver ]] || cargo install tauri-driver --locked --root target/gui-tools
  GALLERIA_SMOKE_APP="$target/release/galleria-gui" GALLERIA_SMOKE_DAEMON="$target/debug/examples/fake-daemon" \
    GALLERIA_SMOKE_DRIVER="$PWD/target/gui-tools/bin/tauri-driver" lab/tests/gui-smoke.sh
}

case "${1:-}" in
  image) ensure_image ;;
  run) shift; (( $# )) || { echo 'usage: scripts/gui.sh run CMD...' >&2; exit 2; }; run_in_image "$@" ;;
  check) run_in_image scripts/gui.sh check-here ;;
  check-here) check_here ;;
  build) run_in_image scripts/gui.sh build-here ;;
  build-here) build_here ;;
  smoke) run_in_image scripts/gui.sh smoke-here ;;
  smoke-here) smoke_here ;;
  pnpm-version) pnpm_version ;;
  *) echo 'usage: scripts/gui.sh <image|run CMD...|check|check-here|build|smoke|pnpm-version>' >&2; exit 2 ;;
esac
