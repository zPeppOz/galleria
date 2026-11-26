#!/usr/bin/env bash
# G13 / Q57(a): three cold starts INSIDE a desktop VM, never drop the host's caches.
# Requires the built app installed with gui-vm.sh, a running graphical session and no Galleria window/process.
# Results stay in /mnt/b/vm/evidence/b6/<vm>/startup-<tag>/ (including failed runs). No network: updater points to closed loopback.
# Four marks: main (0), setup_done, window_shown, page_first_call. Clock starts on entry to run(), before Tauri is built.
# This does not include the dynamic loader before run(), nor assert a painted/usable frame; page_first_call is the saved-state request
# before React mounts. Report the boundary, not a claim about visible paint. G13 is non-blocking.
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
vm=${1:?usage: startup-time.sh <ubuntu24|ubuntu|fedora> [tag]}; tag=${2:-$(date +%Y%m%d-%H%M%S)}
[[ $tag =~ ^[a-zA-Z0-9_-]+$ ]] || { echo 'bad tag' >&2; exit 2; }
case $vm in ubuntu24|ubuntu|fedora) ;; *) exit 2;; esac
evidence=/mnt/b/vm/evidence/b6/$vm/startup-$tag
mkdir -p "$evidence"
set +e
"$here/gui-vm.sh" "$vm" sh "bash -s -- '$tag'" > "$evidence/run.txt" 2>&1 <<'GUEST'
set -euo pipefail
export XDG_RUNTIME_DIR=/run/user/$(id -u) DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus
export WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland
export GALLERIA_SOCKET=$HOME/spike/run/galleriad.sock XDG_CONFIG_HOME=$HOME/spike/cfg
export GALLERIA_UPDATE_URL=http://127.0.0.1:9/releases GALLERIA_STARTUP_TRACE=1
if pgrep -x galleria-gui >/dev/null; then echo 'Quit Galleria before measuring (including a hidden app)' >&2; exit 2; fi
out=$HOME/spike/startup-$1; mkdir -p "$out"
uname -a; lscpu | grep -E '^CPU\(s\)|^Model name'; free -m | head -2
cleanup() { if [[ -n ${pid:-} ]]; then kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; fi; }
trap cleanup EXIT
for n in 1 2 3; do
  sudo sh -c 'sync; echo 3 > /proc/sys/vm/drop_caches'
  /usr/local/bin/galleria-gui > "$out/$n.log" 2>&1 & pid=$!
  for _ in $(seq 200); do
    grep -q '^galleria-startup: page_first_call ' "$out/$n.log" && break
    kill -0 "$pid" 2>/dev/null || break
    sleep .1
  done
  echo "--- cold start $n (guest caches dropped, $(date -Is))"
  grep '^galleria-startup:' "$out/$n.log" || true
  for mark in main setup_done window_shown page_first_call; do
    count=$(grep -c "^galleria-startup: $mark " "$out/$n.log" || true)
    if [[ $count != 1 ]]; then echo "FAILED: $mark occurred $count times (expected 1)"; exit 1; fi
  done
  cleanup; pid=''; sleep 1
done
GUEST
rc=$?
set -e
# The guest logs are kept even if measuring failed; copy them via SSH, not into the repo.
"$here/gui-vm.sh" "$vm" sh "for f in ~/spike/startup-$tag/*.log; do echo \"--- \$f\"; cat \"\$f\"; done" > "$evidence/stderr.txt" 2>&1 || true
cat "$evidence/run.txt"
echo "startup evidence: $evidence (exit $rc)"
exit "$rc"
