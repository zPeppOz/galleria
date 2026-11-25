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
import { bodyText, closeApp, daemon, openApp, readState, TEN_SECONDS, waitForBody, type Browser } from "./support";

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

  // G8: the language changes from the menu, in the same window and the same WebDriver session, with no restart.
  it("language_switches_without_a_restart", async () => {
    daemon("stop");
    const b = await openApp();
    await waitForBody(b, START_THE_SERVICE, "the «Start the service» panel never appeared");
    const session = b.sessionId;
    const choose = async (label: string) => {
      await b.$("button=Menu").click();
      const option = await b.waitUntil(
        async () => {
          for (const item of await b.$$('[role="menuitemradio"]')) if ((await item.getText()) === label) return item;
          return false;
        },
        { timeout: TEN_SECONDS, interval: 250, timeoutMsg: `the menu never offered «${label}»` },
      );
      await option.click();
    };
    await choose("Italiano");
    await waitForBody(b, "Avvia il servizio", "the window did not switch to Italian");
    expect(await bodyText(b)).not.toContain("Start the service");
    await choose("English");
    await waitForBody(b, "Start the service", "the window did not switch back to English");
    expect(await bodyText(b)).not.toContain("Avvia il servizio");
    expect(b.sessionId).toBe(session);
  });

  // D32, Q25, S21: what the menu chose is in `state.toml` and the next run starts with it. A second app on the same
  // state file stands for the restart: the window is closed with its session, and a new one opened.
  it("language_and_theme_survive_a_restart", async () => {
    daemon("stop");
    const first = await openApp();
    await waitForBody(first, START_THE_SERVICE, "the «Start the service» panel never appeared");
    const choose = async (label: string) => {
      await first.$("button=Menu").click();
      const option = await first.waitUntil(
        async () => {
          for (const item of await first.$$('[role="menuitemradio"]')) if ((await item.getText()) === label) return item;
          return false;
        },
        { timeout: TEN_SECONDS, interval: 250, timeoutMsg: `the menu never offered «${label}»` },
      );
      await option.click();
    };
    // The opposite of what the system gives, so that «saved» cannot be mistaken for «followed the system».
    const before = await first.execute(() => document.documentElement.dataset.theme);
    const [label, theme, tomlTheme] = before === "light" ? ["Scuro", "dark", 'theme = "dark"'] : ["Chiaro", "light", 'theme = "light"'];
    await choose("Italiano");
    await choose(label);
    await first.waitUntil(async () => readState().includes('lang = "it"') && readState().includes(tomlTheme), {
      timeout: TEN_SECONDS,
      interval: 250,
      timeoutMsg: `the app did not write the choices to state.toml; it holds: ${readState()}`,
    });
    await closeApp();

    const second = await openApp(true);
    await waitForBody(second, "Avvia il servizio", "the new window did not start in Italian");
    expect(await second.execute(() => document.documentElement.dataset.theme)).toBe(theme);
    expect(await second.execute(() => document.documentElement.lang)).toBe("it");
  });

  // Q34, S23: the size of the window comes back. WebKitWebDriver sets the rectangle; the app writes it half a second
  // after the last resize (it may be killed with the session, without a close event).
  it("window_size_survives_a_restart", async () => {
    daemon("stop");
    const first = await openApp();
    await waitForBody(first, START_THE_SERVICE, "the «Start the service» panel never appeared");
    await first.setWindowSize(900, 620);
    await first.waitUntil(async () => /\[window\]/.test(readState()) && /width = 9\d\d/.test(readState()), {
      timeout: TEN_SECONDS,
      interval: 250,
      timeoutMsg: `the app did not write the size of the window; state.toml holds: ${readState()}`,
    });
    await closeApp();

    const second = await openApp(true);
    await waitForBody(second, START_THE_SERVICE, "the new window never rendered");
    const size = await second.getWindowSize();
    console.log(`S23: saved 900×620, the window opens at ${size.width}×${size.height}`);
    expect(Math.abs(size.width - 900)).toBeLessThanOrEqual(40);
    expect(Math.abs(size.height - 620)).toBeLessThanOrEqual(40);
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

// The colour tokens, read from tokens.css: the dark theme is `:root`, the light one overrides it.
const tokensCss = readFileSync(new URL("../src/design/tokens.css", import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
const tokenBlock = (selector: string) => {
  const start = tokensCss.indexOf(selector);
  return tokensCss.slice(tokensCss.indexOf("{", start) + 1, tokensCss.indexOf("}", start));
};
const readTokens = (text: string) => Object.fromEntries([...text.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)].map((m) => [m[1], m[2].trim()]));
const darkTokens = readTokens(tokenBlock(":root {"));
const themes = { dark: darkTokens, light: { ...darkTokens, ...readTokens(tokenBlock(':root[data-theme="light"]')) } } as const;

// T2: the colour tokens as the real window paints them (S8), and the theme on the root.
describe("design tokens in the real window", () => {
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

// T7: the shell with its sample data, in the real window (plan B2, U1 and G8). The daemon is the fake one: the shell does
// not ask it for anything, it only shows the link in the bar.
describe("the shell in the real window", () => {
  // Every `securitypolicyviolation` event from here on is kept; the tests below read them at the end.
  const watchCsp = (b: Browser) =>
    b.execute(() => {
      const w = window as unknown as { __csp: string[] };
      w.__csp = [];
      document.addEventListener("securitypolicyviolation", (e) => w.__csp.push(`${e.violatedDirective} ${e.blockedURI}`));
    });
  const cspViolations = (b: Browser) => b.execute(() => (window as unknown as { __csp: string[] }).__csp);

  const press = (b: Browser, ...keys: string[]) => b.keys(keys);
  const title = (b: Browser) => b.$("h1").getText();
  const activeRole = (b: Browser) => b.execute(() => document.activeElement?.getAttribute("role") ?? document.activeElement?.tagName.toLowerCase() ?? "");
  const waitFor = (b: Browser, check: () => Promise<boolean>, timeoutMsg: string) => b.waitUntil(check, { timeout: TEN_SECONDS, interval: 250, timeoutMsg });

  /** The first element matching `css` whose text (or whose first line, for a row of the list) is `text`. */
  const byText = (b: Browser, css: string, text: string) =>
    b.waitUntil(
      async () => {
        for (const el of await b.$$(css)) {
          const shown = (await el.getText()).trim();
          if (shown === text || shown.startsWith(`${text}\n`)) return el;
        }
        return false;
      },
      { timeout: TEN_SECONDS, interval: 250, timeoutMsg: `no ${css} with the text «${text}»` },
    );

  async function chooseFromMenu(b: Browser, label: string) {
    await b.$("button=Menu").click();
    await (await byText(b, '[role="menuitemradio"]', label)).click();
  }

  async function openShell(): Promise<Browser> {
    daemon("start");
    const b = await openApp();
    await waitForBody(b, CONNECTED, "the bar never showed the active link");
    await watchCsp(b);
    return b;
  }

  // R1: the rubrica is read from the disk, with the daemon stopped: the list is there next to the first-run panel.
  it("rubrica_lists_the_seeded_profiles", async () => {
    daemon("stop");
    const b = await openApp();
    await waitForBody(b, START_THE_SERVICE, "the «Start the service» panel never appeared");
    await waitFor(b, async () => (await b.$$('[role="listbox"] [role="option"]')).length === 5, "the list never showed the five profiles");
    const groups = await b.execute(() => [...document.querySelectorAll('[role="listbox"] [role="group"]')].map((g) => g.getAttribute("aria-label")));
    expect(groups).toEqual(["Lavoro", "Personale", "Clienti"]);
    // the server of the chosen profile is on the page, and the secrets of the seed are nowhere in it
    const text = await bodyText(b);
    expect(text).toContain("vpn.ufficio.example:1194");
    expect(text).not.toContain("SEED-SECRET");
    expect(await title(b)).toBe("Ufficio Napoli");
  });

  it("empty_rubrica_shows_the_import_hint", async () => {
    daemon("stop");
    const b = await openApp(false, "empty");
    await waitForBody(b, "No connections yet", "the empty rubrica did not say so");
    await waitForBody(b, "galleria import <file>", "the empty rubrica does not show the command to import a profile");
    expect(await b.$$('[role="option"]')).toHaveLength(0);
  });

  // R1, D32: the search narrows the list and the sort order chosen comes back after a restart. Every key goes through the
  // real window: the search field is typed in, the sort order is chosen with a click and read back from state.toml.
  it("search_filters_the_list_and_sort_order_survives_a_restart", async () => {
    daemon("stop");
    const first = await openApp();
    await waitForBody(first, START_THE_SERVICE, "the «Start the service» panel never appeared");
    const rows = async () => (await first.execute(() => [...document.querySelectorAll('[role="listbox"] [role="option"] span.profile-name')].map((el) => (el.textContent ?? "").trim()))) as string[];
    await waitFor(first, async () => (await rows()).length === 5, "the list never showed the five profiles");
    await first.$('input[type="search"]').setValue("milano");
    await waitFor(first, async () => (await rows()).join() === "Datacenter Milano", "the search did not narrow the list to «Datacenter Milano»");
    expect(await bodyText(first)).toContain("1 result");
    await first.keys(["Escape"]); // the field still has the focus after the typing: Esc empties it, like the user would
    await waitFor(first, async () => (await rows()).length === 5, "emptying the search did not bring the list back");
    await (await byText(first, "label.gx-segmented-option", "Most recent")).click();
    await first.waitUntil(async () => readState().includes('sort = "recent"'), { timeout: TEN_SECONDS, interval: 250, timeoutMsg: `the app did not write the sort order; state.toml holds: ${readState()}` });
    await closeApp();

    const second = await openApp(true);
    await waitForBody(second, START_THE_SERVICE, "the new window never rendered");
    const checked = await second.execute(() => (document.querySelector('input[type="radio"][value="recent"]') as HTMLInputElement | null)?.checked ?? null);
    expect(checked, "the sort order «Most recent» is the one chosen when the window opens again").toBe(true);
  });

  it("shell_is_navigable_with_the_seeded_rubrica", async () => {
    const b = await openShell();
    expect(await bodyText(b)).not.toContain("Sample data");
    const names: string[] = [];
    for (const row of await b.$$('[role="listbox"] [role="option"]')) names.push((await row.$("span.profile-name").getText()).trim());
    // the groups of the seed in their order, in each the favourites first and then by name
    expect(names).toEqual(["Ufficio Napoli", "Datacenter Milano", "Homelab", "VPS Hetzner", "Cliente Rossi"]);
    expect(await title(b)).toBe("Ufficio Napoli");

    // Menu, the search field, the sort order, then the list: one stop for the chosen row; ↓ moves the choice and the detail follows
    for (let step = 0; step < 4; step++) await press(b, "Tab");
    expect(await activeRole(b)).toBe("option");
    await press(b, "ArrowDown");
    await waitFor(b, async () => (await title(b)) === "Datacenter Milano", "↓ in the list did not change the title of the detail");
    await press(b, "End");
    await waitFor(b, async () => (await title(b)) === "Cliente Rossi", "End in the list did not choose the last profile");
    await press(b, "Home");
    await waitFor(b, async () => (await title(b)) === "Ufficio Napoli", "Home in the list did not choose the first profile");

    // «More actions», then the tab strip: the arrows open the tabs, down to «Log»
    await press(b, "Tab");
    await press(b, "Tab");
    expect(await activeRole(b)).toBe("tab");
    for (let step = 0; step < 4; step++) await press(b, "ArrowRight");
    const log = await byText(b, "[role=tab]", "Log");
    await waitFor(b, async () => (await log.getAttribute("aria-selected")) === "true", "the arrows did not reach the «Log» tab");
    expect(await bodyText(b)).toContain("This section arrives in a later release.");
    expect(await cspViolations(b)).toEqual([]);
  });

  it("shell_works_in_both_languages", async () => {
    const b = await openShell();
    expect(await bodyText(b)).toContain("More actions");
    await chooseFromMenu(b, "Italiano");
    await waitForBody(b, "Altre azioni", "the window did not switch to Italian");
    const italian = await bodyText(b);
    for (const text of ["Generale", "Altre azioni", "Collegato al servizio"]) expect(italian).toContain(text);
    expect(italian).not.toContain("More actions");
    expect(await b.execute(() => document.documentElement.lang)).toBe("it");
    await chooseFromMenu(b, "English");
    await waitForBody(b, "More actions", "the window did not switch back to English");
    expect(await bodyText(b)).toContain("General");
    expect(await bodyText(b)).not.toContain("Altre azioni");
    expect(await cspViolations(b)).toEqual([]);
  });

  it("both_themes_render", async () => {
    const b = await openShell();
    // A spot of the real page where nothing but the body background shows: the content area, away from any control.
    const spot = await b.execute(() => {
      const x = window.innerWidth - 24;
      for (let y = 260; y < window.innerHeight - 60; y += 8) {
        if (document.elementFromPoint(x, y)?.classList.contains("shell-main")) return { x, y, width: window.innerWidth };
      }
      return null;
    });
    expect(spot, "an empty spot in the content area").not.toBeNull();
    const seen: Record<string, Rgb> = {};
    for (const [theme, label] of [["light", "Light"], ["dark", "Dark"]] as const) {
      await chooseFromMenu(b, label);
      await waitFor(b, async () => (await b.execute(() => document.documentElement.dataset.theme)) === theme, `the theme did not become ${theme}`);
      await b.execute(() => new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => done()))));
      const png = PNG.sync.read(Buffer.from(await b.takeScreenshot(), "base64"));
      const scale = png.width / (spot as { width: number }).width;
      const at = (Math.round((spot as { y: number }).y * scale) * png.width + Math.round((spot as { x: number }).x * scale)) * 4;
      const got: Rgb = [png.data[at], png.data[at + 1], png.data[at + 2]];
      const want = parseColour(themes[theme]["--bg"]);
      expect(got.every((v, i) => Math.abs(v - want[i]) <= 3), `${theme}: the body paints ${toHex(got)}, the token --bg is ${toHex(want)}`).toBe(true);
      seen[theme] = got;
    }
    expect(toHex(seen.light)).not.toBe(toHex(seen.dark));
    expect(await cspViolations(b)).toEqual([]);
  });

  it("control_the_csp_listener_sees_a_violation", async () => {
    // Without this, «no violation» could only mean that the listener hears nothing: a <style> element is what the CSP refuses.
    const b = await openShell();
    await b.execute(() => {
      const style = document.createElement("style");
      style.textContent = "body { outline: 1px solid red }";
      document.head.append(style);
    });
    await waitFor(b, async () => (await cspViolations(b)).length > 0, "the listener heard no violation for an inline <style>");
    expect((await cspViolations(b))[0]).toMatch(/style-src|default-src/);
  });

  it("no_csp_violation_during_the_tour", async () => {
    const b = await openShell();
    // the list, the tabs (the indicator of the strip sets its size with a script, not with a style element), the menu,
    // the About dialog, the three actions that arrive later, the notice, both languages and both themes
    await (await byText(b, "[role=option] span.profile-name", "Homelab")).click();
    await waitFor(b, async () => (await title(b)) === "Homelab", "the list did not choose Homelab");
    for (const tab of await b.$$('[role="tab"]')) await tab.click();
    await (await byText(b, "[role=tab]", "General")).click();
    await b.$("button=More actions").click();
    await (await byText(b, "[role=menuitem]", "Delete")).click();
    await waitForBody(b, "This action arrives in a later release.", "the notice never appeared");
    await b.$("button=Menu").click();
    await (await byText(b, "[role=menuitem]", "About Galleria")).click();
    await b.$('[role="dialog"]').waitForDisplayed({ timeout: TEN_SECONDS });
    await press(b, "Escape");
    await chooseFromMenu(b, "Italiano");
    await waitForBody(b, "Altre azioni", "the window did not switch to Italian");
    await chooseFromMenu(b, "Scuro");
    await chooseFromMenu(b, "Chiaro");
    expect(await cspViolations(b)).toEqual([]);
  });
});
