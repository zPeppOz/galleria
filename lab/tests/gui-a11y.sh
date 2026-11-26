#!/usr/bin/env bash
# Screen reader check of the GUI (spec §12, plan B2 T1/T7, Q22): the real app in a real WebKitGTK window under Orca,
# with the accessibility tree read over AT-SPI. Local only: it is not a CI job (Q22). Like the smoke test it is
# driven by tauri-driver + WebKitWebDriver, against the fake daemon; it needs the same variables as gui-smoke.sh
# (GALLERIA_SMOKE_APP, GALLERIA_SMOKE_DAEMON, GALLERIA_SMOKE_SEED, GALLERIA_SMOKE_DRIVER) and the packages of the galleria-gui-dev image
# (orca, at-spi2-core, speech-dispatcher, python3-gi, gir1.2-atspi-2.0, gsettings).
#
# Inside the private session bus and Xvfb it: turns toolkit accessibility on, starts the AT-SPI bus, starts Orca with
# a debug file (Orca writes every sentence it speaks there as a `SPEECH OUTPUT` line, so no audio device is needed),
# starts tauri-driver and runs `pnpm --dir ui test:a11y` (ui/e2e/*.a11y.ts). The tests read the Orca log through
# GALLERIA_A11Y_ORCA_LOG and the tree through lab/tests/atspi-dump.py (GALLERIA_A11Y_DUMP); the GTK «Save» window of «Export»
# and the one of «Import» that asks for a file, which are not part of the page, are driven through lab/tests/atspi-file-window.py (GALLERIA_A11Y_FILE_WINDOW). They write what they saw into
# GALLERIA_A11Y_OUT (default target/gui-a11y/): `transcript.txt` (speech and tree per check), `orca-speech.log` (every
# `SPEECH OUTPUT` line of the run) and `orca-debug.log` (Orca's own debug file).
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
# shellcheck source=lab/tests/gui-lib.sh
source "$here/gui-lib.sh"

if [[ ${1:-} != --inner ]]; then
  : "${GALLERIA_SMOKE_APP:?path of the galleria-gui binary}"
  gui_prepare_outer
  export GALLERIA_A11Y_OUT=${GALLERIA_A11Y_OUT:-$repo/target/gui-a11y}
  rm -rf -- "$GALLERIA_A11Y_OUT"
  mkdir -p "$GALLERIA_A11Y_OUT"
  dbus-run-session -- xvfb-run -a --server-args='-screen 0 1280x800x24' bash "$0" --inner
  exit
fi

# Inner half: runs under the private bus and Xvfb.
export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 WEBKIT_DISABLE_DMABUF_RENDERER=1
export GALLERIA_A11Y_ORCA_LOG=$GALLERIA_A11Y_OUT/orca-debug.log GALLERIA_A11Y_DUMP=$GALLERIA_SMOKE_REPO/lab/tests/atspi-dump.py
export GALLERIA_A11Y_FILE_WINDOW=$GALLERIA_SMOKE_REPO/lab/tests/atspi-file-window.py
export GALLERIA_A11Y_DRAG_SOURCE=$GALLERIA_SMOKE_REPO/lab/tests/gtk-drag-source.py

# The toolkits only talk to the AT-SPI bus when this setting is on, and the bus has to be up before Orca.
gsettings set org.gnome.desktop.interface toolkit-accessibility true
/usr/libexec/at-spi-bus-launcher --launch-immediately > "$GALLERIA_A11Y_OUT/at-spi-bus.log" 2>&1 &
bus_pid=$!
python3 "$GALLERIA_SMOKE_REPO/lab/tests/orca-line-buffered.py" --debug-file="$GALLERIA_A11Y_ORCA_LOG" > "$GALLERIA_A11Y_OUT/orca.out" 2>&1 &
orca_pid=$!
# What every process of the run was doing, once a second: the state, the CPU and what it waits on. If the driver or the
# window ever stops answering, this says which one and since when (processes.log).
(
  while sleep 1; do
    echo "== $(date +%H:%M:%S)"
    ps -eo pid,ppid,stat,pcpu,rss,wchan:18,args --sort=pid | grep -E 'WebKit|galleria-gui|tauri-driver|orca|at-spi|atspi|speech|python3|vitest|node' | grep -v grep | cut -c1-200
  done
) > "$GALLERIA_A11Y_OUT/processes.log" 2>&1 &
sampler_pid=$!
collect() {
  kill "$sampler_pid" 2>/dev/null || true
  grep 'SPEECH OUTPUT' "$GALLERIA_A11Y_ORCA_LOG" > "$GALLERIA_A11Y_OUT/orca-speech.log" 2>/dev/null || true
  kill "$orca_pid" "$bus_pid" 2>/dev/null || true
}
trap collect EXIT
# Orca prints «Screen reader on.» once it is ready to read.
for _ in $(seq 1 100); do
  grep -q 'SPEECH OUTPUT' "$GALLERIA_A11Y_ORCA_LOG" 2>/dev/null && break
  kill -0 "$orca_pid" 2>/dev/null || { echo "orca exited:" >&2; cat "$GALLERIA_A11Y_OUT/orca.out" >&2; exit 1; }
  sleep 0.1
done

gui_start_keyring
# One tauri-driver (and so one WebKitWebDriver) for every test is how a driver that died took the rest of the run with it
# (`UND_ERR_SOCKET`, then «connection refused» for every later test). Here every test starts its own pair, and the end of
# the test ends it (ui/e2e/support.ts, GALLERIA_DRIVER_PER_SESSION). The native driver goes through a wrapper that writes
# in webkitwebdriver.log how it ended.
export GALLERIA_DRIVER_PER_SESSION=1 GALLERIA_SMOKE_DRIVER=${GALLERIA_SMOKE_DRIVER:-tauri-driver} GALLERIA_NATIVE_DRIVER
GALLERIA_NATIVE_DRIVER=$(command -v WebKitWebDriver)
export GALLERIA_NATIVE_DRIVER_WRAPPER=$GALLERIA_SMOKE_REPO/lab/tests/native-driver-trace.py

cd "$GALLERIA_SMOKE_REPO"
# GALLERIA_A11Y_FILTER runs only the checks whose name matches (a vitest -t pattern), to work on one of them.
# GALLERIA_A11Y_REPEAT=N runs the whole check N times one after the other under the same bus, Xvfb and Orca (to look for what
# comes after a long time of sessions in one run); the exit status is the first one that was not 0.
status=0
for round in $(seq 1 "${GALLERIA_A11Y_REPEAT:-1}"); do
  (( ${GALLERIA_A11Y_REPEAT:-1} == 1 )) || echo "=== screen reader check, round $round of $GALLERIA_A11Y_REPEAT"
  pnpm --dir ui test:a11y ${GALLERIA_A11Y_FILTER:+-t "$GALLERIA_A11Y_FILTER"} || { (( status != 0 )) || status=$?; }
done
exit "$status"
