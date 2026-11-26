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

(( failures == 0 )) || { echo "$failures check(s) failed" >&2; exit 1; }
echo "check-gates: all checks passed"
