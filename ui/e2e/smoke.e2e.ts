// WebDriver smoke test (level 3 of the spec §14): the real app in a real WebKitGTK window against a daemon that serves the real IPC (the `fake-daemon`
// example: the real `galleriad` server with a fake backend, S5). Run by lab/tests/gui-smoke.sh, which starts tauri-driver, Xvfb and the daemon and exports:
//   GALLERIA_SMOKE_APP     the built galleria-gui binary
//   GALLERIA_SMOKE_BIN     a directory with `daemon-start` and `daemon-stop` (the tests run them from Node, not
//                          from the webview)
//   GALLERIA_SMOKE_DRIVER_PORT  the port tauri-driver listens on
// Nothing here asks the daemon to connect: no route, DNS or interface is touched.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { PNG } from "pngjs";
import { afterEach, describe, expect, it } from "vitest";
import { parseColour, tintOver, toHex, type Rgb } from "../src/design/color";
import { closeApp, daemon, openApp, TEN_SECONDS, waitForBody, type Browser } from "./support";

const CONNECTED = /Connected to the service|Collegato al servizio/;
const DISCONNECTED = /Disconnected|Disconnesso/;
const START_THE_SERVICE = /Start the service|Avvia il servizio/;
const SYSTEMCTL = "sudo systemctl enable --now galleriad";

afterEach(closeApp);

describe("galleria-gui against the real daemon", () => {
  it("shows_the_real_daemon_state", async () => {
    daemon("start");
    const b = await openApp();
    await waitForBody(b, DISCONNECTED, "the window never showed the daemon state «Disconnected»");
    await waitForBody(b, CONNECTED, "the bar never showed the active link");
  });

  it("relinks_after_a_daemon_restart", async () => {
    daemon("start");
    const b = await openApp();
    await waitForBody(b, CONNECTED, "no link at the start");
    daemon("stop");
    await waitForBody(b, "systemctl", "the window never showed the command to start the service");
    daemon("start");
    await waitForBody(b, DISCONNECTED, "the window did not relink by itself after the daemon came back");
    await waitForBody(b, CONNECTED, "the bar did not show the link after the restart");
  });

  it("first_run_without_daemon", async () => {
    daemon("stop");
    const b = await openApp();
    await waitForBody(b, START_THE_SERVICE, "the «Start the service» panel never appeared");
    await waitForBody(b, SYSTEMCTL, "the panel does not show the command");
  });

  it("window_has_no_remote_content", async () => {
    daemon("stop");
    const b = await openApp();
    await waitForBody(b, START_THE_SERVICE, "the window never rendered");
    const remoteRefs = await b.execute(
      () => document.querySelectorAll('script[src^="http"],link[href^="http"],img[src^="http"]').length,
    );
    expect(remoteRefs).toBe(0);
  });

  // S7: «Copy» must fill the system clipboard from the real WebKitGTK webview. Read back with xclip under Xvfb.
  it("copy_fills_the_system_clipboard", async () => {
    daemon("stop");
    const b = await openApp();
    await waitForBody(b, START_THE_SERVICE, "the window never rendered");
    await b.$("button=Copy").click().catch(async () => b.$("button=Copia").click());
    await b.waitUntil(
      async () => {
        try {
          return execFileSync("xclip", ["-o", "-selection", "clipboard"], { encoding: "utf8" }) === SYSTEMCTL;
        } catch {
          return false;
        }
      },
      { timeout: TEN_SECONDS, interval: 250, timeoutMsg: "the clipboard did not receive the command" },
    );
  });
});

// T2: the colour tokens as the real window paints them (S8), and the theme on the root.
describe("design tokens in the real window", () => {
  const css = readFileSync(new URL("../src/design/tokens.css", import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  const block = (selector: string) => {
    const start = css.indexOf(selector);
    return css.slice(css.indexOf("{", start) + 1, css.indexOf("}", start));
  };
  const read = (text: string) => Object.fromEntries([...text.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)].map((m) => [m[1], m[2].trim()]));
  const dark = read(block(":root {"));
  const themes = { dark, light: { ...dark, ...read(block(':root[data-theme="light"]')) } } as const;
  const COLOURS = ["--bg", "--panel", "--panel-2", "--line", "--line-strong", "--ink", "--muted", "--faint", "--accent", "--accent-ink", "--ok", "--warn", "--err", "--wg", "--ovpn"];
  const TINTED = ["--ok", "--warn", "--err", "--wg", "--ovpn"];

  async function paint(b: Browser, theme: string, background: string, inner?: string): Promise<Rgb> {
    await b.execute(
      (themeName: string, outer: string, tint: string | undefined) => {
        document.documentElement.dataset.theme = themeName;
        document.getElementById("probe")?.remove();
        const box = document.createElement("div");
        box.id = "probe";
        Object.assign(box.style, { position: "absolute", top: "0", left: "0", width: "40px", height: "40px", zIndex: "99999", background: outer });
        if (tint) {
          const child = document.createElement("div");
          Object.assign(child.style, { width: "100%", height: "100%", background: tint });
          box.append(child);
        }
        document.body.append(box);
      },
      theme,
      background,
      inner,
    );
    // Wait for two frames, so the box is painted before the window is photographed. WebKitWebDriver's element
    // screenshot returns the whole page (checked: 1180×740 for a 60×60 box), so the page is photographed and the pixel
    // in the middle of the box, which sits at the top left corner, is read.
    await b.execute(() => new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => done()))));
    const png = PNG.sync.read(Buffer.from(await b.takeScreenshot(), "base64"));
    const at = (20 * png.width + 20) * 4;
    return [png.data[at], png.data[at + 1], png.data[at + 2]];
  }

  let worst = 0;
  const near = (got: Rgb, want: Rgb, tolerance: number) => {
    worst = Math.max(worst, ...got.map((v, i) => Math.abs(v - want[i])));
    return got.every((v, i) => Math.abs(v - want[i]) <= tolerance);
  };
  const fmt = (c: Rgb) => `${toHex(c)} (${c.map((v) => Math.round(v)).join(",")})`;

  it("tokens_paint_the_expected_srgb", async () => {
    daemon("stop");
    const b = await openApp();
    await waitForBody(b, START_THE_SERVICE, "the window never rendered");
    const wrong: string[] = [];
    for (const theme of ["dark", "light"] as const) {
      for (const token of COLOURS) {
        const want = parseColour(themes[theme][token]);
        const got = await paint(b, theme, `var(${token})`);
        if (!near(got, want, 3)) wrong.push(`${theme} ${token}: painted ${fmt(got)}, expected ${fmt(want)}`);
      }
      for (const token of TINTED) {
        const surface = "--panel";
        const want = tintOver(parseColour(themes[theme][token]), 0.14, parseColour(themes[theme][surface]));
        const got = await paint(b, theme, `var(${surface})`, `color-mix(in oklch, var(${token}) var(--tint), transparent)`);
        if (!near(got, want, 3)) wrong.push(`${theme} tint of ${token} over ${surface}: painted ${fmt(got)}, expected ${fmt(want)}`);
      }
    }
    console.log(`S8: ${COLOURS.length + TINTED.length} colours in 2 themes painted, largest difference from the expected sRGB: ${worst} per channel`);
    expect(wrong).toEqual([]);
  });

  it("theme_attribute_is_set_before_first_paint", async () => {
    daemon("stop");
    const b = await openApp();
    await waitForBody(b, START_THE_SERVICE, "the window never rendered");
    expect(["dark", "light"]).toContain(await b.execute(() => document.documentElement.dataset.theme));
  });
});
