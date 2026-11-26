#!/usr/bin/env bash
# Does each candidate build on this host (no libdbus-1-dev here) and what does the binary link?
set -uo pipefail
cd /home/g7aro/dev/galleria/.worktrees/t_72ccf7b0
out=/home/g7aro/.hermes/profiles/nello/cache/scratch/b5/keyring
export CARGO_TARGET_DIR=/home/g7aro/.hermes/profiles/nello/cache/scratch/b5/target
try() {
  local label=$1; shift
  git checkout -q -- Cargo.lock crates/galleria-profile/Cargo.toml
  echo "=== $label: cargo add $*"
  ( cd crates/galleria-profile && cargo add "$@" ) > /dev/null 2>&1
  local start=$SECONDS
  cargo build -p galleria-cli > "$out/$label.build.txt" 2>&1; echo "cargo build -p galleria-cli exit: $? ($((SECONDS-start)) s)"
  grep -E '^error|pkg-config|libdbus' "$out/$label.build.txt" | head -6
  [[ -x $CARGO_TARGET_DIR/debug/galleria ]] && { ldd "$CARGO_TARGET_DIR/debug/galleria" | grep -E 'dbus|ssl|crypto' || echo 'ldd: no dbus/ssl library'; ls -la "$CARGO_TARGET_DIR/debug/galleria" | awk '{print $5}'; rm -f "$CARGO_TARGET_DIR/debug/galleria"; }
}
try dbus-plain dbus-secret-service@4.2.0
try dbus-vendored dbus-secret-service@4.2.0 --features vendored
try secret-service-tokio secret-service@6.0.0 --no-default-features --features rt-tokio,crypto-rust
git checkout -q -- Cargo.lock crates/galleria-profile/Cargo.toml
git status --short
