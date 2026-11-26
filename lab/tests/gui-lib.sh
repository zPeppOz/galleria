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
  # The single window of the app (plan B4, T5) keeps its lock and its socket in `$XDG_RUNTIME_DIR/galleria`: the run has a
  # runtime folder of its own, `0700` like the one of a session, so the tests never touch the one of whoever runs them.
  export XDG_RUNTIME_DIR=$tmp/run
  mkdir -m 700 "$XDG_RUNTIME_DIR"
  bin=$tmp/bin
  mkdir -p "$bin"

  cat > "$bin/daemon-start" <<SCRIPT
#!/usr/bin/env bash
set -euo pipefail
if [[ -f "$tmp/daemon.pid" ]] && kill -0 "\$(cat "$tmp/daemon.pid")" 2>/dev/null; then exit 0; fi
rm -f "$tmp/galleriad.sock"
# \`daemon-start <scenario>\`: the scripts of the fake backend, one for each connect (tests/support/scenarios.rs); none: it says nothing
scenario=()
[[ -z "\${1:-}" ]] || scenario=(--scenario "\$1")
"$GALLERIA_SMOKE_DAEMON" "$tmp" "\${scenario[@]}" \\
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

  # The app as a session without a keyring sees it: the session bus it is given is not there, so `keyring_present()` says no and
  # the window falls back to the file. The tests that compare the files of the window with the ones of the CLI, and the one that
  # looks for the notice of the fallback, open the app through it (`openApp(..., "none")`).
  printf '#!/usr/bin/env bash\nexport DBUS_SESSION_BUS_ADDRESS=unix:path=%s/no-bus\nexec "%s" "$@"\n' "$tmp" "${GALLERIA_SMOKE_APP:-/nonexistent}" > "$bin/app-no-keyring"
  chmod +x "$bin/app-no-keyring"

  # The app on a desktop without the library of the indicator (plan B6, T1, point 2 of the review): a shim that makes `dlopen` fail for the
  # two libraries of the tray, preloaded by a wrapper like the one above. The libraries stay in the image; the app does not find them.
  cat > "$tmp/no-indicator.c" <<'SHIM'
#define _GNU_SOURCE
#include <dlfcn.h>
#include <string.h>
void *dlopen(const char *file, int flags) {
  static void *(*real)(const char *, int);
  if (!real) real = (void *(*)(const char *, int))dlsym(RTLD_NEXT, "dlopen");
  if (file && strstr(file, "appindicator")) return NULL;
  return real(file, flags);
}
SHIM
  gcc -shared -fPIC -o "$tmp/no-indicator.so" "$tmp/no-indicator.c" -ldl
  printf '#!/usr/bin/env bash\nexport LD_PRELOAD=%s\nexec "%s" "$@" 2> %s\n' "$tmp/no-indicator.so" "${GALLERIA_SMOKE_APP:-/nonexistent}" "$tmp/no-indicator.stderr" > "$bin/app-no-indicator"
  chmod +x "$bin/app-no-indicator"

  # The version check (plan B6, T5): nothing in a run reaches the network. The whole run points the check at a closed port of the machine; the
  # tests of G11 start lab/tests/update-server.py and open the app through `app-update`, which reads the address of that server from a file at every
  # start (the driver that starts the app was started before the test, so the environment of the test cannot reach it).
  export GALLERIA_UPDATE_URL=http://127.0.0.1:9/zPeppOz/galleria/releases/latest
  printf '#!/usr/bin/env bash\nexport GALLERIA_UPDATE_URL=$(cat "%s/update-url" 2>/dev/null || echo http://127.0.0.1:9/zPeppOz/galleria/releases/latest)\nexec "%s" "$@"\n' "$bin" "${GALLERIA_SMOKE_APP:-/nonexistent}" > "$bin/app-update"
  chmod +x "$bin/app-update"

  # The app of the whole-day test (plan B6, T7): what it says on `stderr` and how it ends are kept in files, to be read when the day is over. The
  # wrapper is not an `exec`: it waits for the app and writes its exit status, and passes on the signals it gets (the driver ends a session by
  # killing what it started). A test that fails before the end kills the app by name (`pkill`), as `stopPrivateDriver` does.
  cat > "$bin/app-day" <<SCRIPT
#!/usr/bin/env bash
"${GALLERIA_SMOKE_APP:-/nonexistent}" "\$@" 2>> "$tmp/day.stderr" &
child=\$!
trap 'kill -TERM "\$child" 2>/dev/null' TERM INT HUP
wait "\$child"; status=\$?
while kill -0 "\$child" 2>/dev/null; do wait "\$child"; status=\$?; done
echo "\$status" > "$tmp/day.exit"
exit "\$status"
SCRIPT
  chmod +x "$bin/app-day"

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

  export GALLERIA_SOCKET=$tmp/galleriad.sock GALLERIA_SMOKE_BIN=$bin GALLERIA_SMOKE_REPO=$repo GALLERIA_SMOKE_APP_NO_KEYRING=$bin/app-no-keyring
  # The tray and the window seen from outside (plan B6): the probe that stands in for the desktop that shows the tray, and the one that
  # reads and closes the window like a window manager (ui/e2e/support.ts).
  export GALLERIA_SMOKE_APP_NO_INDICATOR=$bin/app-no-indicator GALLERIA_SMOKE_NO_INDICATOR_STDERR=$tmp/no-indicator.stderr GALLERIA_TRAY_PROBE=$repo/lab/tests/tray-probe.py GALLERIA_X11_WINDOW=$repo/lab/tests/x11-window.py
  export GALLERIA_SMOKE_APP_DAY=$bin/app-day GALLERIA_SMOKE_DAY_STDERR=$tmp/day.stderr GALLERIA_SMOKE_DAY_EXIT=$tmp/day.exit
  export GALLERIA_SMOKE_APP_UPDATE=$bin/app-update GALLERIA_SMOKE_UPDATE_URL_FILE=$bin/update-url GALLERIA_UPDATE_SERVER=$repo/lab/tests/update-server.py
}

# A keyring of the run's own for the window (plan B5, T3). GNOME Keyring is installed in the image and starts by itself on the
# session bus, but a keyring nobody has created asks for a password in a window on the virtual screen the first time a secret
# is written, and the test would wait for it. So the run starts one unlocked (a throw-away password, its data in the run's
# temporary folder, gone with it), as lab/tests/gui-keyring.sh does. Call it from the inner half, under the private bus.
# GALLERIA_SMOKE_KEYRING=absent leaves the keyring out (the window then falls back to the file).
gui_start_keyring() {
  [[ ${GALLERIA_SMOKE_KEYRING:-unlocked} == absent ]] && return 0
  export XDG_DATA_HOME=$XDG_RUNTIME_DIR/keyring-data
  mkdir -m 700 -p "$XDG_DATA_HOME"
  printf '%s' galleria-test-keyring | gnome-keyring-daemon --unlock --components=secrets > /dev/null 2>&1 \
    || { echo "gui_start_keyring: GNOME Keyring did not start" >&2; exit 1; }
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
