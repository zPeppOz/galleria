#!/usr/bin/env node
// Licence check of the production JavaScript dependencies (spec §12): every package that ends up in the app must
// carry a licence from the `allow` list of deny.toml, the list `cargo deny` uses for the Rust side.
//
// Reads `pnpm licenses list --prod --json` run in ui/ (S16: an object keyed by licence, each value an array of
// packages with `name`, `versions` and `license`). LICENSES_JSON replaces that output, for the test of this script.
// Exit 0 when every licence is allowed; otherwise 1, with one line per offending package.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function allowedLicences() {
  const toml = readFileSync(join(root, "deny.toml"), "utf8");
  const section = toml.split(/^\[licenses\]\s*$/m)[1]?.split(/^\[/m)[0] ?? "";
  const list = section.match(/^allow\s*=\s*\[([\s\S]*?)^\]/m)?.[1];
  if (!list) throw new Error("deny.toml has no [licenses] allow list");
  const allowed = [...list.replace(/#.*$/gm, "").matchAll(/"([^"]+)"/g)].map((m) => m[1]);
  if (allowed.length === 0) throw new Error("the allow list of deny.toml is empty");
  return new Set(allowed);
}

/** An SPDX expression is fine when one side of every OR is fine and both sides of every AND are. */
function expressionAllowed(expression, allowed) {
  const text = expression.trim();
  if (allowed.has(text)) return true;
  const inner = text.replace(/^\((.*)\)$/s, "$1");
  if (inner !== text) return expressionAllowed(inner, allowed);
  const top = (op) => {
    let depth = 0;
    const parts = [];
    let start = 0;
    const re = /[()]|\s(OR|AND)\s/g;
    for (let m; (m = re.exec(text)); ) {
      if (m[0] === "(") depth++;
      else if (m[0] === ")") depth--;
      else if (depth === 0 && m[1] === op) {
        parts.push(text.slice(start, m.index));
        start = m.index + m[0].length;
      }
    }
    return parts.length === 0 ? null : [...parts, text.slice(start)];
  };
  const alternatives = top("OR");
  if (alternatives) return alternatives.some((part) => expressionAllowed(part, allowed));
  const all = top("AND");
  if (all) return all.every((part) => expressionAllowed(part, allowed));
  return false;
}

function licencesJson() {
  if (process.env.LICENSES_JSON) return process.env.LICENSES_JSON;
  return execFileSync("pnpm", ["licenses", "list", "--prod", "--json"], {
    cwd: join(root, "ui"),
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
}

const allowed = allowedLicences();
const byLicence = JSON.parse(licencesJson());
const offenders = [];
let count = 0;
for (const [licence, packages] of Object.entries(byLicence)) {
  for (const pkg of packages) {
    count++;
    const declared = pkg.license || licence;
    if (!declared || declared === "Unknown" || !expressionAllowed(declared, allowed)) {
      offenders.push(`${pkg.name}@${(pkg.versions ?? []).join(",")}: ${declared || "no licence"}`);
    }
  }
}
if (count === 0) {
  console.error("check-js-licenses: no package found; refusing to call that a pass");
  process.exit(1);
}
if (offenders.length > 0) {
  console.error("check-js-licenses: licence not in the allow list of deny.toml:");
  for (const line of offenders) console.error(`  ${line}`);
  process.exit(1);
}
console.log(`check-js-licenses: ${count} production packages, all licences allowed`);
