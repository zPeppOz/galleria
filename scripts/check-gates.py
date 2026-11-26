#!/usr/bin/env python3
"""Checks that every proof a CI job declares for a gate really ran and passed (plan B5, T2).

A proof that is skipped counts as green in most runners, and that is how a gate stops proving anything. This reads
`lab/tests/gates-b5.tsv` (gate, job, kind, name; tab separated, `#` starts a comment) and, for the rows of one job, looks
for each proof in the output the job kept:

  cargo   a line `test <path::>name ... ok` in a `cargo test` log. `ignored`, `FAILED`, or no line at all, is an error.
  vitest  `name` is the full name or the title of a test in the JSON of `vitest --reporter=json`, with status `passed`.
          `skipped`, `todo`, `pending`, `failed`, or not there at all, is an error.
  lab     `name` is the name of a test in a log that has lines `ok <name>`; for the lab jobs, see the later tasks.

    check-gates.py --job <job> [--kind K]... [--cargo-log FILE]... [--vitest-json FILE]... [--lab-log FILE]... [--manifest FILE]

Prints one line per proof and exits with 1 at the first gate that is not proved (the others are still printed), 2 on a bad
call or a bad manifest.
"""
import argparse
import json
import re
import sys
from pathlib import Path

CARGO_LINE = re.compile(r"^test (\S+) \.\.\. (\w+)")
LAB_LINE = re.compile(r"^\s*(ok|FAIL|SKIP|skipped)\s+(\S+)")


def read_manifest(path):
    rows = []
    for number, line in enumerate(Path(path).read_text().splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        cells = line.split("\t")
        if len(cells) != 4 or not all(c.strip() for c in cells):
            sys.exit(f"{path}:{number}: want gate, job, kind and name separated by tabs, got {line!r}")
        gate, job, kind, name = (c.strip() for c in cells)
        if kind not in ("cargo", "vitest", "lab"):
            sys.exit(f"{path}:{number}: unknown kind {kind!r}")
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
    ap.add_argument("--manifest", default=str(Path(__file__).resolve().parent.parent / "lab/tests/gates-b5.tsv"))
    args = ap.parse_args()

    rows = [r for r in read_manifest(args.manifest) if r[1] == args.job and (not args.kind or r[2] in args.kind)]
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
    if unproved:
        print(f"check-gates: {unproved} of {len(rows)} proofs of {args.job} are not proved", file=sys.stderr)
        return 1
    print(f"check-gates: {len(rows)} proofs of {args.job} ran and passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
