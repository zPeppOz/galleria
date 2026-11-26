#!/usr/bin/env bash
# Build, test and run the Galleria GUI in the `galleria-gui-dev` image, so the host needs no WebKitGTK, Node or pnpm.
#
#   scripts/gui.sh image          build the image (rebuilt on its own when the Dockerfile, .nvmrc or pnpm change)
#   scripts/gui.sh run CMD...     run CMD in the image, repo mounted at /work, as the invoking user
#   scripts/gui.sh check          the gui checks: ui tests, ui build, clippy and tests of galleria-gui
#   scripts/gui.sh check-here     the same checks on this machine (used inside the image and by the `gui` CI job)
#   scripts/gui.sh build          build ui/ and the app as for a release, without bundles: target/gui-image/release/galleria-gui
#   scripts/gui.sh smoke          build the app, the fake daemon and the seed of the rubrica, then the WebDriver smoke test (lab/tests/gui-smoke.sh)
#   scripts/gui.sh a11y           the same build, then the screen reader check under Orca and AT-SPI (lab/tests/gui-a11y.sh);
#                                 local only (Q22), writes target/gui-a11y/
#   scripts/gui.sh lab-import     G3, the half in the image: the real window imports the profiles of the laboratory (lab/out) and leaves
#                                 its folder of profiles in target/gui-lab/ (lab/tests/gui-lab-import.sh); the laboratory must be up
#   scripts/gui.sh lab-g3 <26|27|79>   G3 whole, on the host: brings the laboratory up, runs `lab-import` between `lab/up.sh` and the
#                                 tests, then t17 on that client (the real CLI and daemon list and connect what the window imported)
#   scripts/gui.sh socket-owner   the owner check of the single-instance socket against two real users (uid 1001 and 1002) and root, in
#                                 the image as root (lab/tests/gui-socket-owner.sh): the unit tests run as one user and cannot hold it
#   scripts/gui.sh keyring        the keyring store against a real GNOME Keyring (unlocked, locked, absent) and the CLI on the same
#                                 keyring (lab/tests/gui-keyring.sh); ends with scripts/check-gates.py on what ran
#   scripts/gui.sh pnpm-version   print the pnpm version pinned in package.json (for CI)
#
# GUI_UID / GUI_GID set the user of the container (default: the invoking user), to try a uid the image does not know.
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
  # GUI_UID / GUI_GID force the user of the container, to try a runner whose user is not in the image (CI: 1001).
  user="${GUI_UID:-${user%%:*}}:${GUI_GID:-${user##*:}}"
  mkdir -p "$HOME/.cargo/registry" target/gui-home target/gui-etc
  local -a opts=(--rm --user "$user" -v "$PWD:/work" -w /work)
  # dbus-run-session (and so the smoke test) refuses to start for a user missing from /etc/passwd, and the uid of a
  # CI runner (1001) is not in the image: give the container a passwd and a group with an entry for the user it runs as.
  if [[ ${user%%:*} != 0 ]]; then
    printf 'root:x:0:0:root:/root:/bin/bash\ngui:x:%s:%s:Galleria GUI:/work/target/gui-home:/bin/bash\n' \
      "${user%%:*}" "${user##*:}" > target/gui-etc/passwd
    printf 'root:x:0:\ngui:x:%s:\n' "${user##*:}" > target/gui-etc/group
    opts+=(-v "$PWD/target/gui-etc/passwd:/etc/passwd:ro" -v "$PWD/target/gui-etc/group:/etc/group:ro")
  fi
  opts+=(
    -v "${RUSTUP_HOME:-$HOME/.rustup}:/opt/rustup:ro"
    -v "$HOME/.cargo/bin:/opt/cargo/bin:ro" -v "$HOME/.cargo/registry:/cargo-registry"
    -e HOME=/work/target/gui-home -e RUSTUP_HOME=/opt/rustup -e CARGO_HOME=/work/target/gui-home/cargo
    -e CARGO_TARGET_DIR=/work/target/gui-image -e GUI_INSIDE_IMAGE=1
    -e PATH=/opt/cargo/bin:/opt/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin)
  [[ -z ${GUI_CONTAINER_NAME:-} ]] || opts+=(--name "$GUI_CONTAINER_NAME")
  # one test or one check at a time (lab/tests/gui-smoke.sh, lab/tests/gui-a11y.sh)
  [[ -z ${GALLERIA_SMOKE_FILTER:-} ]] || opts+=(-e "GALLERIA_SMOKE_FILTER=$GALLERIA_SMOKE_FILTER")
  [[ -z ${GALLERIA_A11Y_FILTER:-} ]] || opts+=(-e "GALLERIA_A11Y_FILTER=$GALLERIA_A11Y_FILTER")
  docker run "${opts[@]}" "$IMAGE" bash -euc \
    'mkdir -p "$CARGO_HOME"; ln -sfn /cargo-registry "$CARGO_HOME/registry"; exec "$@"' gui-run "$@"
}

check_here() {
  pnpm install --frozen-lockfile
  scripts/check-js-licenses.mjs
  scripts/tests/check-js-licenses.sh
  mkdir -p target/gates
  # the output of the tests is kept for scripts/check-gates.py: a proof that was skipped must not pass as green (plan B5)
  pnpm --dir ui test --reporter=default --reporter=json --outputFile.json=../target/gates/gui-vitest.json
  pnpm --dir ui typecheck
  pnpm --dir ui build
  cargo clippy -p galleria-gui --all-targets -- -D warnings
  cargo test -p galleria-gui 2>&1 | tee target/gates/gui-cargo.log
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
smoke_here() { run_webdriver_here lab/tests/gui-smoke.sh; }

# Screen reader check (Q22): the same app and fake daemon, under Orca and the AT-SPI bus. Local only, not a CI job.
a11y_here() { run_webdriver_here lab/tests/gui-a11y.sh; }

# G3 in the laboratory (plan B4, T6): the real window imports the profiles of the laboratory, see lab/tests/gui-lab-import.sh.
lab_import_here() { run_webdriver_here lab/tests/gui-lab-import.sh; }

run_webdriver_here() {
  local target=${CARGO_TARGET_DIR:-$PWD/target}
  build_here
  cargo build -p galleria-gui --example fake-daemon --example seed-rubrica
  # the `galleria` binary of the phase 1: the smoke test asks it to list what the window saved (R1, R6)
  cargo build -p galleria-cli --bin galleria
  [[ -x target/gui-tools/bin/tauri-driver ]] || cargo install tauri-driver --locked --root target/gui-tools
  GALLERIA_SMOKE_APP="$target/release/galleria-gui" GALLERIA_SMOKE_DAEMON="$target/debug/examples/fake-daemon" \
    GALLERIA_SMOKE_SEED="$target/debug/examples/seed-rubrica" GALLERIA_SMOKE_CLI="$target/debug/galleria" \
    GALLERIA_SMOKE_DRIVER="$PWD/target/gui-tools/bin/tauri-driver" "$1"
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
  a11y) run_in_image scripts/gui.sh a11y-here ;;
  a11y-here) a11y_here ;;
  lab-import)
    # The profiles of the laboratory are made in containers, root-owned and 0600. With rootless Docker container root is the
    # invoking user and reads them; on a runner with plain Docker (CI, uid 1001) only container root does.
    if [[ -r lab/out/base.ovpn ]]; then run_in_image scripts/gui.sh lab-import-here
    else GUI_UID=0 GUI_GID=0 run_in_image scripts/gui.sh lab-import-here; fi ;;
  lab-import-here) lab_import_here ;;
  socket-owner)
    # Built as the invoking user like every other build; the run is root of the container (setpriv to two other uids).
    run_in_image scripts/gui.sh build-here
    GUI_UID=0 GUI_GID=0 run_in_image scripts/gui.sh socket-owner-here ;;
  socket-owner-here) GALLERIA_SMOKE_APP="${CARGO_TARGET_DIR:-$PWD/target}/release/galleria-gui" exec lab/tests/gui-socket-owner.sh ;;
  keyring) run_in_image scripts/gui.sh keyring-here ;;
  keyring-here) exec lab/tests/gui-keyring.sh ;;
  lab-g3)
    # The WireGuard keys of the laboratory are new at every `lab/up.sh`, so the window imports after the laboratory is up and
    # before the tests: lab/run.sh runs LAB_BEFORE_TESTS at that point. Host side: it needs docker, the lab images and the image of the GUI.
    [[ ${2:-} =~ ^(26|27|79)$ ]] || { echo 'usage: scripts/gui.sh lab-g3 <26|27|79>' >&2; exit 2; }
    ensure_image
    GALLERIA_E2E_GUI_STORE=1 LAB_BEFORE_TESTS='scripts/gui.sh lab-import' exec lab/run.sh "$2" t17 ;;
  pnpm-version) pnpm_version ;;
  *) echo 'usage: scripts/gui.sh <image|run CMD...|check|check-here|build|smoke|a11y|lab-import|lab-g3 <26|27|79>|socket-owner|keyring|pnpm-version>' >&2; exit 2 ;;
esac
