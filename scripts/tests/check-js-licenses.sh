#!/usr/bin/env bash
# Test of scripts/check-js-licenses.mjs with the JSON of `pnpm licenses` replaced by LICENSES_JSON (needs only Node).
set -euo pipefail
cd -- "$(dirname -- "$0")/../.."

ok='{"MIT":[{"name":"left-pad","versions":["1.0.0"],"license":"MIT"}],"ISC":[{"name":"right-pad","versions":["2.0.0"],"license":"ISC"}],"dual":[{"name":"both","versions":["1.0.0"],"license":"(MIT OR GPL-2.0-only)"}]}'
LICENSES_JSON=$ok scripts/check-js-licenses.mjs >/dev/null

bad='{"MIT":[{"name":"left-pad","versions":["1.0.0"],"license":"MIT"}],"GPL-2.0-only":[{"name":"copyleft-pad","versions":["3.1.0"],"license":"GPL-2.0-only"}]}'
if out=$(LICENSES_JSON=$bad scripts/check-js-licenses.mjs 2>&1); then
  echo "licence check accepted GPL-2.0-only" >&2; exit 1
fi
grep -q 'copyleft-pad@3.1.0' <<<"$out" || { echo "licence check did not name the package: $out" >&2; exit 1; }

# Both sides of an AND must be allowed, and a missing licence is a failure.
and='{"x":[{"name":"andpkg","versions":["1.0.0"],"license":"MIT AND GPL-2.0-only"}]}'
if LICENSES_JSON=$and scripts/check-js-licenses.mjs >/dev/null 2>&1; then echo "licence check accepted an AND with GPL" >&2; exit 1; fi
none='{"Unknown":[{"name":"nolicence","versions":["1.0.0"]}]}'
if LICENSES_JSON=$none scripts/check-js-licenses.mjs >/dev/null 2>&1; then echo "licence check accepted a missing licence" >&2; exit 1; fi

echo "licence check rejects licences outside deny.toml"
