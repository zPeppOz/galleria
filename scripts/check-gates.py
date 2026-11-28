#!/usr/bin/env python3
"""Checks that every proof a CI job declares for a gate really ran and passed (plan B5, T2).

A proof that is skipped counts as green in most runners, and that is how a gate stops proving anything. This reads
every `lab/tests/gates-*.tsv` (gate, job, kind, name; tab separated, `#` starts a comment; one file for each block of the
plan, and a gate that is in two of them is an error) and, for the rows of one job, looks for each proof in the output the
job kept:

  cargo   a line `test <path::>name ... ok` in a `cargo test` log. `ignored`, `FAILED`, or no line at all, is an error.
  vitest  `name` is the full name or the title of a test in the JSON of `vitest --reporter=json`, with status `passed`.
          `skipped`, `todo`, `pending`, `failed`, or not there at all, is an error.
  lab     `name` is the name of a test in a log that has lines `ok <name>`; for the lab jobs, see the later tasks.

    check-gates.py --job <job> [--kind K]... [--cargo-log FILE]... [--vitest-json FILE]... [--lab-log FILE]... [--manifest FILE]...
                   [--no-ignored] [--min-passed N]

`--no-ignored` is for a job that must not leave a test out (plan P1, T4b: `win-check`, where a test of Linux does not exist
instead of being ignored). A `test ... ignored` line anywhere in the cargo logs, or a `test result:` line that has `ignored`
or `filtered out` other than 0, is an error, and the last line is `check-gates: <job>: <n> passed; 0 ignored`. `--min-passed N`
is the floor of the tests that passed (the sum of the `test result:` lines of the cargo logs): fewer is an error, so a
test binary that was dropped from the run cannot go unseen. Without them the check is the one of the other jobs
(the job `check` has 103 legitimate `ignored`).

Prints one line per proof and exits with 1 at the first gate that is not proved (the others are still printed), 2 on a bad
call or a bad manifest.
"""
import argparse
import json
import re
import sys
from pathlib import Path

CARGO_LINE = re.compile(r"^test (\S+) \.\.\. (\w+)")
RESULT_LINE = re.compile(
    r"^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out"
)
LAB_LINE = re.compile(r"^\s*(ok|FAIL|SKIP|skipped)\s+(\S+)")


def bad_manifest(message):
    """A manifest that cannot be read is a bad call (2), not a proof that is missing (1)."""
    print(f"check-gates: {message}", file=sys.stderr)
    sys.exit(2)


def read_manifests(paths):
    """The rows of every manifest. A gate is the subject of one manifest only: each block owns its gates."""
    rows = []
    owner = {}
    for path in paths:
        for row in read_manifest(path):
            first = owner.setdefault(row[0], path)
            if first != path:
                bad_manifest(f"{path}: the gate {row[0]} is also in {first}: a gate has the rows of one manifest only")
            rows.append(row)
    return rows


def read_manifest(path):
    rows = []
    for number, line in enumerate(Path(path).read_text().splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        cells = line.split("\t")
        if len(cells) != 4 or not all(c.strip() for c in cells):
            bad_manifest(f"{path}:{number}: want gate, job, kind and name separated by tabs, got {line!r}")
        gate, job, kind, name = (c.strip() for c in cells)
        if kind not in ("cargo", "vitest", "lab"):
            bad_manifest(f"{path}:{number}: unknown kind {kind!r}")
        rows.append((gate, job, kind, name))
    return rows


def cargo_results(files):
    results = {}
    for file in files:
        for line in Path(file).read_text(errors="replace").splitlines():
            m = CARGO_LINE.match(line)
            if m:
                full, status = m.groups()
                results.setdefault(full, []).append(status)
    return results


def cargo_totals(files):
    """What the `test result:` lines and the test lines of the cargo logs say, summed: passed, ignored, filtered out."""
    totals = {"passed": 0, "ignored_in_results": 0, "filtered_out": 0, "ignored_lines": []}
    for file in files:
        for line in Path(file).read_text(errors="replace").splitlines():
            m = RESULT_LINE.match(line)
            if m:
                passed, _failed, ignored, _measured, filtered = (int(n) for n in m.groups())
                totals["passed"] += passed
                totals["ignored_in_results"] += ignored
                totals["filtered_out"] += filtered
                continue
            m = CARGO_LINE.match(line)
            if m and m.group(2) == "ignored":
                totals["ignored_lines"].append(m.group(1))
    return totals


def vitest_results(files):
    results = {}
    for file in files:
        data = json.loads(Path(file).read_text())
        for suite in data.get("testResults", []):
            for case in suite.get("assertionResults", []):
                for name in {case.get("fullName"), case.get("title")} - {None}:
                    results.setdefault(name, []).append(case.get("status"))
    return results


def lab_results(files):
    results = {}
    for file in files:
        for line in Path(file).read_text(errors="replace").splitlines():
            m = LAB_LINE.match(line)
            if m:
                status, name = m.groups()
                results.setdefault(name, []).append("ok" if status == "ok" else status)
    return results


def lookup(results, name, kind):
    """Statuses of the proof `name`; a cargo name may carry the path of its module in front."""
    if kind == "cargo":
        found = []
        for full, statuses in results.items():
            if full == name or full.endswith("::" + name):
                found += statuses
        return found
    return results.get(name, [])


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--job", required=True)
    ap.add_argument("--cargo-log", action="append", default=[])
    ap.add_argument("--vitest-json", action="append", default=[])
    ap.add_argument("--lab-log", action="append", default=[])
    ap.add_argument("--kind", action="append", choices=["cargo", "vitest", "lab"], help="only the proofs of this kind (repeatable); default: all")
    ap.add_argument("--manifest", action="append", default=[], help="a manifest (repeatable); default: every lab/tests/gates-*.tsv")
    ap.add_argument("--no-ignored", action="store_true", help="a cargo test that is ignored or filtered out is an error; prints the total")
    ap.add_argument("--min-passed", type=int, metavar="N", help="fewer than N cargo tests passed is an error")
    args = ap.parse_args()

    manifests = args.manifest or sorted(str(p) for p in (Path(__file__).resolve().parent.parent / "lab/tests").glob("gates-*.tsv"))
    if not manifests:
        print("check-gates: there is no manifest (lab/tests/gates-*.tsv)", file=sys.stderr)
        return 2
    rows = [r for r in read_manifests(manifests) if r[1] == args.job and (not args.kind or r[2] in args.kind)]
    if not rows:
        print(f"check-gates: the manifest has no proof for the job {args.job!r}", file=sys.stderr)
        return 2
    try:
        results = {
            "cargo": cargo_results(args.cargo_log),
            "vitest": vitest_results(args.vitest_json),
            "lab": lab_results(args.lab_log),
        }
    except (OSError, ValueError) as e:
        print(f"check-gates: cannot read the output of the job: {e}", file=sys.stderr)
        return 2

    unproved = 0
    for gate, _, kind, name in rows:
        statuses = lookup(results[kind], name, kind)
        if not statuses:
            verdict = "MISSING (no such proof in the output)"
        elif all(s in ("ok", "passed") for s in statuses):
            verdict = "ok"
        else:
            verdict = f"NOT PROVED ({', '.join(sorted(set(statuses)))})"
        if verdict != "ok":
            unproved += 1
        print(f"{gate} {kind} {name}: {verdict}")
    problems = []
    totals = None
    if args.no_ignored or args.min_passed is not None:
        totals = cargo_totals(args.cargo_log)
    if args.no_ignored:
        if totals["ignored_lines"]:
            problems.append(f"{len(totals['ignored_lines'])} test(s) ignored, first: {totals['ignored_lines'][0]}")
        if totals["ignored_in_results"]:
            problems.append(f"{totals['ignored_in_results']} ignored in the `test result:` lines")
        if totals["filtered_out"]:
            problems.append(f"{totals['filtered_out']} filtered out in the `test result:` lines")
    if args.min_passed is not None and totals["passed"] < args.min_passed:
        problems.append(f"{totals['passed']} passed, below the floor of {args.min_passed}")
    if unproved:
        print(f"check-gates: {unproved} of {len(rows)} proofs of {args.job} are not proved", file=sys.stderr)
    for problem in problems:
        print(f"check-gates: {args.job}: {problem}", file=sys.stderr)
    if unproved or problems:
        return 1
    print(f"check-gates: {len(rows)} proofs of {args.job} ran and passed")
    if args.no_ignored:
        print(f"check-gates: {args.job}: {totals['passed']} passed; 0 ignored")
    elif totals is not None:
        print(f"check-gates: {args.job}: {totals['passed']} passed (floor {args.min_passed})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
