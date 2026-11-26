#!/usr/bin/env bash
# Many rounds of the screen reader check at the same time, to catch a failure that comes once in a while. Builds nothing:
# it runs lab/tests/gui-a11y.sh on the binaries that `scripts/gui.sh a11y` (or `scripts/gui.sh build`) left in
# target/gui-image, in `jobs` containers at once, each with its own Xvfb, bus, Orca and output folder, `rounds` times each.
#
#   lab/tests/a11y-parallel.sh <tag> <jobs> <rounds>
#
# The output of job J, round N is in target/a11y-rounds/<tag>/j<J>-r<N>/ (run.log and the files of the check), the
# one-line outcomes in target/a11y-rounds/<tag>/summary.tsv: job-round, exit status, passed, failed, skipped, first red check.
set -uo pipefail
cd -- "$(dirname -- "$0")/../.."
tag=${1:?usage: lab/tests/a11y-parallel.sh <tag> <jobs> <rounds>}
jobs=${2:?jobs}
rounds=${3:?rounds}
base=target/a11y-rounds/$tag
mkdir -p "$base"
: > "$base/summary.tsv"
t=/work/target/gui-image

one() {
  local job=$1 round=$2 dir=$base/j$1-r$2
  mkdir -p "$dir"
  GUI_CONTAINER_NAME="a11y-$tag-j$job-r$round" scripts/gui.sh run env \
    GALLERIA_SMOKE_APP=$t/release/galleria-gui GALLERIA_SMOKE_DAEMON=$t/debug/examples/fake-daemon \
    GALLERIA_SMOKE_SEED=$t/debug/examples/seed-rubrica GALLERIA_SMOKE_CLI=$t/debug/galleria \
    GALLERIA_SMOKE_DRIVER=/work/target/gui-tools/bin/tauri-driver GALLERIA_A11Y_OUT=/work/$dir/out GALLERIA_A11Y_REPEAT=${GALLERIA_A11Y_REPEAT:-1} ${GALLERIA_A11Y_FILTER:+GALLERIA_A11Y_FILTER=$GALLERIA_A11Y_FILTER} ${GALLERIA_NATIVE_DRIVER_DIE_AFTER:+GALLERIA_NATIVE_DRIVER_DIE_AFTER=$GALLERIA_NATIVE_DRIVER_DIE_AFTER} \
    lab/tests/gui-a11y.sh > "$dir/run.log" 2>&1
  local status=$?
  sed 's/\x1b\[[0-9;]*m//g' "$dir/run.log" > "$dir/run.txt"
  local line passed failed skipped first
  line=$(grep -E '^ +Tests +' "$dir/run.txt" | tail -1)
  passed=$(sed -n 's/.*\b\([0-9]\+\) passed.*/\1/p' <<< "$line"); failed=$(sed -n 's/.*\b\([0-9]\+\) failed.*/\1/p' <<< "$line"); skipped=$(sed -n 's/.*\b\([0-9]\+\) skipped.*/\1/p' <<< "$line")
  first=$(grep -m1 -E '^ +× ' "$dir/run.txt" | sed 's/^ *× //')
  printf 'j%s-r%s\t%s\t%s\t%s\t%s\t%s\n' "$job" "$round" "$status" "${passed:-0}" "${failed:-0}" "${skipped:-0}" "${first:--}" >> "$base/summary.tsv"
}

for job in $(seq 1 "$jobs"); do
  ( for round in $(seq 1 "$rounds"); do one "$job" "$round"; done ) &
done
wait
sort -V "$base/summary.tsv" -o "$base/summary.tsv"
cat "$base/summary.tsv"
echo "green: $(awk -F'\t' '$2 == 0' "$base/summary.tsv" | wc -l) of $(wc -l < "$base/summary.tsv")"
