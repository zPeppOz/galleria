#!/usr/bin/env bash
# WebDriver smoke test of the GUI (level 3, spec §14): the real app in a real WebKitGTK window, driven by
# tauri-driver + WebKitWebDriver (both outside the app, Q13), against a daemon that serves the real IPC.
#
# Runs where WebKitGTK, Xvfb, dbus, xclip, tauri-driver and Node are installed: the galleria-gui-dev image
# (scripts/gui.sh smoke) and the `gui-smoke` CI job. Needs these variables:
#   GALLERIA_SMOKE_APP      the built galleria-gui binary
#   GALLERIA_SMOKE_DAEMON   the built fake-daemon example of galleria-gui (a real `galleriad` server with a fake backend)
#   GALLERIA_SMOKE_DRIVER   the tauri-driver binary (default: tauri-driver from PATH)
#
# It creates a temporary directory with the daemon socket, runs `fake-daemon <dir>` (S5: the real `galleriad run` cleans
# the network state at startup, `ip rule del …`, so the smoke test cannot use it)
# and points the app at it with GALLERIA_SOCKET. No test asks the daemon to connect, so no route, DNS or interface
# of this machine is touched. On exit, always: stop the daemon and the driver, delete the directory.
set -euo pipefail

if [[ ${1:-} != --inner ]]; then
  : "${GALLERIA_SMOKE_APP:?path of the galleria-gui binary}" "${GALLERIA_SMOKE_DAEMON:?path of the fake-daemon binary}"
  repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
  tmp=$(mktemp -d)
  bin=$tmp/bin
  mkdir -p "$bin"

  # The tests run these two from Node: the daemon is part of the scenario, not of the webview.
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

  cleanup() {
    status=$?
    "$bin/daemon-stop" || true
    if (( status != 0 )) && [[ -s $tmp/daemon.log ]]; then echo "--- galleriad log ---" >&2; cat "$tmp/daemon.log" >&2; fi
    rm -rf -- "$tmp"
    exit "$status"
  }
  trap cleanup EXIT

  export GALLERIA_SOCKET=$tmp/galleriad.sock GALLERIA_SMOKE_BIN=$bin GALLERIA_SMOKE_REPO=$repo
  # A session bus and a virtual screen of our own; the host display is never used.
  dbus-run-session -- xvfb-run -a --server-args='-screen 0 1280x800x24' bash "$0" --inner
  exit
fi

# Inner half: runs under the private bus and Xvfb.
driver_port=${GALLERIA_SMOKE_DRIVER_PORT:-4444}
# WebKitGTK's bubblewrap sandbox cannot start inside an unprivileged container or on a runner with restricted user
# namespaces; the window only loads our own ui/dist, so the sandbox is switched off for the test only. The DMA-BUF
# renderer needs a GPU that Xvfb does not have.
export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 WEBKIT_DISABLE_DMABUF_RENDERER=1
export GALLERIA_SMOKE_DRIVER_PORT=$driver_port GALLERIA_SMOKE_APP

"${GALLERIA_SMOKE_DRIVER:-tauri-driver}" --port "$driver_port" --native-driver "$(command -v WebKitWebDriver)" &
driver_pid=$!
trap 'kill "$driver_pid" 2>/dev/null || true; wait "$driver_pid" 2>/dev/null || true' EXIT
for _ in $(seq 1 50); do
  (exec 3<>"/dev/tcp/127.0.0.1/$driver_port") 2>/dev/null && break
  kill -0 "$driver_pid" 2>/dev/null || { echo "tauri-driver exited" >&2; exit 1; }
  sleep 0.1
done

cd "$GALLERIA_SMOKE_REPO"
pnpm --dir ui test:e2e
