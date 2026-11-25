/// <reference types="node" />
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

const SRC = join(import.meta.dirname, "..");
const TOKENS = join("design", "tokens.css");

function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return cssFiles(path);
    return name.endsWith(".css") ? [path] : [];
  });
}

interface Declaration { file: string; selector: string; property: string; value: string }

function declarations(): Declaration[] {
  return cssFiles(SRC)
    .filter((file) => relative(SRC, file) !== TOKENS)
    .flatMap((file) => {
      const text = readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
      return [...text.matchAll(/([^{}]*)\{([^{}]*)\}/g)].flatMap((block) =>
        block[2].split(";").flatMap((decl) => {
          const at = decl.indexOf(":");
          if (at < 0) return [];
          return [{ file: relative(SRC, file), selector: block[1].trim(), property: decl.slice(0, at).trim().toLowerCase(), value: decl.slice(at + 1).trim() }];
        }),
      );
    });
}

const all = declarations();
const show = (d: Declaration) => `${d.file}: ${d.selector} { ${d.property}: ${d.value} }`;

/** Spaces: padding, margin and gap take `var(--sp-*)`, `0`, `auto` or a `calc()` of tokens, one value per side. */
const SPACING_PROPERTY = /^(padding|margin)(-[a-z]+)*$|^(gap|row-gap|column-gap|grid-gap)$/;
const SPACE_ONE = String.raw`(?:var\(--sp-[1-9]\)|calc\((?:var\(--sp-[1-9]\)|[\s+\-*/()]|\d+(?:\.\d+)?(?![\w.%]))+\)|0|auto)`;
const SPACE_VALUE = new RegExp(String.raw`^${SPACE_ONE}(?:\s+${SPACE_ONE}){0,3}$`);
/** The one written exception: the standard way to hide text and still let a screen reader read it. */
const SPACING_EXCEPTIONS = [{ selector: ".visually-hidden", property: "margin", value: "-1px" }];
const offScaleSpacing = (d: Declaration) =>
  SPACING_PROPERTY.test(d.property) &&
  !SPACE_VALUE.test(d.value) &&
  !SPACING_EXCEPTIONS.some((e) => e.selector === d.selector && e.property === d.property && e.value === d.value);

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
    const off = all.filter((d) => scale[d.property] && !scale[d.property].test(d.value));
    expect(off.map(show)).toEqual([]);
  });

  it("spacing_uses_the_scale", () => {
    expect(all.filter(offScaleSpacing).map(show)).toEqual([]);
  });

  it("the_spacing_rule_tells_a_token_from_a_raw_length", () => {
    const d = (property: string, value: string, selector = ".x"): Declaration => ({ file: "f.css", selector, property, value });
    expect(offScaleSpacing(d("padding", "var(--sp-2) 10px"))).toBe(true);
    expect(offScaleSpacing(d("margin-top", "8px"))).toBe(true);
    expect(offScaleSpacing(d("row-gap", "1rem"))).toBe(true);
    expect(offScaleSpacing(d("margin", "-1px"))).toBe(true);
    expect(offScaleSpacing(d("padding", "calc(var(--sp-2) + 3px)"))).toBe(true);
    expect(offScaleSpacing(d("padding", "var(--sp-2) var(--sp-5)"))).toBe(false);
    expect(offScaleSpacing(d("margin", "0 auto"))).toBe(false);
    expect(offScaleSpacing(d("gap", "calc(var(--sp-2) * 2)"))).toBe(false);
    expect(offScaleSpacing(d("margin", "-1px", ".visually-hidden"))).toBe(false);
    expect(offScaleSpacing(d("width", "10px"))).toBe(false);
  });
});
