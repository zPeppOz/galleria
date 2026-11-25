/// <reference types="node" />
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { contrast, OKLCH_TABLE, parseColour, tintOver, toHex, type Rgb } from "./color";

const css = readFileSync(join(import.meta.dirname, "tokens.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

function declarations(block: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const m of block.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) out[m[1]] = m[2].trim();
  return out;
}

function blockOf(selector: RegExp): string {
  const m = selector.exec(css);
  if (!m) throw new Error(`tokens.css has no block for ${selector}`);
  let depth = 0;
  for (let i = m.index + m[0].length - 1; i < css.length; i++) {
    if (css[i] === "{") depth++;
    if (css[i] === "}" && --depth === 0) return css.slice(m.index + m[0].length, i);
  }
  throw new Error("unbalanced braces in tokens.css");
}

const root = declarations(blockOf(/:root\s*\{/));
const lightOnly = declarations(blockOf(/:root\[data-theme="light"\]\s*\{/));
const THEMES = {
  dark: root,
  light: { ...root, ...lightOnly },
} as const;
type ThemeName = keyof typeof THEMES;
const NAMES = Object.keys(THEMES) as ThemeName[];

const SURFACES = ["--bg", "--panel", "--panel-2"] as const;
const colourOf = (theme: ThemeName, token: string): Rgb => parseColour(THEMES[theme][token] ?? `missing ${token}`);
const tintAlpha = (theme: ThemeName): number => Number.parseFloat(THEMES[theme]["--tint"] ?? "0") / 100;
const cases = <T>(list: readonly T[]) => NAMES.flatMap((theme) => list.map((item) => [theme, item] as const));

describe("design tokens", () => {
  it("both_themes_define_the_same_tokens", () => {
    // What a theme must restate: colours. The rest (radii, spaces, heights, type) is the same by construction and
    // lives in :root only.
    const byConstruction = /^--(r-|sp-|h-|fs-|fw-|lh|font|mono|tint|scrim|shadow-|focus-)/;
    const colours = (names: string[]) => names.filter((n) => !byConstruction.test(n)).sort();
    expect(colours(Object.keys(lightOnly))).toEqual(colours(Object.keys(root)));
    expect(/color-scheme:\s*dark/.test(blockOf(/:root\s*\{/))).toBe(true);
    expect(/color-scheme:\s*light/.test(blockOf(/:root\[data-theme="light"\]\s*\{/))).toBe(true);
  });

  it.each(cases(["--ink", "--muted", "--faint", "--ok", "--warn", "--err", "--wg", "--ovpn"]))("text_tokens_reach_4_5_on_every_surface: %s %s", (theme, token) => {
    for (const surface of SURFACES) {
      expect(contrast(colourOf(theme, token), colourOf(theme, surface)), `${token} on ${surface}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it.each(NAMES)("accent_ink_reaches_4_5_on_accent: %s", (theme) => {
    expect(contrast(colourOf(theme, "--accent-ink"), colourOf(theme, "--accent"))).toBeGreaterThanOrEqual(4.5);
  });

  it.each(cases(["--line-strong", "--accent"]))("control_borders_and_focus_ring_reach_3: %s %s", (theme, token) => {
    for (const surface of SURFACES) {
      expect(contrast(colourOf(theme, token), colourOf(theme, surface)), `${token} on ${surface}`).toBeGreaterThanOrEqual(3);
    }
  });

  it.each(cases(["--wg", "--ovpn"]))("protocol_labels_reach_4_5_on_their_tint: %s %s", (theme, token) => {
    for (const surface of SURFACES) {
      const tint = tintOver(colourOf(theme, token), tintAlpha(theme), colourOf(theme, surface));
      expect(contrast(colourOf(theme, token), tint), `${token} on its tint over ${surface}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it.each(cases(["--ok", "--warn", "--err"]))("ink_reaches_4_5_on_status_tints: %s %s", (theme, token) => {
    for (const surface of SURFACES) {
      const tint = tintOver(colourOf(theme, token), tintAlpha(theme), colourOf(theme, surface));
      expect(contrast(colourOf(theme, "--ink"), tint), `--ink on the tint of ${token} over ${surface}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("oklch_tokens_match_the_hex_table", () => {
    for (const theme of NAMES) {
      for (const [token, { oklch, hex }] of Object.entries(OKLCH_TABLE[theme])) {
        expect(THEMES[theme][token], `${token} (${theme}) in tokens.css`).toBe(oklch);
        const got = colourOf(theme, token);
        const want = parseColour(hex);
        got.forEach((v, i) => expect(Math.abs(Math.round(v) - want[i]), `${token} (${theme}) channel ${i}: ${toHex(got)} against ${hex}`).toBeLessThanOrEqual(1));
      }
    }
  });
});
