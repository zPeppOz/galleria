#!/bin/bash
set -euo pipefail
if [[ -e /proc/sys/fs/binfmt_misc/WSLInterop ]] && grep -q enabled /proc/sys/fs/binfmt_misc/WSLInterop; then exit 0; fi
if ! grep -qi 'microsoft|wsl' /proc/sys/kernel/osrelease; then exit 0; fi
echo 'restoring WSLInterop after laboratory execution' >&2
# Explicitly authorized repair for WSL, scoped to an ephemeral Galleria container.
docker run --rm --name glab-wsl-repair --privileged ubuntu:24.04 sh -c "mount -t binfmt_misc none /proc/sys/fs/binfmt_misc 2>/dev/null; echo ':WSLInterop:M::MZ::/init:PF' > /proc/sys/fs/binfmt_misc/register"
grep -q enabled /proc/sys/fs/binfmt_misc/WSLInterop
