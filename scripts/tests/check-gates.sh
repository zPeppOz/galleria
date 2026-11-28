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

# Plan P1, T4b: a job that has no ignored test at all (`win-check`: on Windows a test of Linux does not exist, it is not ignored).
result='test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s'
printf 'running 2 tests\ntest first_proof ... ok\ntest second_proof ... ok\n\n%s\n' "$result" > "$tmp/none-ignored.log"
printf 'test first_proof ... ok\ntest second_proof ... ok\ntest other ... ignored, needs a bus\n\n%s\n' "$result" > "$tmp/ignored-outside.log"
printf 'test first_proof ... ok\ntest second_proof ... ok\n\ntest result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s\n' > "$tmp/ignored-in-result.log"
printf 'test first_proof ... ok\ntest second_proof ... ok\n\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s\n' > "$tmp/filtered.log"
printf 'test first_proof ... ok\ntest second_proof ... ok\n\n%s\n\n%s\n' "$result" "$result" > "$tmp/two-binaries.log"
check no_ignored_is_green_when_nothing_is_ignored 0 --job job-a --kind cargo --cargo-log "$tmp/none-ignored.log" --no-ignored
grep -q '^check-gates: job-a: 2 passed; 0 ignored$' "$tmp/out" || { echo "FAIL the total line is not 'check-gates: job-a: 2 passed; 0 ignored'" >&2; cat "$tmp/out" >&2; failures=$((failures + 1)); }
check the_total_adds_up_the_result_lines_of_every_binary 0 --job job-a --kind cargo --cargo-log "$tmp/two-binaries.log" --no-ignored
grep -q '^check-gates: job-a: 4 passed; 0 ignored$' "$tmp/out" || { echo "FAIL the total of two binaries is not 4" >&2; cat "$tmp/out" >&2; failures=$((failures + 1)); }
check no_ignored_fails_on_an_ignored_test_outside_the_manifest 1 --job job-a --kind cargo --cargo-log "$tmp/ignored-outside.log" --no-ignored
check no_ignored_fails_on_a_result_line_that_counts_an_ignored_test 1 --job job-a --kind cargo --cargo-log "$tmp/ignored-in-result.log" --no-ignored
check no_ignored_fails_on_a_result_line_with_filtered_out_tests 1 --job job-a --kind cargo --cargo-log "$tmp/filtered.log" --no-ignored
check min_passed_fails_below_the_floor 1 --job job-a --kind cargo --cargo-log "$tmp/none-ignored.log" --no-ignored --min-passed 3
grep -q 'below the floor' "$tmp/err" || { echo "FAIL the floor is not named in the error" >&2; cat "$tmp/err" >&2; failures=$((failures + 1)); }
check min_passed_is_green_at_the_floor 0 --job job-a --kind cargo --cargo-log "$tmp/none-ignored.log" --no-ignored --min-passed 2
check without_the_flag_an_ignored_test_outside_the_manifest_is_fine 0 --job job-a --kind cargo --cargo-log "$tmp/ignored-outside.log"
check without_the_flag_a_filtered_result_line_is_fine 0 --job job-a --kind cargo --cargo-log "$tmp/filtered.log"
# the floor and the flag do not make a proof that is missing green
check a_missing_proof_is_red_with_the_flag_too 1 --job job-a --kind cargo --cargo-log "$tmp/missing.log" --no-ignored --min-passed 1
# a floor that is not a number is a bad call
check a_floor_that_is_not_a_number_is_an_error 2 --job job-a --kind cargo --cargo-log "$tmp/none-ignored.log" --min-passed many
# a log with no result line at all passes nothing: the floor sees 0
printf 'test first_proof ... ok\ntest second_proof ... ok\n' > "$tmp/no-result.log"
check a_log_without_result_lines_has_no_passed_tests 1 --job job-a --kind cargo --cargo-log "$tmp/no-result.log" --no-ignored --min-passed 1
# the manifest of P1 is read with the others
scripts/check-gates.py --job win-check --cargo-log /dev/null > "$tmp/out" 2> /dev/null
grep -q '^WP1 cargo the_client_says_unsupported_without_a_transport: MISSING' "$tmp/out" && grep -q '^WP2 cargo a_link_is_not_imported: MISSING' "$tmp/out" \
  && echo "ok   the_default_reads_the_manifest_of_win_check" || { echo "FAIL the default did not read lab/tests/gates-w.tsv" >&2; cat "$tmp/out" >&2; failures=$((failures + 1)); }

(( failures == 0 )) || { echo "$failures check(s) failed" >&2; exit 1; }
echo "check-gates: all checks passed"
