#!/usr/bin/env bash
# Many rounds of the WebDriver smoke test at the same time, to catch a failure that comes once in a while (the «stale element reference»
# of a click that finds its element just before React renders it again). Builds nothing: it runs lab/tests/gui-smoke.sh on the binaries
# that `scripts/gui.sh smoke` (or `scripts/gui.sh build`) left in target/gui-image, in `jobs` containers at once, each with its own Xvfb,
# bus and driver, `rounds` times each. GALLERIA_SMOKE_FILTER (a vitest -t pattern) is passed through.
#
#   lab/tests/smoke-parallel.sh <tag> <jobs> <rounds>
#
# The output of job J, round N is in target/smoke-rounds/<tag>/j<J>-r<N>/run.log, the one-line outcomes in
# target/smoke-rounds/<tag>/summary.tsv: job-round, exit status, passed, failed, skipped, how many times «stale element» came out, the
# first red test.
set -uo pipefail
cd -- "$(dirname -- "$0")/../.."
tag=${1:?usage: lab/tests/smoke-parallel.sh <tag> <jobs> <rounds>}
jobs=${2:?jobs}
rounds=${3:?rounds}
base=target/smoke-rounds/$tag
mkdir -p "$base"
: > "$base/summary.tsv"
t=/work/target/gui-image

one() {
  local job=$1 round=$2 dir=$base/j$1-r$2
  mkdir -p "$dir"
  GUI_CONTAINER_NAME="smoke-$tag-j$job-r$round" scripts/gui.sh run env \
    GALLERIA_SMOKE_APP=$t/release/galleria-gui GALLERIA_SMOKE_DAEMON=$t/debug/examples/fake-daemon \
    GALLERIA_SMOKE_SEED=$t/debug/examples/seed-rubrica GALLERIA_SMOKE_CLI=$t/debug/galleria \
    GALLERIA_SMOKE_DRIVER=/work/target/gui-tools/bin/tauri-driver ${GALLERIA_SMOKE_FILTER:+GALLERIA_SMOKE_FILTER=$GALLERIA_SMOKE_FILTER} \
    lab/tests/gui-smoke.sh > "$dir/run.log" 2>&1
  local status=$?
  sed 's/\x1b\[[0-9;]*m//g' "$dir/run.log" > "$dir/run.txt"
  local line passed failed skipped first stale
  line=$(grep -E '^ +Tests +' "$dir/run.txt" | tail -1)
  passed=$(sed -n 's/.*\b\([0-9]\+\) passed.*/\1/p' <<< "$line"); failed=$(sed -n 's/.*\b\([0-9]\+\) failed.*/\1/p' <<< "$line"); skipped=$(sed -n 's/.*\b\([0-9]\+\) skipped.*/\1/p' <<< "$line")
  first=$(grep -m1 -E '^ +× ' "$dir/run.txt" | sed 's/^ *× //')
  stale=$(grep -ci 'stale element' "$dir/run.txt")
  printf 'j%s-r%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$job" "$round" "$status" "${passed:-0}" "${failed:-0}" "${skipped:-0}" "$stale" "${first:--}" >> "$base/summary.tsv"
}

for job in $(seq 1 "$jobs"); do
  ( for round in $(seq 1 "$rounds"); do one "$job" "$round"; done ) &
done
wait
sort -V "$base/summary.tsv" -o "$base/summary.tsv"
cat "$base/summary.tsv"
echo "green: $(awk -F'\t' '$2 == 0' "$base/summary.tsv" | wc -l) of $(wc -l < "$base/summary.tsv"); rounds that said «stale element»: $(awk -F'\t' '$7 > 0' "$base/summary.tsv" | wc -l)"
