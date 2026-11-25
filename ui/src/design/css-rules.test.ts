/// <reference types="node" />
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

const SRC = join(import.meta.dirname, "..");
const TOKENS = join("design", "tokens.css");
// styles.css still carries the sizes of B1; it disappears in T6 and takes this exception with it.
const SCALE_EXCEPTIONS = new Set(["styles.css"]);

function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return cssFiles(path);
    return name.endsWith(".css") ? [path] : [];
  });
}

interface Declaration { file: string; property: string; value: string }

function declarations(): Declaration[] {
  return cssFiles(SRC)
    .filter((file) => relative(SRC, file) !== TOKENS)
    .flatMap((file) => {
      const text = readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
      return [...text.matchAll(/\{([^{}]*)\}/g)].flatMap((block) =>
        block[1].split(";").flatMap((decl) => {
          const at = decl.indexOf(":");
          if (at < 0) return [];
          return [{ file: relative(SRC, file), property: decl.slice(0, at).trim().toLowerCase(), value: decl.slice(at + 1).trim() }];
        }),
      );
    });
}

const all = declarations();
const show = (d: Declaration) => `${d.file}: ${d.property}: ${d.value}`;

describe("css rules", () => {
  it("there_are_css_files_to_check", () => {
    expect(all.length).toBeGreaterThan(0);
  });

  it("no_raw_colour_outside_tokens_css", () => {
    const raw = /#[0-9a-f]{3,8}\b|\brgba?\(|\bhsla?\(|\boklch\(|\boklab\(|\blab\(|\blch\(/i;
    expect(all.filter((d) => raw.test(d.value)).map(show)).toEqual([]);
  });

  it("accent_is_never_a_text_colour", () => {
    expect(all.filter((d) => d.property === "color" && /var\(--accent\)/.test(d.value)).map(show)).toEqual([]);
  });

  it("radius_and_font_size_use_the_scale", () => {
    const scale: Record<string, RegExp> = {
      "border-radius": /^(var\(--r-(xs|sm|md|lg|full)\)\s*)+$|^(inherit|0|50%)$/,
      "font-size": /^(var\(--fs-(xs|sm|md|base|lg|xl)\)|inherit)$/,
    };
    const off = all.filter((d) => !SCALE_EXCEPTIONS.has(d.file) && scale[d.property] && !scale[d.property].test(d.value));
    expect(off.map(show)).toEqual([]);
  });
});
