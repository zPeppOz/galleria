import type { Lang } from "../i18n";

const UNITS = ["B", "KiB", "MiB", "GiB", "TiB"] as const;

/** A count of bytes in a unit people read: «1.5 MiB». The exact value goes in the `title`, with `exactBytes`. */
export function formatBytes(bytes: number, lang: Lang): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = unit === 0 ? 0 : value < 10 ? 2 : 1;
  return `${new Intl.NumberFormat(lang, { minimumFractionDigits: digits, maximumFractionDigits: digits }).format(value)} ${UNITS[unit]}`;
}

export const exactBytes = (bytes: number, lang: Lang): string => `${new Intl.NumberFormat(lang).format(bytes)} B`;

/** `m:ss`, and `h:mm:ss` from the hour: the same in every language. */
export function formatDuration(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const two = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${two(m)}:${two(s)}` : `${m}:${two(s)}`;
}

/**
 * The time of the last handshake as the backend gives it: seconds since the epoch in a string (WireGuard), or a date. A text that
 * is neither is shown as it is. Anything is better than a made-up time.
 */
export function handshakeMillis(value: string): number | null {
  if (/^\d+$/.test(value)) return Number(value) * 1000;
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? null : parsed;
}

/** «12 s ago» for the last handshake, in the language of the window. */
export function formatHandshakeAge(value: string, now: number, lang: Lang): string {
  const at = handshakeMillis(value);
  if (at === null) return value;
  const seconds = Math.max(0, Math.round((now - at) / 1000));
  const relative = new Intl.RelativeTimeFormat(lang, { numeric: "always", style: "short" });
  if (seconds < 60) return relative.format(-seconds, "second");
  if (seconds < 3600) return relative.format(-Math.floor(seconds / 60), "minute");
  if (seconds < 86400) return relative.format(-Math.floor(seconds / 3600), "hour");
  return relative.format(-Math.floor(seconds / 86400), "day");
}

/** «2:05» for the time left to answer a question of the service. */
export function formatClock(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds));
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, "0")}`;
}
