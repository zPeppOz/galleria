#!/usr/bin/env bash
# G3 in the laboratory, the half that runs in the `galleria-gui-dev` image (plan B4, T6, Q37 (a); scripts/gui.sh lab-import).
#
# The real app, in a real WebKitGTK window under Xvfb, imports the profiles of the laboratory (`lab/out`: `base.ovpn`,
# `malicious.ovpn`, `wg-split.conf`) the way a file manager would hand them over, and leaves the folder of profiles it wrote in
# `target/gui-lab/config/galleria`. The clients take that folder (crates/galleria-e2e/tests/t17_gui_import.rs, run by
# `GALLERIA_E2E_GUI_STORE=1 lab/run.sh <26|27|79> t17`) and the real CLI and daemon list and connect what is in it. The
# WireGuard keys of `wg-split.conf` are new each time the laboratory comes up, so `scripts/gui.sh lab-g3` runs this step
# between `lab/up.sh` and the tests, through LAB_BEFORE_TESTS of lab/run.sh.
#
# Needs the variables of lab/tests/gui-smoke.sh (GALLERIA_SMOKE_APP, GALLERIA_SMOKE_DAEMON, GALLERIA_SMOKE_SEED,
# GALLERIA_SMOKE_DRIVER). The laboratory is not started here: if `lab/out` has no profiles it says which step makes them.
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
# shellcheck source=lab/tests/gui-lib.sh
source "$here/gui-lib.sh"

if [[ ${1:-} != --inner ]]; then
  : "${GALLERIA_SMOKE_APP:?path of the galleria-gui binary}"
  lab_out=$(cd -- "$here/../.." && pwd)/lab/out
  missing=()
  for name in base.ovpn malicious.ovpn wg-split.conf; do [[ -s $lab_out/$name ]] || missing+=("$name"); done
  if (( ${#missing[@]} )); then
    {
      echo "lab-import: missing in $lab_out: ${missing[*]}"
      echo "  the .ovpn files are written by lab/gen-pki.sh (lab/server/make-profiles.py, in a container);"
      echo "  wg-split.conf by the WireGuard server of the laboratory when lab/up.sh starts it (lab/wg/entrypoint.sh)."
      echo "  Run \`lab/gen-pki.sh && CLIENTS= lab/up.sh\`, or \`scripts/gui.sh lab-g3 <26|27|79>\`, which does it all."
    } >&2
    exit 1
  fi
  gui_prepare_outer
  export GALLERIA_LAB_OUT=$lab_out GALLERIA_LAB_STORE=$repo/target/gui-lab/config/galleria GALLERIA_LAB_IMPORT=1
  dbus-run-session -- xvfb-run -a --server-args='-screen 0 1280x800x24' bash "$0" --inner
  exit
fi

# Inner half: runs under the private bus and Xvfb.
export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 WEBKIT_DISABLE_DMABUF_RENDERER=1
# G3: the profiles go to the file, which the client of the laboratory reads: the window runs as a session without a keyring.
export GALLERIA_SMOKE_APP=$GALLERIA_SMOKE_APP_NO_KEYRING
gui_start_driver

cd "$GALLERIA_SMOKE_REPO"
pnpm --dir ui test:e2e
