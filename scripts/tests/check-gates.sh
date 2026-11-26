#!/usr/bin/env bash
# scripts/check-gates.py on made-up output: a gate must not come out green when its proof is missing, skipped or ignored.
set -uo pipefail
cd -- "$(dirname -- "$0")/../.."
tmp=$(mktemp -d)
trap 'rm -rf -- "$tmp"' EXIT
failures=0

manifest=$tmp/gates.tsv
printf '# comment\nG1\tjob-a\tcargo\tfirst_proof\nG1\tjob-a\tcargo\tsecond_proof\nG2\tjob-a\tvitest\tthe_window_asks\nG3\tjob-b\tcargo\tother_proof\n' > "$manifest"

check() { # description, wanted exit code, args...
  local what=$1 wanted=$2; shift 2
  scripts/check-gates.py --manifest "$manifest" "$@" > "$tmp/out" 2> "$tmp/err"
  local got=$?
  if [[ $got == "$wanted" ]]; then echo "ok   $what"; else echo "FAIL $what: exit $got, wanted $wanted" >&2; cat "$tmp/out" "$tmp/err" >&2; failures=$((failures + 1)); fi
}

printf 'running 2 tests\ntest tests::first_proof ... ok\ntest second_proof ... ok\n' > "$tmp/all-ok.log"
printf 'test first_proof ... ok\ntest second_proof ... ignored, needs a bus\n' > "$tmp/ignored.log"
printf 'test first_proof ... ok\ntest unrelated ... ok\n' > "$tmp/missing.log"
printf 'test first_proof ... ok\ntest second_proof ... FAILED\n' > "$tmp/failed.log"
printf '{"testResults":[{"assertionResults":[{"title":"the_window_asks","fullName":"suite the_window_asks","status":"passed"}]}]}' > "$tmp/vitest-ok.json"
printf '{"testResults":[{"assertionResults":[{"title":"the_window_asks","fullName":"suite the_window_asks","status":"skipped"}]}]}' > "$tmp/vitest-skipped.json"

check all_listed_tests_passed_is_green 0 --job job-a --cargo-log "$tmp/all-ok.log" --vitest-json "$tmp/vitest-ok.json"
check an_ignored_test_fails_the_gate 1 --job job-a --cargo-log "$tmp/ignored.log" --vitest-json "$tmp/vitest-ok.json"
check a_missing_test_fails_the_gate 1 --job job-a --cargo-log "$tmp/missing.log" --vitest-json "$tmp/vitest-ok.json"
check a_failed_test_fails_the_gate 1 --job job-a --cargo-log "$tmp/failed.log" --vitest-json "$tmp/vitest-ok.json"
check a_skipped_vitest_test_fails_the_gate 1 --job job-a --cargo-log "$tmp/all-ok.log" --vitest-json "$tmp/vitest-skipped.json"
check a_vitest_test_that_is_not_in_the_json_fails_the_gate 1 --job job-a --cargo-log "$tmp/all-ok.log"
check a_job_without_rows_is_an_error 2 --job job-none --cargo-log "$tmp/all-ok.log"
check a_log_that_is_not_there_is_an_error 2 --job job-a --cargo-log "$tmp/not-there.log"
# the same test in two logs: one that did not pass is enough to fail
check one_bad_line_among_good_ones_fails_the_gate 1 --job job-a --cargo-log "$tmp/all-ok.log" --cargo-log "$tmp/ignored.log" --vitest-json "$tmp/vitest-ok.json"

# a name that is only the end of another name is not that proof
printf 'test not_first_proof ... ok\ntest second_proof ... ok\n' > "$tmp/suffix.log"
check a_name_that_only_ends_alike_is_not_the_proof 1 --job job-a --cargo-log "$tmp/suffix.log" --vitest-json "$tmp/vitest-ok.json"

# the real manifest reads
scripts/check-gates.py --job gui-smoke --cargo-log /dev/null > /dev/null 2>&1
[[ $? == 1 ]] && echo "ok   the_real_manifest_fails_without_output" || { echo "FAIL the real manifest did not fail on empty output" >&2; failures=$((failures + 1)); }

# Plan B6, T6: one manifest for each block, read together; a gate belongs to one of them.
second=$tmp/gates-second.tsv
printf 'G9\tjob-a\tcargo\tthird_proof\n' > "$second"
printf 'test first_proof ... ok\ntest second_proof ... ok\ntest third_proof ... ok\n' > "$tmp/three-ok.log"
printf 'test first_proof ... ok\ntest second_proof ... ok\ntest third_proof ... ignored\n' > "$tmp/three-ignored.log"
check_two() { # description, wanted exit code, args...
  local what=$1 wanted=$2; shift 2
  scripts/check-gates.py --manifest "$manifest" --manifest "$second" "$@" > "$tmp/out" 2> "$tmp/err"
  local got=$?
  if [[ $got == "$wanted" ]]; then echo "ok   $what"; else echo "FAIL $what: exit $got, wanted $wanted" >&2; cat "$tmp/out" "$tmp/err" >&2; failures=$((failures + 1)); fi
}
check_two two_manifests_are_read_together 0 --job job-a --cargo-log "$tmp/three-ok.log" --vitest-json "$tmp/vitest-ok.json"
grep -q '^G1 cargo first_proof: ok' "$tmp/out" && grep -q '^G9 cargo third_proof: ok' "$tmp/out" || { echo "FAIL the proof of the second manifest was not read" >&2; failures=$((failures + 1)); }
check_two a_proof_of_the_second_manifest_that_is_ignored_fails_the_gate 1 --job job-a --cargo-log "$tmp/three-ignored.log" --vitest-json "$tmp/vitest-ok.json"
check_two a_proof_of_the_second_manifest_that_is_missing_fails_the_gate 1 --job job-a --cargo-log "$tmp/all-ok.log" --vitest-json "$tmp/vitest-ok.json"
printf 'G1\tjob-b\tcargo\tfourth_proof\n' > "$second"
check_two a_gate_in_two_manifests_is_an_error 2 --job job-a --cargo-log "$tmp/all-ok.log" --vitest-json "$tmp/vitest-ok.json"
grep -q "the gate G1 is also in" "$tmp/err" || { echo "FAIL the error does not name the gate" >&2; cat "$tmp/err" >&2; failures=$((failures + 1)); }

# Without --manifest every lab/tests/gates-*.tsv is read: the gates of B5 and the ones of B6 in the one job `gui-smoke`.
scripts/check-gates.py --job gui-smoke --cargo-log /dev/null > "$tmp/out" 2> /dev/null
grep -q '^G5 cargo keyring_roundtrip: MISSING' "$tmp/out" && grep -q '^G10 vitest G10b_' "$tmp/out" \
  && echo "ok   the_default_reads_every_manifest" || { echo "FAIL the default did not read the manifests of B5 and B6 together" >&2; cat "$tmp/out" >&2; failures=$((failures + 1)); }

# The smoke test is the proof of G10 and G11: the JSON of its run, made from the manifest itself. One proof that was skipped or is
# not in it, and the gate is red (a skipped test counts as green in most runners, and that is how a gate stops proving anything).
python3 - "$tmp" <<'PY'
import json, sys
tmp = sys.argv[1]
rows = [l.split("\t") for l in open("lab/tests/gates-b6.tsv").read().splitlines() if l.strip() and not l.startswith("#")]
names = [r[3].strip() for r in rows if r[1] == "gui-smoke" and r[2] == "vitest"]
def write(file, skip=None, leave=None):
    cases = [{"title": n, "fullName": "suite " + n, "status": "skipped" if n == skip else "passed"} for n in names if n != leave]
    json.dump({"testResults": [{"assertionResults": cases}]}, open(f"{tmp}/{file}", "w"))
write("smoke-ok.json")
write("smoke-skipped.json", skip=next(n for n in names if n.startswith("G10b")))
write("smoke-left-out.json", leave=next(n for n in names if n.startswith("G10b")))
PY
smoke() { scripts/check-gates.py --job gui-smoke --kind vitest --vitest-json "$1" > "$tmp/out" 2> "$tmp/err"; }
smoke "$tmp/smoke-ok.json"; [[ $? == 0 ]] && echo "ok   the_whole_smoke_run_is_green" || { echo "FAIL the smoke run with every proof passed is not green" >&2; cat "$tmp/out" "$tmp/err" >&2; failures=$((failures + 1)); }
smoke "$tmp/smoke-skipped.json"; got=$?
[[ $got == 1 ]] && grep -q 'G10 vitest G10b_.*NOT PROVED (skipped)' "$tmp/out" && echo "ok   a_skipped_smoke_test_fails_the_gate" || { echo "FAIL a skipped smoke test did not fail the gate (exit $got)" >&2; cat "$tmp/out" "$tmp/err" >&2; failures=$((failures + 1)); }
smoke "$tmp/smoke-left-out.json"; got=$?
[[ $got == 1 ]] && grep -q 'G10 vitest G10b_.*MISSING' "$tmp/out" && echo "ok   a_smoke_test_that_is_not_in_the_json_fails_the_gate" || { echo "FAIL a smoke test missing from the JSON did not fail the gate (exit $got)" >&2; cat "$tmp/out" "$tmp/err" >&2; failures=$((failures + 1)); }

(( failures == 0 )) || { echo "$failures check(s) failed" >&2; exit 1; }
echo "check-gates: all checks passed"
