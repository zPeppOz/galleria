#!/usr/bin/env bash
# Measures the stability of `scripts/gui.sh a11y`: N rounds one after the other, each in its own container, and for each
# round the count of the checks (passed / failed / skipped), the first check that went red and the logs, kept in
# target/a11y-rounds/<tag>/round<N>/ (the output of the run, target/gui-a11y/ with the log and the speech of Orca, and
# the container's own copy of every process log the run left in target/gui-a11y/).
#
#   lab/tests/a11y-rounds.sh <tag> [rounds]      default 10 rounds; GALLERIA_A11Y_FILTER is passed through
#
# Prints one line per round and a summary; exits 0 only when every round was green.
set -uo pipefail
cd -- "$(dirname -- "$0")/../.."
tag=${1:?usage: lab/tests/a11y-rounds.sh <tag> [rounds]}
rounds=${2:-10}
base=target/a11y-rounds/$tag
mkdir -p "$base"
: > "$base/summary.tsv"
green=0
for n in $(seq 1 "$rounds"); do
  dir=$base/round$n
  rm -rf -- "$dir"
  mkdir -p "$dir"
  start=$(date +%s)
  GUI_CONTAINER_NAME="a11y-$tag-$n" scripts/gui.sh a11y > "$dir/run.log" 2>&1
  status=$?
  secs=$(( $(date +%s) - start ))
  cp -a target/gui-a11y/. "$dir/gui-a11y/" 2>/dev/null || true
  # the vitest summary line: «Tests  1 failed | 46 passed (47)»
  sed 's/\x1b\[[0-9;]*m//g' "$dir/run.log" > "$dir/run.txt"
  line=$(grep -E '^ +Tests +' "$dir/run.txt" | tail -1)
  passed=$(sed -n 's/.*\b\([0-9]\+\) passed.*/\1/p' <<< "$line"); passed=${passed:-0}
  failed=$(sed -n 's/.*\b\([0-9]\+\) failed.*/\1/p' <<< "$line"); failed=${failed:-0}
  skipped=$(sed -n 's/.*\b\([0-9]\+\) skipped.*/\1/p' <<< "$line"); skipped=${skipped:-0}
  first=$(grep -m1 -E '^ +× ' "$dir/run.txt" | sed 's/^ *× //')
  (( status != 0 )) || green=$(( green + 1 ))
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$n" "$status" "$passed" "$failed" "$skipped" "${first:--}" "$secs" | tee -a "$base/summary.tsv"
done
echo "green rounds: $green of $rounds ($base/summary.tsv: round, exit status, passed, failed, skipped, first red check, seconds)"
(( green == rounds ))
