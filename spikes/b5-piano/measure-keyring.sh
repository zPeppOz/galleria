#!/usr/bin/env bash
# Measure what each Secret Service crate costs when added to galleria-profile: new crates in Cargo.lock and cargo deny.
# Leaves the worktree as it was (git checkout of the two files).
set -uo pipefail
cd /home/g7aro/dev/galleria/.worktrees/t_72ccf7b0
out=/home/g7aro/.hermes/profiles/nello/cache/scratch/b5/keyring
mkdir -p "$out"
base=$(grep -c '^name = ' Cargo.lock)
try() {
  local label=$1; shift
  git checkout -q -- Cargo.lock crates/galleria-profile/Cargo.toml
  grep '^name = ' Cargo.lock | sort > "$out/base.names"
  echo "=== $label: cargo add $*"
  ( cd crates/galleria-profile && cargo add "$@" ) > "$out/$label.add.txt" 2>&1 || { echo "cargo add failed"; tail -5 "$out/$label.add.txt"; return; }
  cargo metadata --format-version 1 > /dev/null 2> "$out/$label.meta.txt" || { echo "metadata failed"; tail -5 "$out/$label.meta.txt"; }
  local now; now=$(grep -c '^name = ' Cargo.lock)
  grep '^name = ' Cargo.lock | sort > "$out/$label.names"
  echo "crates in Cargo.lock: $base -> $now (+$((now-base)))"
  comm -13 "$out/base.names" "$out/$label.names" | tr '\n' ' '; echo
  cargo deny check > "$out/$label.deny.txt" 2>&1; echo "cargo deny exit: $?"
  grep -E '^(advisories|bans|licenses|sources) |RUSTSEC-|^error\[' "$out/$label.deny.txt" | sort | uniq -c | head -30
  cargo tree -p galleria-cli -e normal 2>/dev/null | grep -E ' (zbus|dbus|libdbus-sys|async-io|openssl|keyring[a-z-]*|oo7|secret-service|dbus-secret-service) ' | sed 's/^[ │├└─]*//' | sort -u | tr '\n' ';'; echo
}
try dbus-plain dbus-secret-service@4.2.0
try dbus-rustcrypto dbus-secret-service@4.2.0 --features crypto-rust
try secret-service-tokio secret-service@6.0.0 --no-default-features --features rt-tokio,crypto-rust
try oo7 oo7@0.7.3
try keyring-default keyring@4.2.0
try keyring-dbus keyring@4.2.0 --no-default-features --features dbus-secret-service-keyring-store
git checkout -q -- Cargo.lock crates/galleria-profile/Cargo.toml
git status --short
