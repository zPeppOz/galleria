#!/bin/bash
# Exercise the repair branch with controlled proc reads and a fake Docker boundary.
set -euo pipefail
repo=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$repo/target/local/tmp"
tmp=$(mktemp -d "$repo/target/local/tmp/wsl-check.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/bin"
cat > "$tmp/bin/grep" <<'GREP'
#!/bin/bash
args=()
for arg in "$@"; do
  case "$arg" in
    /proc/sys/fs/binfmt_misc/WSLInterop) args+=("$WSL_TEST_DIR/interop");;
    /proc/sys/kernel/osrelease) args+=("$WSL_TEST_DIR/osrelease");;
    *) args+=("$arg");;
  esac
done
exec /usr/bin/grep "${args[@]}"
GREP
cat > "$tmp/bin/docker" <<'DOCKER'
#!/bin/bash
printf '%s\n' called > "$WSL_TEST_DIR/docker-called"
printf '%s\n' enabled > "$WSL_TEST_DIR/interop"
DOCKER
chmod +x "$tmp/bin/grep" "$tmp/bin/docker"
printf '%s\n' disabled > "$tmp/interop"
printf '%s\n' '6.18.0-microsoft-standard-WSL2' > "$tmp/osrelease"
PATH="$tmp/bin:$PATH" WSL_TEST_DIR="$tmp" bash "$repo/lab/check-wsl.sh"
[[ -f "$tmp/docker-called" ]] || { echo 'WSL repair was skipped on a WSL kernel' >&2; exit 1; }
rm "$tmp/docker-called"
# An absent registration must also be repaired; enabled interop must be left alone.
rm "$tmp/interop"
PATH="$tmp/bin:$PATH" WSL_TEST_DIR="$tmp" bash "$repo/lab/check-wsl.sh"
[[ -f "$tmp/docker-called" && -f "$tmp/interop" ]] || { echo 'absent interop was not repaired' >&2; exit 1; }
rm "$tmp/docker-called"
PATH="$tmp/bin:$PATH" WSL_TEST_DIR="$tmp" bash "$repo/lab/check-wsl.sh"
[[ ! -e "$tmp/docker-called" ]] || { echo 'enabled interop was unnecessarily repaired' >&2; exit 1; }
printf '%s\n' disabled > "$tmp/interop"
printf '%s\n' '6.18.0-generic' > "$tmp/osrelease"
PATH="$tmp/bin:$PATH" WSL_TEST_DIR="$tmp" bash "$repo/lab/check-wsl.sh"
[[ ! -e "$tmp/docker-called" ]] || { echo 'non-WSL host must not trigger repair' >&2; exit 1; }
echo 'WSL repair detection tests passed'
