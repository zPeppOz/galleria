#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
# Capture the actual build invocation; the fake Docker returns existing images.
tmp=$(mktemp -d "$PWD/target/local/build-user.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
cat > "$tmp/docker" <<'DOCKER'
#!/bin/bash
if [[ $1 == run ]]; then
  printf '%s\n' "$@" > "$CAPTURE"
  exit 42
fi
DOCKER
chmod +x "$tmp/docker"
export CAPTURE="$tmp/args"
PATH="$tmp:$PATH" LAB_BUILD=container bash lab/build.sh 26 >/dev/null 2>&1 && exit 1
python3 - "$CAPTURE" <<'PY'
import os, pathlib, sys
args=pathlib.Path(sys.argv[1]).read_text().splitlines()
assert '--user' in args, 'Docker build must run as invoking user'
assert args[args.index('--user')+1] == f'{os.getuid()}:{os.getgid()}'
assert 'HOME=/work/target/lab-cargo-home' in args, 'HOME must be writable'
PY
