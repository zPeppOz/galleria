#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
client=${1:-26}
filter=${2:-}
case "$client" in 26|27|79) ;; *) echo 'usage: lab/run.sh <26|27|79> [test-file-or-test-filter]' >&2; exit 2;; esac
cleanup() {
  result=$?
  trap - EXIT INT TERM
  if [[ ${KEEP:-0} != 1 ]]; then lab/down.sh || result=1; fi
  lab/check-wsl.sh || result=1
  exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
lab/build.sh "$client"
lab/gen-pki.sh
CLIENTS="$client" lab/up.sh
# A step that needs the laboratory up before the tests (G3: the window imports the profiles, scripts/gui.sh lab-g3).
if [[ -n ${LAB_BEFORE_TESTS:-} ]]; then bash -euc "$LAB_BEFORE_TESTS"; fi
matched=0
for executable in lab/out/tests/*; do
  name=$(basename "$executable")
  # t17 needs the profiles the window imported (scripts/gui.sh lab-g3). Run without them it would pass by skipping itself.
  if [[ $name == t17_gui_import && ${GALLERIA_E2E_GUI_STORE:-0} != 1 ]]; then
    echo "lab/run.sh: $name not run: it needs GALLERIA_E2E_GUI_STORE=1 (scripts/gui.sh lab-g3 $client)"
    continue
  fi
  args=()
  if [[ -n $filter && $name != *"$filter"* ]]; then args+=("$filter"); fi
  # Count real matches so a misspelled filter cannot produce a false green run.
  listed=$(docker exec -e GALLERIA_LAB=1 -e "GALLERIA_E2E_DCO=${GALLERIA_E2E_DCO:-0}" -e "GALLERIA_E2E_GUI_STORE=${GALLERIA_E2E_GUI_STORE:-0}" "glab-client$client" "/lab/$executable" --ignored --list "${args[@]}")
  if [[ $listed != *': test'* ]]; then continue; fi
  matched=$((matched+1))
  docker exec -e GALLERIA_LAB=1 -e "GALLERIA_E2E_DCO=${GALLERIA_E2E_DCO:-0}" -e "GALLERIA_E2E_GUI_STORE=${GALLERIA_E2E_GUI_STORE:-0}" "glab-client$client" "/lab/$executable" --ignored --test-threads=1 --nocapture "${args[@]}"
done
((matched>0)) || { echo 'no end-to-end tests matched' >&2; exit 1; }
