#!/usr/bin/env bash
# G4, G5, G6, G7 and G9 in the laboratory, the half that runs in the `galleria-gui-dev` image (plan B5, T9 and T10, Q42 (b);
# scripts/gui.sh lab-connect-here).
#
# The real app, in a real WebKitGTK window under Xvfb, with a private bus and an unlocked GNOME Keyring, drives the REAL daemon of a
# client of the laboratory: GALLERIA_SOCKET is the bridge of `carla` (`scripts/gui.sh lab-connect-window` starts it on the host, and
# a second one for `alice`, GALLERIA_LAB_SOCKET_OTHER). The proofs are ui/e2e/lab-connect.e2e.ts; what the client saw while they ran
# is checked by crates/galleria-e2e/tests/t18_gui_connect.rs.
#
# Needs the variables of lab/tests/gui-smoke.sh (GALLERIA_SMOKE_APP, GALLERIA_SMOKE_DAEMON, GALLERIA_SMOKE_SEED, GALLERIA_SMOKE_CLI,
# GALLERIA_SMOKE_DRIVER) and GALLERIA_LAB_SOCKET, GALLERIA_LAB_SOCKET_OTHER. The laboratory is not started here.
#
# The stderr of the app goes to target/gui-lab/app.stderr (G7: it must hold nothing secret); the results of the proofs to
# target/gui-lab/vitest.json, which `scripts/check-gates.py --job lab-connect` reads. The file window «Save» of «Export log…» is
# driven over AT-SPI with the bus alone, without Orca (S41).
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
# shellcheck source=lab/tests/gui-lib.sh
source "$here/gui-lib.sh"

if [[ ${1:-} != --inner ]]; then
  : "${GALLERIA_SMOKE_APP:?path of the galleria-gui binary}"
  : "${GALLERIA_SMOKE_CLI:?path of the galleria binary}"
  : "${GALLERIA_LAB_SOCKET:?the bridge of carla (scripts/gui.sh lab-connect-window)}"
  : "${GALLERIA_LAB_SOCKET_OTHER:?the bridge of alice (scripts/gui.sh lab-connect-window)}"
  lab_out=$(cd -- "$here/../.." && pwd)/lab/out
  missing=()
  for name in base.ovpn otp.ovpn enckey.ovpn unreachable.ovpn wg-split.conf; do [[ -s $lab_out/$name ]] || missing+=("$name"); done
  if (( ${#missing[@]} )); then
    {
      echo "lab-connect: missing in $lab_out: ${missing[*]}"
      echo "  the .ovpn files are written by lab/gen-pki.sh, wg-split.conf by the WireGuard server of the laboratory when lab/up.sh starts it."
      echo "  Run \`scripts/gui.sh lab-connect <26|27|79>\`, which does it all."
    } >&2
    exit 1
  fi
  for socket in "$GALLERIA_LAB_SOCKET" "$GALLERIA_LAB_SOCKET_OTHER"; do
    [[ -S $socket ]] || { echo "lab-connect: no bridge on $socket: scripts/gui.sh lab-connect-window starts them" >&2; exit 1; }
  done
  gui_prepare_outer
  run_out=$repo/target/gui-lab
  mkdir -p "$run_out"
  rm -f "$run_out/app.stderr" "$run_out/vitest.json" "$run_out/used-otp" "$run_out/exported-log.txt"
  : > "$run_out/app.stderr"
  # The app is started by tauri-driver, so its stderr is the one of the driver: through this shim the stderr of the app and of
  # nothing else is kept in a file, to be searched for secrets.
  shim=$tmp/bin/app-keep-stderr
  printf '#!/usr/bin/env bash\nexec "%s" "$@" 2>> "%s/app.stderr"\n' "$GALLERIA_SMOKE_APP" "$run_out" > "$shim"
  chmod +x "$shim"
  export GALLERIA_SMOKE_APP=$shim
  # The window talks to the real daemon through the bridge, not to the fake one that gui_prepare_outer set up.
  export GALLERIA_SOCKET=$GALLERIA_LAB_SOCKET
  export GALLERIA_LAB_OUT=$lab_out GALLERIA_LAB_RUN_OUT=$run_out GALLERIA_LAB_USED_OTP=$run_out/used-otp GALLERIA_LAB_CONNECT=1
  dbus-run-session -- xvfb-run -a --server-args='-screen 0 1280x800x24' bash "$0" --inner
  exit
fi

# Inner half: runs under the private bus and Xvfb.
export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 WEBKIT_DISABLE_DMABUF_RENDERER=1
gui_start_keyring
# The window «Save» of Rust is read and answered over AT-SPI: the bus and the setting are enough, there is no Orca here.
export GALLERIA_A11Y_FILE_WINDOW=$GALLERIA_SMOKE_REPO/lab/tests/atspi-file-window.py
gsettings set org.gnome.desktop.interface toolkit-accessibility true
/usr/libexec/at-spi-bus-launcher --launch-immediately > "$GALLERIA_LAB_RUN_OUT/at-spi-bus.log" 2>&1 &
bus_pid=$!
gui_start_driver
trap 'kill "$driver_pid" "$bus_pid" 2>/dev/null || true; wait "$driver_pid" 2>/dev/null || true' EXIT

cd "$GALLERIA_SMOKE_REPO"
# the results go where scripts/check-gates.py reads them (the path is relative to ui/)
pnpm --dir ui test:e2e --reporter=default --reporter=json --outputFile.json=../target/gui-lab/vitest.json ${GALLERIA_SMOKE_FILTER:+-t "$GALLERIA_SMOKE_FILTER"}
