// Colour arithmetic for the tests of the design tokens (tokens.test.ts, the smoke test of S8). Not used by the app.
// Reads `#rgb`, `#rrggbb` and `oklch(L C h)`, converts to sRGB, computes the WCAG 2 contrast ratio and composes a
// translucent tint over a surface, the way the browser does when it paints `color-mix(in oklch, X 14%, transparent)`.

export type Rgb = [number, number, number]; // 0–255, not rounded

/** The twelve colour tokens that are written as `oklch()` in tokens.css, with the sRGB hex the plan computed for them. */
export const OKLCH_TABLE: Record<"dark" | "light", Record<string, { oklch: string; hex: string }>> = {
  dark: {
    "--accent": { oklch: "oklch(0.76 0.14 62)", hex: "#f09b48" },
    "--ok": { oklch: "oklch(0.76 0.15 150)", hex: "#61cb7c" },
    "--warn": { oklch: "oklch(0.8 0.14 85)", hex: "#e7b643" },
    "--err": { oklch: "oklch(0.68 0.19 25)", hex: "#f75d59" },
    "--wg": { oklch: "oklch(0.74 0.11 200)", hex: "#3ebfc6" },
    "--ovpn": { oklch: "oklch(0.75 0.12 40)", hex: "#ee9373" },
  },
  light: {
    "--accent": { oklch: "oklch(0.62 0.15 55)", hex: "#c9690c" },
    "--ok": { oklch: "oklch(0.47 0.13 150)", hex: "#006e30" },
    "--warn": { oklch: "oklch(0.48 0.095 75)", hex: "#7c550f" },
    "--err": { oklch: "oklch(0.5 0.19 27)", hex: "#b7191c" },
    "--wg": { oklch: "oklch(0.46 0.075 215)", hex: "#146272" },
    "--ovpn": { oklch: "oklch(0.48 0.14 40)", hex: "#9c3a11" },
  },
};

const encode = (v: number): number => (v <= 0.0031308 ? 12.92 * v : 1.055 * v ** (1 / 2.4) - 0.055);
const decode = (v: number): number => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);

/** Linear sRGB of an OKLCH colour, not clamped: a channel outside 0–1 means the colour is outside the sRGB gamut. */
export function oklchToLinear(L: number, C: number, hDeg: number): Rgb {
  const h = (hDeg * Math.PI) / 180;
  const a = C * Math.cos(h);
  const b = C * Math.sin(h);
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
}

/** True when the OKLCH colour fits in sRGB (every channel within a rounding step of 0–1). */
export function oklchInGamut(L: number, C: number, h: number): boolean {
  return oklchToLinear(L, C, h).every((v) => v > -0.0005 && v < 1.0005);
}

export function parseColour(text: string): Rgb {
  const value = text.trim().toLowerCase();
  const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/.exec(value);
  if (hex) {
    const digits = hex[1].length === 3 ? [...hex[1]].map((c) => c + c).join("") : hex[1];
    return [0, 2, 4].map((i) => Number.parseInt(digits.slice(i, i + 2), 16)) as Rgb;
  }
  const ok = /^oklch\(\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)\s*\)$/.exec(value);
  if (ok) {
    const [L, C, h] = [Number(ok[1]), Number(ok[2]), Number(ok[3])];
    return oklchToLinear(L, C, h).map((v) => 255 * encode(Math.min(1, Math.max(0, v)))) as Rgb;
  }
  throw new Error(`cannot read the colour «${text}» (only #rgb, #rrggbb and oklch(L C h))`);
}

export const toHex = (rgb: Rgb): string => `#${rgb.map((v) => Math.round(v).toString(16).padStart(2, "0")).join("")}`;

const luminance = (rgb: Rgb): number => {
  const [r, g, b] = rgb.map((v) => decode(v / 255));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};

/** WCAG 2 contrast ratio between two opaque colours, 1 to 21. */
export function contrast(a: Rgb, b: Rgb): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/** `colour` painted at `alpha` (0–1) over the opaque `surface`: what `color-mix(in oklch, X 14%, transparent)` looks like. */
export const tintOver = (colour: Rgb, alpha: number, surface: Rgb): Rgb => colour.map((v, i) => v * alpha + surface[i] * (1 - alpha)) as Rgb;
