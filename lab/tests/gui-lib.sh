#!/usr/bin/env bash
# Shared by lab/tests/gui-smoke.sh and lab/tests/gui-a11y.sh (source it, do not run it).
#
# gui_prepare_outer: creates a temporary directory with the fake daemon's socket, the `daemon-start` and
#   `daemon-stop` scripts the tests run from Node (the daemon is part of the scenario, not of the webview), a
#   `rubrica-seed` script that writes the made-up rubrica of the tests (the `seed-rubrica` example of galleria-gui, with
#   the profiles of the phase 1 import functions) into the temporary XDG_CONFIG_HOME, and a cleanup that stops the
#   daemon and deletes the directory. Needs GALLERIA_SMOKE_DAEMON and GALLERIA_SMOKE_SEED. Sets and exports
#   GALLERIA_SOCKET, GALLERIA_SMOKE_BIN and GALLERIA_SMOKE_REPO. Call it from the outer half of a script.
# gui_start_driver: starts tauri-driver (GALLERIA_SMOKE_DRIVER, default from PATH) on GALLERIA_SMOKE_DRIVER_PORT
#   (default 4444) and waits for the port; the driver is stopped on exit. Call it from the inner half.

gui_prepare_outer() {
  : "${GALLERIA_SMOKE_DAEMON:?path of the fake-daemon binary}"
  : "${GALLERIA_SMOKE_SEED:?path of the seed-rubrica binary}"
  repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
  tmp=$(mktemp -d)
  # The app reads and writes `state.toml` (D32) and, from B3, the profiles: both inside the temporary directory, never
  # in the home of whoever runs the test.
  export XDG_CONFIG_HOME=$tmp/config XDG_STATE_HOME=$tmp/state
  bin=$tmp/bin
  mkdir -p "$bin"

  cat > "$bin/daemon-start" <<SCRIPT
#!/usr/bin/env bash
set -euo pipefail
if [[ -f "$tmp/daemon.pid" ]] && kill -0 "\$(cat "$tmp/daemon.pid")" 2>/dev/null; then exit 0; fi
rm -f "$tmp/galleriad.sock"
"$GALLERIA_SMOKE_DAEMON" "$tmp" \\
  >> "$tmp/daemon.log" 2>&1 &
echo \$! > "$tmp/daemon.pid"
for _ in \$(seq 1 100); do
  [[ -S "$tmp/galleriad.sock" ]] && exit 0
  kill -0 "\$(cat "$tmp/daemon.pid")" 2>/dev/null || { echo "galleriad exited:" >&2; cat "$tmp/daemon.log" >&2; exit 1; }
  sleep 0.1
done
echo "galleriad did not create its socket" >&2; cat "$tmp/daemon.log" >&2; exit 1
SCRIPT
  cat > "$bin/daemon-stop" <<SCRIPT
#!/usr/bin/env bash
set -uo pipefail
if [[ -f "$tmp/daemon.pid" ]]; then
  pid=\$(cat "$tmp/daemon.pid")
  kill "\$pid" 2>/dev/null
  for _ in \$(seq 1 50); do kill -0 "\$pid" 2>/dev/null || break; sleep 0.1; done
  kill -9 "\$pid" 2>/dev/null
  rm -f "$tmp/daemon.pid"
fi
rm -f "$tmp/galleriad.sock"
exit 0
SCRIPT
  chmod +x "$bin/daemon-start" "$bin/daemon-stop"

  # The tests start every run from a rubrica they know: this wipes the profiles and writes the seed again (`--empty`:
  # the store only). It writes inside $XDG_CONFIG_HOME of the run and refuses to run without it.
  cat > "$bin/rubrica-seed" <<SCRIPT
#!/usr/bin/env bash
set -euo pipefail
rm -rf -- "$tmp/config/galleria"
mkdir -p "$tmp/config"
XDG_CONFIG_HOME="$tmp/config" exec "$GALLERIA_SMOKE_SEED" "\$@"
SCRIPT
  chmod +x "$bin/rubrica-seed"

  # The files an import is tried on (plan B4): written fresh into `$tmp/fixtures` every time, and printed as `name=path`.
  # They are outside the rubrica, so a run that wipes the profiles does not touch them.
  cat > "$bin/import-fixtures" <<SCRIPT
#!/usr/bin/env bash
set -euo pipefail
rm -rf -- "$tmp/fixtures"
exec "$GALLERIA_SMOKE_SEED" --fixtures "$tmp/fixtures"
SCRIPT
  chmod +x "$bin/import-fixtures"

  gui_cleanup() {
    status=$?
    "$bin/daemon-stop" || true
    if (( status != 0 )) && [[ -s $tmp/daemon.log ]]; then echo "--- galleriad log ---" >&2; cat "$tmp/daemon.log" >&2; fi
    rm -rf -- "$tmp"
    exit "$status"
  }
  trap gui_cleanup EXIT

  export GALLERIA_SOCKET=$tmp/galleriad.sock GALLERIA_SMOKE_BIN=$bin GALLERIA_SMOKE_REPO=$repo
}

gui_start_driver() {
  driver_port=${GALLERIA_SMOKE_DRIVER_PORT:-4444}
  export GALLERIA_SMOKE_DRIVER_PORT=$driver_port GALLERIA_SMOKE_APP
  "${GALLERIA_SMOKE_DRIVER:-tauri-driver}" --port "$driver_port" --native-driver "$(command -v WebKitWebDriver)" &
  driver_pid=$!
  trap 'kill "$driver_pid" 2>/dev/null || true; wait "$driver_pid" 2>/dev/null || true' EXIT
  for _ in $(seq 1 50); do
    (exec 3<>"/dev/tcp/127.0.0.1/$driver_port") 2>/dev/null && break
    kill -0 "$driver_pid" 2>/dev/null || { echo "tauri-driver exited" >&2; exit 1; }
    sleep 0.1
  done
}
