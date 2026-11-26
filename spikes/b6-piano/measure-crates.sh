#!/bin/bash
# Piano B6: costo dei crate candidati (tray di Tauri, client HTTP). Lavora su una copia dei manifest, non sul worktree.
set -uo pipefail
src=$1
S=$(mktemp -d "$TMPDIR/b6-spike.XXXX")
echo "copy: $S"
git -C "$src" archive HEAD Cargo.toml Cargo.lock deny.toml rust-toolchain.toml crates | tar -x -C "$S"
cd "$S" || exit 1
export CARGO_TARGET_DIR=$S/target
base=$(grep -c '^name = ' Cargo.lock)
cp Cargo.lock Cargo.lock.orig
cp crates/galleria-gui/Cargo.toml crates/galleria-gui/Cargo.toml.orig
tree() { cargo tree -p galleria-gui -e normal --prefix none 2>/dev/null | sort -u | wc -l; }
echo "base: lock=$base tree(normal, linux)=$(tree) deny: $(cargo deny check 2>&1 | tail -1)"
try() {
  local label=$1; shift
  cp Cargo.lock.orig Cargo.lock
  cp crates/galleria-gui/Cargo.toml.orig crates/galleria-gui/Cargo.toml
  if ! "$@" >/dev/null 2>"$S/err.txt"; then echo "$label: FAILED $(tail -2 "$S/err.txt")"; return; fi
  local n; n=$(grep -c '^name = ' Cargo.lock)
  echo "$label: lock +$((n-base)), tree(normal, linux)=$(tree), deny: $(cargo deny check 2>&1 | tail -1)"
  cargo deny check 2>&1 | grep -E '^(error|warning)\[' | sort | uniq -c | head -5
}
try "tauri tray-icon" cargo add -p galleria-gui tauri -F tray-icon
try "tauri tray-icon+image-png" cargo add -p galleria-gui tauri -F tray-icon,image-png
try "soup3 0.5" cargo add -p galleria-gui soup3@0.5
try "ureq3 rustls(default)" cargo add -p galleria-gui ureq@3
try "ureq3 native-tls" cargo add -p galleria-gui ureq@3 --no-default-features -F native-tls
try "reqwest0.13 rustls" cargo add -p galleria-gui reqwest@0.13 --no-default-features -F rustls
try "reqwest0.13 native-tls" cargo add -p galleria-gui reqwest@0.13 --no-default-features -F native-tls
try "semver 1" cargo add -p galleria-gui semver@1
