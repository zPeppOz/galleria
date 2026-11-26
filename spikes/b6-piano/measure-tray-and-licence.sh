#!/bin/bash
# Piano B6, seconda passata: tray di Tauri e la licenza rifiutata con rustls. Copia dei manifest, non il worktree.
set -uo pipefail
src=$1
S=$(mktemp -d "$TMPDIR/b6-spike.XXXX")
git -C "$src" archive HEAD Cargo.toml Cargo.lock deny.toml rust-toolchain.toml crates | tar -x -C "$S"
cd "$S" || exit 1
export CARGO_TARGET_DIR=$S/target
base=$(grep -c '^name = ' Cargo.lock)
cp Cargo.lock Cargo.lock.orig
cp crates/galleria-gui/Cargo.toml crates/galleria-gui/Cargo.toml.orig
names() { cargo tree -p galleria-gui -e normal --prefix none 2>/dev/null | sed 's/ (.*//' | sort -u; }
names > base.names
echo "base: lock=$base tree=$(wc -l < base.names) deny: $(cargo deny check 2>&1 | tail -1)"
try() {
  local label=$1; shift
  cp Cargo.lock.orig Cargo.lock
  cp crates/galleria-gui/Cargo.toml.orig crates/galleria-gui/Cargo.toml
  if ! "$@" >/dev/null 2>"$S/err.txt"; then echo "$label: FAILED $(tail -2 "$S/err.txt")"; return; fi
  local n; n=$(grep -c '^name = ' Cargo.lock)
  names > now.names
  echo "$label: lock +$((n-base)), tree=$(wc -l < now.names), deny: $(cargo deny check 2>&1 | tail -1)"
  echo "  new in tree: $(comm -13 base.names now.names | tr '\n' ' ')"
  cargo deny check licenses 2>&1 | grep -E 'rejected|license =|├ |= .* v[0-9]' | head -8
}
try "tauri tray-icon" cargo add -p galleria-gui tauri -F tray-icon
try "tauri tray-icon+image-png" cargo add -p galleria-gui tauri -F tray-icon,image-png
try "ureq3 rustls" cargo add -p galleria-gui ureq@3
try "soup3" cargo add -p galleria-gui soup3@0.5
