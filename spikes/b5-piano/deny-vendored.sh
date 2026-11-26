#!/usr/bin/env bash
# cargo deny and the feature unification with tao's dbus, for dbus-secret-service with the vendored libdbus.
set -uo pipefail
cd /home/g7aro/dev/galleria/.worktrees/t_72ccf7b0
( cd crates/galleria-profile && cargo add dbus-secret-service@4.2.0 --features vendored ) >/dev/null 2>&1
echo "crates in Cargo.lock: $(grep -c '^name = ' Cargo.lock)"
git diff --stat -- Cargo.lock | tail -1
cargo deny check 2>&1 | tail -1
echo '--- who brings libdbus-sys into galleria-gui, and with which features'
cargo tree -p galleria-gui -e normal,features -i libdbus-sys 2>&1 | head -12
git checkout -q -- Cargo.lock crates/galleria-profile/Cargo.toml
git status --short
echo "worktree clean: $?"
