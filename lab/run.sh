#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
client=${1:-26}
filter=${2:-}
case "$client" in 26|27|79) ;; *) echo 'usage: lab/run.sh <26|27|79> [test-file-or-test-filter]' >&2; exit 2;; esac
cleanup() {
  result=$?
  trap - EXIT INT TERM
  # A red run keeps what the containers say before they are removed (Q48): the register of the watch and the journal of the daemon
  # of the client, the logs of every `glab-*` container. Best effort: it never changes the result of the run.
  if ((result != 0)) && [[ ${KEEP:-0} != 1 ]]; then
    kept="target/lab-failures/$(date -u +%Y%m%dT%H%M%SZ)-$client"
    if mkdir -p "$kept" 2>/dev/null; then
      for container in $(docker ps -a --format '{{.Names}}' | awk '/^glab-/'); do
        docker logs "$container" > "$kept/$container.log" 2>&1 || true
      done
      docker exec "glab-client$client" journalctl -u galleriad --no-pager -o short-precise > "$kept/galleriad.journal" 2>&1 || true
      docker exec "glab-client$client" cat /tmp/gui-lab-watch.log > "$kept/gui-lab-watch.log" 2>&1 || true
      echo "lab/run.sh: the run failed; the logs of the containers are kept in $kept" >&2
    fi
  fi
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
  # t18 needs the window to drive the daemon of the client (scripts/gui.sh lab-connect): without it there was nothing to see.
  if [[ $name == t18_gui_connect && ${GALLERIA_E2E_GUI_CONNECT:-0} != 1 ]]; then
    echo "lab/run.sh: $name not run: it needs GALLERIA_E2E_GUI_CONNECT=1 (scripts/gui.sh lab-connect $client)"
    continue
  fi
  args=()
  if [[ -n $filter && $name != *"$filter"* ]]; then args+=("$filter"); fi
  # Count real matches so a misspelled filter cannot produce a false green run.
  listed=$(docker exec -e GALLERIA_LAB=1 -e "GALLERIA_E2E_DCO=${GALLERIA_E2E_DCO:-0}" -e "GALLERIA_E2E_GUI_STORE=${GALLERIA_E2E_GUI_STORE:-0}" -e "GALLERIA_E2E_GUI_CONNECT=${GALLERIA_E2E_GUI_CONNECT:-0}" "glab-client$client" "/lab/$executable" --ignored --list "${args[@]}")
  if [[ $listed != *': test'* ]]; then continue; fi
  matched=$((matched+1))
  # LAB_TEST_LOG keeps what the tests print, for scripts/check-gates.py: a proof that was ignored or never ran must not pass as green.
  docker exec -e GALLERIA_LAB=1 -e "GALLERIA_E2E_DCO=${GALLERIA_E2E_DCO:-0}" -e "GALLERIA_E2E_GUI_STORE=${GALLERIA_E2E_GUI_STORE:-0}" -e "GALLERIA_E2E_GUI_CONNECT=${GALLERIA_E2E_GUI_CONNECT:-0}" "glab-client$client" "/lab/$executable" --ignored --test-threads=1 --nocapture "${args[@]}" \
    | if [[ -n ${LAB_TEST_LOG:-} ]]; then tee -a "$LAB_TEST_LOG"; else cat; fi
done
((matched>0)) || { echo 'no end-to-end tests matched' >&2; exit 1; }
