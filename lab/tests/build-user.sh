#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
# Capture the actual build invocation; the fake Docker returns existing images.
tmp=$(mktemp -d "$PWD/target/local/build-user.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
cat > "$tmp/docker" <<'DOCKER'
#!/bin/bash
if [[ $1 == info ]]; then
  printf '%s\n' "${FAKE_SECURITY_OPTIONS:-[name=seccomp,profile=builtin]}"
  exit 0
fi
if [[ $1 == run ]]; then
  printf '%s\n' "$@" > "$CAPTURE"
  exit 42
fi
DOCKER
chmod +x "$tmp/docker"
export CAPTURE="$tmp/args"
check() {
  PATH="$tmp:$PATH" LAB_BUILD=container bash lab/build.sh 26 >/dev/null 2>&1 && exit 1
  python3 - "$CAPTURE" "$1" <<'PY'
import pathlib, sys
args=pathlib.Path(sys.argv[1]).read_text().splitlines()
assert '--user' in args, 'Docker build must run as invoking user'
assert args[args.index('--user')+1] == sys.argv[2], args[args.index('--user')+1]
assert 'HOME=/work/target/lab-cargo-home' in args, 'HOME must be writable'
PY
}
check "$(id -u):$(id -g)"
# Rootless Docker maps container root to the invoking user, so uid 0 inside is the user.
FAKE_SECURITY_OPTIONS='[name=seccomp,profile=builtin name=rootless name=cgroupns]' check 0:0
