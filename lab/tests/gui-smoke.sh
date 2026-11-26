#!/usr/bin/env bash
# WebDriver smoke test of the GUI (level 3, spec §14): the real app in a real WebKitGTK window, driven by
# tauri-driver + WebKitWebDriver (both outside the app, Q13), against a daemon that serves the real IPC.
#
# Runs where WebKitGTK, Xvfb, dbus, xclip, tauri-driver and Node are installed: the galleria-gui-dev image
# (scripts/gui.sh smoke) and the `gui-smoke` CI job. Needs these variables:
#   GALLERIA_SMOKE_APP      the built galleria-gui binary
#   GALLERIA_SMOKE_DAEMON   the built fake-daemon example of galleria-gui (a real `galleriad` server with a fake backend)
#   GALLERIA_SMOKE_SEED     the built seed-rubrica example of galleria-gui (the made-up rubrica every test starts from)
#   GALLERIA_SMOKE_CLI      the built `galleria` binary of the phase 1 (the test that asks it what the window saved)
#   GALLERIA_SMOKE_DRIVER   the tauri-driver binary (default: tauri-driver from PATH)
#
# It creates a temporary directory with the daemon socket, runs `fake-daemon <dir>` (S5: the real `galleriad run` cleans
# the network state at startup, `ip rule del …`, so the smoke test cannot use it)
# and points the app at it with GALLERIA_SOCKET. No test asks the daemon to connect, so no route, DNS or interface
# of this machine is touched. On exit, always: stop the daemon and the driver, delete the directory.
# The temporary directory, the daemon scripts and the driver are set up by lab/tests/gui-lib.sh, shared with gui-a11y.sh.
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
# shellcheck source=lab/tests/gui-lib.sh
source "$here/gui-lib.sh"

if [[ ${1:-} != --inner ]]; then
  : "${GALLERIA_SMOKE_APP:?path of the galleria-gui binary}"
  gui_prepare_outer
  # A session bus and a virtual screen of our own; the host display is never used.
  dbus-run-session -- xvfb-run -a --server-args='-screen 0 1280x800x24' bash "$0" --inner
  exit
fi

# Inner half: runs under the private bus and Xvfb.
# WebKitGTK's bubblewrap sandbox cannot start inside an unprivileged container or on a runner with restricted user
# namespaces; the window only loads our own ui/dist, so the sandbox is switched off for the test only. The DMA-BUF
# renderer needs a GPU that Xvfb does not have.
export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 WEBKIT_DISABLE_DMABUF_RENDERER=1
gui_start_keyring
gui_start_driver

cd "$GALLERIA_SMOKE_REPO"
# The results of the run go to target/gates/smoke-vitest.json (the path is relative to ui/), where `scripts/check-gates.py --job gui-smoke`
# reads them: G10 and G11 are proved by tests of this run, and a test that is skipped counts as green in the runner (plan B6, T6). A file left
# by an earlier run must not stand in for this one.
mkdir -p target/gates
rm -f target/gates/smoke-vitest.json
# GALLERIA_SMOKE_FILTER runs only the tests whose name matches (a vitest -t pattern), to work on one test.
pnpm --dir ui test:e2e --reporter=default --reporter=json --outputFile.json=../target/gates/smoke-vitest.json ${GALLERIA_SMOKE_FILTER:+-t "$GALLERIA_SMOKE_FILTER"}
