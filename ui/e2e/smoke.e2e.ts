// WebDriver smoke test (level 3 of the spec §14): the real app in a real WebKitGTK window against a daemon that serves the real IPC (the `fake-daemon`
// example: the real `galleriad` server with a fake backend, S5). Run by lab/tests/gui-smoke.sh, which starts tauri-driver, Xvfb and the daemon and exports:
//   GALLERIA_SMOKE_APP     the built galleria-gui binary
//   GALLERIA_SMOKE_BIN     a directory with `daemon-start` and `daemon-stop` (the tests run them from Node, not
//                          from the webview)
//   GALLERIA_SMOKE_DRIVER_PORT  the port tauri-driver listens on
// Nothing here asks the daemon to connect: no route, DNS or interface is touched.
import { execFileSync, spawn, spawnSync, type ChildProcess } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { PNG } from "pngjs";
import { afterEach, describe, expect, it } from "vitest";
import { parseColour, tintOver, toHex, type Rgb } from "../src/design/color";
import { bodyText, closeApp, daemon, env, importFixtures, knownUpdateState, openApp, openAppWithState, readState, TEN_SECONDS, waitForBody, type Browser } from "./support";

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

  // T5 (R1): a profile is edited in the real window and saved to the disk. The name is typed in, «Save» writes the file of
  // the phase 1 (read back here from the folder of the run), the list and the title follow, and the notice says it.
  it("a_profile_is_renamed_and_the_list_follows", async () => {
    const b = await openShell();
    const rows = async () => (await b.execute(() => [...document.querySelectorAll('[role="listbox"] [role="option"] span.profile-name')].map((el) => (el.textContent ?? "").trim()))) as string[];
    await waitFor(b, async () => (await rows()).length === 5, "the list never showed the five profiles");
    const field = await b.$('input[name="name"]');
    await field.click();
    await press(b, "Control", "a"); // select the name; the next keys replace it (the driver's clearValue does not fire `input`)
    await press(b, ...Array.from("Sede Napoli"));
    await waitForBody(b, "You have changes that are not saved.", "the save bar never appeared");
    expect((await rows())[0], "nothing is written before «Save»").toBe("Ufficio Napoli");
    await b.$("button=Save").click();
    await waitFor(b, async () => (await rows()).includes("Sede Napoli"), "the list did not follow the new name");
    expect(await title(b)).toBe("Sede Napoli");
    await waitForBody(b, "Profile saved.", "the notice of the save never showed");
    expect(await bodyText(b)).not.toContain("You have changes that are not saved.");
    // the file on the disk has it, and it is still a private file of the phase 1
    const dir = join(env("XDG_CONFIG_HOME"), "galleria", "profiles");
    const saved = readdirSync(dir).map((name) => readFileSync(join(dir, name), "utf8")).filter((text) => text.includes('name = "Sede Napoli"'));
    expect(saved, "one profile file holds the new name").toHaveLength(1);
    expect(saved[0]).toContain("schema_version = 1");
    // a switch writes at once, with no bar and no notice
    await (await byText(b, "[role=tab]", "Network")).click();
    await (await b.$$('button[role="switch"]'))[0].click(); // «Ignore the routes the server pushes», the first of the two
    await waitFor(b, async () => readdirSync(dir).map((name) => readFileSync(join(dir, name), "utf8")).some((text) => text.includes('name = "Sede Napoli"') && text.includes("ignore_pushed_routes = true")), "the switch did not write the profile");
    expect(await bodyText(b)).not.toContain("You have changes that are not saved.");
    expect(await cspViolations(b)).toEqual([]);
  });

  // T6 (R1, Q30): the groups are managed from the window and kept in `galleria.toml`. A group is made and renamed, a group
  // with profiles in it is renamed (the profiles follow, on the disk too), and all of it is there after a restart.
  it("a_group_is_created_renamed_and_survives_a_restart", async () => {
    const config = (file: string) => join(env("XDG_CONFIG_HOME"), "galleria", file);
    const profileFiles = () => readdirSync(config("profiles")).map((name) => readFileSync(join(config("profiles"), name), "utf8"));
    const first = await openShell();
    const listGroups = (b: Browser) => b.execute(() => [...document.querySelectorAll('[role="listbox"] [role="group"]')].map((g) => g.getAttribute("aria-label"))) as Promise<string[]>;
    await waitFor(first, async () => (await listGroups(first)).length === 3, "the list never showed the three groups");
    await first.$("button=Manage groups").click();
    await first.$('input[name="new-group"]').waitForDisplayed({ timeout: TEN_SECONDS });
    // a new group goes to the end of the list and the file
    await first.$('input[name="new-group"]').click();
    await press(first, ...Array.from("Casa"), "Enter");
    await first.$('input[name="group:Casa"]').waitForDisplayed({ timeout: TEN_SECONDS });
    await waitFor(first, async () => readFileSync(config("galleria.toml"), "utf8").includes('groups = ["Lavoro", "Personale", "Clienti", "Casa"]'), "the new group is not in galleria.toml");
    // and it is renamed in its place (select the name, type the new one, Enter)
    await first.$('input[name="group:Casa"]').click();
    await press(first, "Control", "a");
    await press(first, ...Array.from("Famiglia"), "Enter");
    await first.$('input[name="group:Famiglia"]').waitForDisplayed({ timeout: TEN_SECONDS });
    // a group with a profile in it: «Clienti» has «Cliente Rossi»
    await first.$('input[name="group:Clienti"]').click();
    await press(first, "Control", "a");
    await press(first, ...Array.from("Clienti VIP"), "Enter");
    await first.$('input[name="group:Clienti VIP"]').waitForDisplayed({ timeout: TEN_SECONDS });
    await waitFor(first, async () => readFileSync(config("galleria.toml"), "utf8").includes('groups = ["Lavoro", "Personale", "Clienti VIP", "Famiglia"]'), `galleria.toml holds: ${readFileSync(config("galleria.toml"), "utf8")}`);
    expect(profileFiles().filter((text) => text.includes('group = "Clienti VIP"')), "the profile of the group moved with it").toHaveLength(1);
    expect(profileFiles().some((text) => text.includes('group = "Clienti"\n')), "no profile is left in the old name").toBe(false);
    await first.keys(["Escape"]);
    await waitFor(first, async () => (await listGroups(first)).join() === "Lavoro,Personale,Clienti VIP,Famiglia", "the list does not show the groups as they are now");
    await closeApp();

    const second = await openApp(true);
    await waitForBody(second, CONNECTED, "the new window never showed the link");
    await waitFor(second, async () => (await listGroups(second)).join() === "Lavoro,Personale,Clienti VIP,Famiglia", "the groups did not come back after the restart");
  });

  // T7 (Q32, R1): the editor of the raw text refuses what the phase 1 refuses, in the real window, and writes nothing. A
  // script directive typed under the last line is listed with its line, the focus goes to the list, the file does not change.
  it("a_script_directive_cannot_be_saved_from_the_editor", async () => {
    const dir = join(env("XDG_CONFIG_HOME"), "galleria", "profiles");
    const onDisk = () => readdirSync(dir).sort().map((name) => readFileSync(join(dir, name), "utf8")).join("\n---\n");
    const b = await openShell();
    await waitFor(b, async () => (await b.$$('[role="listbox"] [role="option"]')).length === 5, "the list never showed the five profiles");
    const before = onDisk();
    await (await byText(b, "[role=tab]", "Configuration")).click();
    const area = await b.$('textarea[name="openvpn_config"]');
    await area.waitForDisplayed({ timeout: TEN_SECONDS });
    await area.click();
    await press(b, "Control", "End");
    await press(b, "Enter", ...Array.from("up /tmp/x.sh"));
    await waitForBody(b, "You have changes that are not saved.", "the save bar never appeared");
    // the list shows the finding by itself, 400 ms after the last key
    await waitForBody(b, "Galleria does not run the scripts of profiles.", "the editor never listed the script directive");
    await b.$("button=Save").click();
    await waitFor(b, async () => ((await b.execute(() => document.activeElement?.textContent ?? "")) as string).includes("· up ·"), "the focus did not go to the first problem of the list");
    expect(await bodyText(b)).not.toContain("Profile saved.");
    expect(onDisk(), "a refused save leaves the files as they were").toBe(before);
    expect(await cspViolations(b)).toEqual([]);
  });

  // T7 (S24): what is typed is what is saved, comments and all, and the phase 1 still reads the file.
  it("a_comment_typed_in_the_editor_is_saved_as_written", async () => {
    const dir = join(env("XDG_CONFIG_HOME"), "galleria", "profiles");
    const b = await openShell();
    await waitFor(b, async () => (await b.$$('[role="listbox"] [role="option"]')).length === 5, "the list never showed the five profiles");
    await (await byText(b, "[role=tab]", "Configuration")).click();
    const area = await b.$('textarea[name="openvpn_config"]');
    await area.waitForDisplayed({ timeout: TEN_SECONDS });
    await area.click();
    await press(b, "Control", "Home");
    // short, with no doubled letter: the driver's key sequence drops a repeated key and some of the spaces
    await press(b, ...Array.from("# gateway"), "Enter");
    await b.$("button=Save").click();
    await waitForBody(b, "Profile saved.", "the notice of the save never showed");
    await waitFor(b, async () => readdirSync(dir).some((name) => readFileSync(join(dir, name), "utf8").includes("# gateway")), `the comment is not in the file; the office profile holds: ${readdirSync(dir).map((name) => readFileSync(join(dir, name), "utf8")).find((text) => text.includes("Ufficio")) ?? "(no file)"}`);
    expect(await bodyText(b)).not.toContain("You have changes that are not saved.");
  });

  // R1, R6: what the window writes is what the phase 1 reads. A profile is renamed and copied in the window; the `galleria`
  // binary of the phase 1 (built in the image) lists them, with their new names, and nothing goes wrong.
  const runCli = (...args: string[]) => execFileSync(env("GALLERIA_SMOKE_CLI"), args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  const rowNames = async (b: Browser) => (await b.execute(() => [...document.querySelectorAll('[role="listbox"] [role="option"] span.profile-name')].map((el) => (el.textContent ?? "").trim()))) as string[];
  async function renameFirstProfile(b: Browser, name: string) {
    await waitFor(b, async () => (await rowNames(b)).length === 5, "the list never showed the five profiles");
    await b.$('input[name="name"]').click();
    await press(b, "Control", "a");
    await press(b, ...Array.from(name));
    await b.$("button=Save").click();
    await waitForBody(b, "Profile saved.", "the notice of the save never showed");
  }
  async function duplicateFromTheMenu(b: Browser) {
    await b.$("button=More actions").click();
    await (await byText(b, "[role=menuitem]", "Duplicate")).click();
    await waitForBody(b, "Duplicated as", "the notice of the copy never showed");
  }

  it("cli_lists_what_the_gui_saved", async () => {
    const b = await openShell();
    await renameFirstProfile(b, "Sede");
    await duplicateFromTheMenu(b);
    await waitFor(b, async () => (await rowNames(b)).includes("Sede (copy)"), "the list did not show the copy");
    const listed = runCli("list");
    const names = listed.trim().split("\n").map((line) => line.split(" ").slice(1).join(" ").replace(/ "?(Open[Vv]pn|Wire[Gg]uard)"?$/, ""));
    expect(names.sort(), `galleria list says: ${listed}`).toEqual(["Cliente Rossi", "Datacenter Milano", "Homelab", "Sede", "Sede (copy)", "VPS Hetzner"]);
    expect(await cspViolations(b)).toEqual([]);
  });

  // R1, §5: the rubrica is read and written with the daemon stopped. The first-run panel is there, and so is the list: a
  // profile is renamed, copied and deleted, and the files on the disk follow.
  it("rubrica_works_with_the_daemon_stopped", async () => {
    daemon("stop");
    const b = await openApp();
    await waitForBody(b, START_THE_SERVICE, "the «Start the service» panel never appeared");
    // the button of the connection is there and is off: without the service there is nothing to connect to (B5, T5)
    const connect = await b.$("button=Connect");
    expect(await connect.isExisting(), "the button «Connect» is on the page").toBe(true);
    expect(await connect.isEnabled(), "and it is off").toBe(false);
    await renameFirstProfile(b, "Sede");
    expect(await title(b)).toBe("Sede");
    await duplicateFromTheMenu(b);
    await waitFor(b, async () => (await rowNames(b)).length === 6, "the copy is not in the list");
    expect(await title(b)).toBe("Sede (copy)");
    // delete the copy: the window asks, the answer «Delete» removes it, and the focus stays in the list
    await b.$("button=More actions").click();
    await (await byText(b, "[role=menuitem]", "Delete")).click();
    await waitForBody(b, "Delete Sede (copy)?", "the confirmation never appeared");
    await (await byText(b, '[role="alertdialog"] button', "Delete")).click();
    await waitFor(b, async () => (await rowNames(b)).length === 5, "the copy is still in the list");
    await waitForBody(b, "Deleted", "the notice of the delete never showed");
    expect(await activeRole(b), "the focus after the delete is in the list").toBe("option");
    const dir = join(env("XDG_CONFIG_HOME"), "galleria", "profiles");
    const files = readdirSync(dir).map((name) => readFileSync(join(dir, name), "utf8"));
    expect(files).toHaveLength(5);
    expect(files.filter((text) => text.includes('name = "Sede"')), "the renamed one is on the disk").toHaveLength(1);
    expect(files.some((text) => text.includes("(copy)")), "the copy is gone from the disk").toBe(false);
    expect(await bodyText(b)).toContain("Start the service");
  });

  it("no_csp_violation_during_the_rubrica_tour", async () => {
    const b = await openShell();
    // search, the raw editor, a save, the groups dialog, a copy, the export that asks and the delete that asks
    await b.$('input[type="search"]').setValue("milano");
    await waitFor(b, async () => (await rowNames(b)).join() === "Datacenter Milano", "the search did not narrow the list");
    await press(b, "Escape");
    await waitFor(b, async () => (await rowNames(b)).length === 5, "the list did not come back");
    await (await byText(b, "[role=tab]", "Configuration")).click();
    const area = await b.$('textarea[name="openvpn_config"]');
    await area.waitForDisplayed({ timeout: TEN_SECONDS });
    await area.click();
    await press(b, "Control", "Home");
    await press(b, ...Array.from("# tour"), "Enter");
    await b.$("button=Save").click();
    await waitForBody(b, "Profile saved.", "the notice of the save never showed");
    await b.$("button=Manage groups").click();
    await b.$('input[name="new-group"]').waitForDisplayed({ timeout: TEN_SECONDS });
    await press(b, "Escape");
    await waitFor(b, async () => (await b.$$('[role="dialog"]')).length === 0, "the groups dialog did not close");
    await duplicateFromTheMenu(b);
    await b.$("button=More actions").click();
    await (await byText(b, "[role=menuitem]", "Export")).click();
    await waitForBody(b, "The file will contain the private keys", "the export did not ask first");
    await press(b, "Escape");
    await b.$("button=More actions").click();
    await (await byText(b, "[role=menuitem]", "Delete")).click();
    await waitForBody(b, "This cannot be undone", "the delete did not ask first");
    await (await byText(b, '[role="alertdialog"] button', "Delete")).click();
    await waitForBody(b, "Deleted", "the notice of the delete never showed");
    expect(await cspViolations(b)).toEqual([]);
  });

  it("shell_is_navigable_with_the_seeded_rubrica", async () => {
    const b = await openShell();
    expect(await bodyText(b)).not.toContain("Sample data");
    const names: string[] = [];
    for (const row of await b.$$('[role="listbox"] [role="option"]')) names.push((await row.$("span.profile-name").getText()).trim());
    // the groups of the seed in their order, in each the favourites first and then by name
    expect(names).toEqual(["Ufficio Napoli", "Datacenter Milano", "Homelab", "VPS Hetzner", "Cliente Rossi"]);
    expect(await title(b)).toBe("Ufficio Napoli");

    // Import…, Menu, the search field, the sort order, then the list: one stop for the chosen row; ↓ moves the choice and the detail follows
    for (let step = 0; step < 5; step++) await press(b, "Tab");
    expect(await activeRole(b)).toBe("option");
    await press(b, "ArrowDown");
    await waitFor(b, async () => (await title(b)) === "Datacenter Milano", "↓ in the list did not change the title of the detail");
    await press(b, "End");
    await waitFor(b, async () => (await title(b)) === "Cliente Rossi", "End in the list did not choose the last profile");
    await press(b, "Home");
    await waitFor(b, async () => (await title(b)) === "Ufficio Napoli", "Home in the list did not choose the first profile");

    // «Manage groups», the star, «Connect» (B5), «More actions», then the tab strip: the arrows open the tabs, down to «Log»
    for (let step = 0; step < 5; step++) await press(b, "Tab");
    expect(await activeRole(b)).toBe("tab");
    for (let step = 0; step < 4; step++) await press(b, "ArrowRight");
    const log = await byText(b, "[role=tab]", "Log");
    await waitFor(b, async () => (await log.getAttribute("aria-selected")) === "true", "the arrows did not reach the «Log» tab");
    expect(await bodyText(b)).toContain("No session for this profile.");
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
    await waitForBody(b, "Delete Homelab?", "the confirmation never appeared");
    await press(b, "Escape");
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

  // Plan B6, T5: the notice of a newer version, its «Copy link», the line of «About» and the switch in the menu draw with the CSP of the window. A
  // version is already known and the day has not passed, so no request is made: the check is the only thing here that could reach the network.
  it("no_csp_violation_with_the_update_notice", async () => {
    daemon("start");
    const b = await openAppWithState(knownUpdateState());
    await waitForBody(b, CONNECTED, "the bar never showed the active link");
    await watchCsp(b);
    await waitForBody(b, "Galleria 9.9.9 is available", "the notice never showed");
    await b.$("button=Copy link").click();
    await waitForBody(b, "Link copied.", "the notice of the copy never showed");
    await b.$("button=Menu").click();
    expect(await (await byText(b, '[role="menuitemcheckbox"]', "Check for a newer version")).getAttribute("aria-checked")).toBe("true");
    await (await byText(b, "[role=menuitem]", "About Galleria")).click();
    await waitForBody(b, "Latest known version", "the line of the latest version never showed in About");
    await press(b, "Escape");
    // Italian, then the dismissal
    await b.$("button=Menu").click();
    await (await byText(b, '[role="menuitemradio"]', "Italiano")).click();
    await waitForBody(b, "È disponibile Galleria 9.9.9", "the notice did not follow the language");
    await b.$("button=Non ricordarmelo").click();
    await waitFor(b, async () => !(await bodyText(b)).includes("È disponibile Galleria"), "the notice stayed after «Non ricordarmelo»");
    expect(readState()).toContain('dismissed = "9.9.9"');
    expect(await cspViolations(b)).toEqual([]);
  });
});

// T2 (plan B4, I1, I2): a profile is pasted as text in the real window. The text is put in the field in one go with
// the setter of the browser and an `input` event, which is what a paste does (the keyboard of WebDriver loses letters).
describe("the import in the real window", () => {
  const watchCsp = (b: Browser) =>
    b.execute(() => {
      const w = window as unknown as { __csp: string[] };
      w.__csp = [];
      document.addEventListener("securitypolicyviolation", (e) => w.__csp.push(`${e.violatedDirective} ${e.blockedURI}`));
    });
  const cspViolations = (b: Browser) => b.execute(() => (window as unknown as { __csp: string[] }).__csp);
  const waitFor = (b: Browser, check: () => Promise<boolean>, timeoutMsg: string) => b.waitUntil(check, { timeout: TEN_SECONDS, interval: 250, timeoutMsg });
  const rowNames = async (b: Browser) => (await b.execute(() => [...document.querySelectorAll('[role="listbox"] [role="option"] span.profile-name')].map((el) => (el.textContent ?? "").trim()))) as string[];
  const runCli = (...args: string[]) => execFileSync(env("GALLERIA_SMOKE_CLI"), args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  const configDir = () => join(env("XDG_CONFIG_HOME"), "galleria");

  /** Every file under the folder of the profiles, with its bytes: what must stay as it is until «Import». */
  const onDisk = (): string => {
    const walk = (dir: string): string[] =>
      readdirSync(dir, { withFileTypes: true })
        .sort((a, b) => a.name.localeCompare(b.name))
        .flatMap((entry) => (entry.isDirectory() ? walk(join(dir, entry.name)) : [`${join(dir, entry.name)}\n${readFileSync(join(dir, entry.name), "utf8")}`]));
    return walk(configDir()).join("\n---\n");
  };

  async function openShell(rubrica: "seeded" | "empty" = "seeded"): Promise<Browser> {
    daemon("start");
    const b = await openApp(false, rubrica);
    await waitForBody(b, CONNECTED, "the bar never showed the active link");
    await watchCsp(b);
    return b;
  }

  /** Opens the window of the import with the button of the bar and pastes the text of the file. */
  async function pasteFile(b: Browser, path: string): Promise<void> {
    await b.$("button=Import…").click();
    const field = await b.$('textarea[name="import-text"]');
    await field.waitForDisplayed({ timeout: TEN_SECONDS });
    await b.execute(
      (selector: string, value: string) => {
        const el = document.querySelector(selector) as HTMLTextAreaElement;
        const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;
        setter?.call(el, value);
        el.dispatchEvent(new Event("input", { bubbles: true }));
      },
      'textarea[name="import-text"]',
      readFileSync(path, "utf8"),
    );
    await b.$("button=Preview").click();
    await b.$('input[name="import-name"]').waitForDisplayed({ timeout: TEN_SECONDS });
  }

  it("a_profile_pasted_as_text_is_imported", async () => {
    const files = importFixtures();
    const b = await openShell();
    await waitFor(b, async () => (await rowNames(b)).length === 5, "the list never showed the five profiles");
    await pasteFile(b, files.inline);
    const shown = await bodyText(b);
    expect(shown).toContain("Found: private key, TLS encryption key. They will be saved in the system keyring.");
    expect(shown, "the secrets are named and never shown").not.toContain("SEED-SECRET-");
    await b.$("button=Import").click();
    await waitForBody(b, "Imported “Imported profile”.", "the notice of the import never showed");
    await waitFor(b, async () => (await rowNames(b)).includes("Imported profile"), "the list did not show the new profile");
    expect(await b.$("h1").getText()).toBe("Imported profile");
    // the files are on the disk, private, with the secrets in the archive
    const profiles = readdirSync(join(configDir(), "profiles")).map((name) => readFileSync(join(configDir(), "profiles", name), "utf8"));
    expect(profiles).toHaveLength(6);
    expect(profiles.filter((text) => text.includes('name = "Imported profile"'))).toHaveLength(1);
    // the keys went to the keyring: the folder of the secrets holds only the five of the seed
    expect(readdirSync(join(configDir(), "secrets"))).toHaveLength(5);
    // and the CLI of the phase 1 lists it
    expect(runCli("list")).toContain("Imported profile");
    expect(await cspViolations(b)).toEqual([]);
  });

  it("an_imported_profile_lands_in_the_keyring", async () => {
    const files = importFixtures();
    const b = await openShell();
    await waitFor(b, async () => (await rowNames(b)).length === 5, "the list never showed the five profiles");
    await pasteFile(b, files.inline);
    await b.$("button=Import").click();
    await waitForBody(b, "Imported “Imported profile”.", "the notice of the import never showed");
    // the profile is on the disk, and its keys are in the keyring, read back the way another program would
    const dir = join(configDir(), "profiles");
    const file = readdirSync(dir).find((name) => readFileSync(join(dir, name), "utf8").includes('name = "Imported profile"'));
    expect(file, "the file of the imported profile").toBeTruthy();
    const id = file!.replace(/\.toml$/, "");
    const found = spawnSync("secret-tool", ["search", "--all", "service", "galleria", "profile", id], { encoding: "utf8" });
    const text = `${found.stdout}${found.stderr}`;
    expect(text, "the keys of the profile are in the keyring").toContain("attribute.key = openvpn.block.key");
    expect(text).toContain("attribute.key = openvpn.block.tls-crypt");
    expect(text).toContain("label = Galleria: Imported profile (openvpn.block.key)");
    // and not in a file: the folder of the secrets still holds the five of the seed
    expect(readdirSync(join(configDir(), "secrets"))).toHaveLength(5);
    expect(readdirSync(join(configDir(), "secrets"))).not.toContain(file);
  });

  it("without_a_keyring_the_window_says_so_and_writes_the_file", async () => {
    const files = importFixtures();
    const b = await openApp(false, "seeded", [], "none");
    await waitForBody(b, "No system keyring: the keys are saved in a file only you can read.", "the notice of the fallback never showed");
    await waitFor(b, async () => (await rowNames(b)).length === 5, "the list never showed the five profiles");
    await pasteFile(b, files.inline);
    expect(await bodyText(b)).toContain("They will be saved in a file only you can read.");
    await b.$("button=Import").click();
    await waitForBody(b, "Imported “Imported profile”.", "the notice of the import never showed");
    // the notice is still there, and the keys are in the file
    expect(await bodyText(b)).toContain("No system keyring: the keys are saved in a file only you can read.");
    expect(readdirSync(join(configDir(), "secrets"))).toHaveLength(6);
  });

  it("nothing_is_written_before_import", async () => {
    const files = importFixtures();
    const b = await openShell();
    await waitFor(b, async () => (await rowNames(b)).length === 5, "the list never showed the five profiles");
    const before = onDisk();
    await pasteFile(b, files.inline);
    expect(onDisk(), "the preview is in memory: the folder is as it was").toBe(before);
    await b.$("button=Cancel").click();
    await waitFor(b, async () => (await b.$$('[role="dialog"]')).length === 0, "the window did not close");
    expect(onDisk(), "after «Cancel» the folder is as it was").toBe(before);
    expect(await rowNames(b)).toHaveLength(5);
  });

  it("the_malicious_profile_shows_the_d19_text_and_runs_nothing", async () => {
    const files = importFixtures();
    const b = await openShell();
    await pasteFile(b, files.malicious);
    const items = (await b.execute(() => [...document.querySelectorAll(".import-notice")].map((el) => (el.textContent ?? "").trim()))) as string[];
    const script = "Galleria does not run the scripts of profiles. If the profile depends on it, it may not work fully.";
    expect(items.find((line) => line.includes(" up ")), `the notices were ${JSON.stringify(items)}`).toContain(script);
    expect(items.find((line) => line.includes(" plugin "))).toContain(script);
    // the five directives the import of the phase 1 reports; the ones of the service (`script-security`, `management`…) go without a word
    expect(items).toHaveLength(5);
    await b.$("button=Import").click();
    await waitForBody(b, "Imported “Imported profile”. 5 directives removed.", "the notice with the number of directives never showed");
    // nothing was run: the script of the profile would have made these files
    expect(readdirSync("/tmp").filter((name) => name.startsWith("PWNED-"))).toEqual([]);
    // what was saved has none of the dropped lines
    const saved = readdirSync(join(configDir(), "profiles")).map((name) => readFileSync(join(configDir(), "profiles", name), "utf8")).find((text) => text.includes('name = "Imported profile"')) ?? "";
    expect(saved).not.toMatch(/PWNED|script-security|plugin/);
    expect(await cspViolations(b)).toEqual([]);
  });

  it("empty_rubrica_offers_the_import_button", async () => {
    daemon("stop");
    const b = await openApp(false, "empty");
    await waitForBody(b, "No connections yet", "the empty rubrica did not say so");
    await waitForBody(b, "Import a profile…", "the empty rubrica does not offer the import");
    expect(await bodyText(b), "no command to copy: the window is enough").not.toMatch(/galleria\s+import/);
    expect(await b.$$('[role="option"]')).toHaveLength(0);
  });
});

describe("a file given to the app, and one window only (plan B4, T5)", () => {
  const app = () => env("GALLERIA_SMOKE_APP");
  const configDir = () => join(env("XDG_CONFIG_HOME"), "galleria");
  /** The processes of the app now running: a second launch must not add one. */
  const running = (): string[] => {
    const found = spawnSync("pgrep", ["-x", "galleria-gui"], { encoding: "utf8" });
    return found.stdout.split("\n").filter(Boolean).sort();
  };
  const alive = (child: ChildProcess) => child.exitCode === null && child.signalCode === null;
  const settle = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

  it("the_app_started_with_a_file_opens_its_preview", async () => {
    daemon("start");
    const files = importFixtures();
    // the file is an argument of the process, as a file manager gives it (S30)
    const b = await openApp(false, "seeded", [files.inline]);
    await waitForBody(b, "File: inline.ovpn", "the preview of the file of the arguments never showed");
    // at the second step, with the name taken from the file, and nothing on the disk yet
    expect(await b.$('input[name="import-name"]').getValue()).toBe("inline");
    expect(readdirSync(join(configDir(), "profiles"))).toHaveLength(5);
    expect(await bodyText(b), "the folder of the file is not in the page").not.toContain(files.inline.slice(0, files.inline.lastIndexOf("/")));
    await b.$("button=Import").click();
    await waitForBody(b, "Imported “inline”.", "the notice of the import never showed");
    expect(readdirSync(join(configDir(), "profiles"))).toHaveLength(6);
    expect(readdirSync(join(configDir(), "profiles")).map((name) => readFileSync(join(configDir(), "profiles", name), "utf8")).filter((text) => text.includes('name = "inline"'))).toHaveLength(1);
  });

  it("a_second_launch_hands_the_file_to_the_first_window", async () => {
    daemon("start");
    const files = importFixtures();
    const b = await openApp();
    await waitForBody(b, CONNECTED, "the bar never showed the active link");
    const before = running();
    expect(before, "one process of the app is running before the second launch").toHaveLength(1);
    // the second launch: it has no display to open, it hands the file over and ends
    const started = Date.now();
    const second = spawnSync(app(), [files.office], { encoding: "utf8", timeout: 5000 });
    expect(second.status, `the second launch ended with ${second.status}: ${second.stderr}`).toBe(0);
    expect(Date.now() - started, "the second launch took long").toBeLessThan(5000);
    // the first window shows the preview of the file, WireGuard with its hook removed
    await waitForBody(b, "File: office.conf", "the first window never showed the file of the second launch");
    expect(await bodyText(b)).toContain("PostUp");
    expect(running(), "still one process of the app").toEqual(before);
    expect(readdirSync(join(configDir(), "profiles")), "nothing is written before «Import»").toHaveLength(5);
    // a second launch with no file only brings the window forward: nothing new in the window
    const bare = spawnSync(app(), [], { encoding: "utf8", timeout: 5000 });
    expect(bare.status, bare.stderr).toBe(0);
    expect(running()).toEqual(before);
    // a file that cannot be previewed is an error inside the same window
    const notAProfile = join(env("XDG_RUNTIME_DIR"), "not-a-profile.txt");
    writeFileSync(notAProfile, "hello\n");
    expect(spawnSync(app(), [notAProfile], { encoding: "utf8", timeout: 5000 }).status).toBe(0);
    await waitForBody(b, "Only .ovpn and .conf files can be imported.", "the error of the file never showed");
    expect(running()).toEqual(before);
  });

  it("a_second_launch_without_a_runtime_dir_opens_a_second_window", async () => {
    daemon("start");
    const files = importFixtures();
    // S11: without a runtime folder there is no channel; both processes start on their own and say so. They are started by
    // hand: the driver would give them the variable of the run.
    const bare = { ...process.env };
    delete bare.XDG_RUNTIME_DIR;
    const children: ChildProcess[] = [];
    const said: string[] = [];
    const launch = (args: string[]) => {
      const child = spawn(app(), args, { env: bare, stdio: ["ignore", "ignore", "pipe"] });
      child.stderr?.on("data", (chunk: Buffer) => said.push(chunk.toString()));
      children.push(child);
      return child;
    };
    try {
      const first = launch([]);
      await settle(4000);
      expect(alive(first), "the first process started").toBe(true);
      const second = launch([files.office]);
      await settle(4000);
      expect(alive(first) && alive(second), "both processes live: the second did not hand over and end").toBe(true);
      expect(said.join("").match(/no single window/g) ?? [], said.join("")).toHaveLength(2);
    } finally {
      for (const child of children) child.kill("SIGKILL");
      await settle(500);
    }
  });

  it("an_ovpn_is_opened_by_the_desktop_entry", async () => {
    daemon("start");
    const files = importFixtures();
    const b = await openApp();
    await waitForBody(b, CONNECTED, "the bar never showed the active link");
    // the file `.desktop` of packaging/linux in a data folder of its own, with the command that points to this binary
    const data = mkdtempSync(join(tmpdir(), "galleria-xdg-data-"));
    try {
      const applications = join(data, "applications");
      mkdirSync(applications);
      const name = "io.github.zpeppoz.galleria.desktop";
      const source = readFileSync(join(env("GALLERIA_SMOKE_REPO"), "packaging", "linux", name), "utf8");
      expect(source).toContain("Exec=galleria-gui %f");
      const entry = join(applications, name);
      writeFileSync(entry, source.replace("Exec=galleria-gui %f", `Exec=${app()} %f`));
      // what `update-desktop-database` writes (the image has no desktop-file-utils): the types of each file `.desktop`
      writeFileSync(join(applications, "mimeinfo.cache"), `[MIME Cache]\napplication/x-openvpn-profile=${name};\n`);
      const gioEnv = { ...process.env, XDG_DATA_HOME: data, XDG_DATA_DIRS: `${data}:/usr/local/share:/usr/share` };
      // a .ovpn has its type, and the application is the one that is registered for it
      const type = execFileSync("gio", ["info", "-a", "standard::content-type", files.inline], { encoding: "utf8", env: gioEnv });
      expect(type).toContain("application/x-openvpn-profile");
      const registered = execFileSync("gio", ["mime", "application/x-openvpn-profile"], { encoding: "utf8", env: gioEnv });
      expect(registered, registered).toContain(name);
      // and the launcher of GLib runs its `Exec` with the path of the file; the app is already open, so it hands the file over
      const before = running();
      execFileSync("gio", ["launch", entry, files.inline], { encoding: "utf8", env: gioEnv, timeout: 15000 });
      await waitForBody(b, "File: inline.ovpn", "the file opened by the desktop entry never showed in the first window");
      await settle(500);
      expect(running(), "no second process of the app was left").toEqual(before);
    } finally {
      rmSync(data, { recursive: true, force: true });
    }
  });
});

// T7 (plan B4, I1, I6): what the window imports is what the CLI of the phase 1 imports from the same file, the CLI lists it,
// the window imports with the daemon stopped, and the whole tour stays inside the CSP. The files come the way a file
// manager hands them over (the arguments of the process, then second launches), so the window of «Open» is not needed.
describe("the import against the CLI of the phase 1 (plan B4, T7)", () => {
  const app = () => env("GALLERIA_SMOKE_APP");
  const configDir = (home = env("XDG_CONFIG_HOME")) => join(home, "galleria");
  const runCli = (args: string[], home?: string) => execFileSync(env("GALLERIA_SMOKE_CLI"), args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], ...(home ? { env: { ...process.env, XDG_CONFIG_HOME: home } } : {}) });
  const waitFor = (b: Browser, check: () => Promise<boolean>, timeoutMsg: string) => b.waitUntil(check, { timeout: TEN_SECONDS, interval: 250, timeoutMsg });
  const rowNames = async (b: Browser) => (await b.execute(() => [...document.querySelectorAll('[role="listbox"] [role="option"] span.profile-name')].map((el) => (el.textContent ?? "").trim()))) as string[];
  const watchCsp = (b: Browser) =>
    b.execute(() => {
      const w = window as unknown as { __csp: string[] };
      w.__csp = [];
      document.addEventListener("securitypolicyviolation", (e) => w.__csp.push(`${e.violatedDirective} ${e.blockedURI}`));
    });
  const cspViolations = (b: Browser) => b.execute(() => (window as unknown as { __csp: string[] }).__csp);

  /** The next file as a second launch of the app: it hands the file to the window that is open and ends. */
  const secondLaunch = (path: string) => {
    const second = spawnSync(app(), [path], { encoding: "utf8", timeout: 5000 });
    expect(second.status, `the second launch ended with ${second.status}: ${second.stderr}`).toBe(0);
  };
  /** The preview that is open is saved with the name it proposes. */
  const importIt = async (b: Browser, name: string) => {
    await b.$('input[name="import-name"]').waitForDisplayed({ timeout: TEN_SECONDS });
    expect(await b.$('input[name="import-name"]').getValue()).toBe(name);
    await b.$("button=Import").click();
    await waitForBody(b, `Imported “${name}”.`, `the notice of the import of ${name} never showed`);
  };
  /** Pastes `text` into the field of the first step (the setter of the browser and an `input` event, which is what a paste does) and asks for the preview. */
  const paste = async (b: Browser, text: string) => {
    const field = await b.$('textarea[name="import-text"]');
    await field.waitForDisplayed({ timeout: TEN_SECONDS });
    await b.execute(
      (selector: string, value: string) => {
        const el = document.querySelector(selector) as HTMLTextAreaElement;
        Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(el, value);
        el.dispatchEvent(new Event("input", { bubbles: true }));
      },
      'textarea[name="import-text"]',
      text,
    );
    await b.$("button=Preview").click();
  };

  /** A profile as the file holds it, without what differs from one import to the next: its id and its dates. */
  const same = (text: string) =>
    text
      .split("\n")
      .filter((line) => !/^(id|created|modified) = /.test(line))
      .join("\n");
  /** The profiles of a folder by name: the file of the profile and the file of its secrets, both without the id. */
  const byName = (home: string): Record<string, { profile: string; secrets: string }> => {
    const out: Record<string, { profile: string; secrets: string }> = {};
    for (const file of readdirSync(join(configDir(home), "profiles"))) {
      const text = readFileSync(join(configDir(home), "profiles", file), "utf8");
      const name = /^name = "(.*)"$/m.exec(text)?.[1] ?? file;
      out[name] = { profile: same(text), secrets: readFileSync(join(configDir(home), "secrets", file), "utf8") };
    }
    return out;
  };

  it("gui_import_equals_cli_import", async () => {
    daemon("stop");
    const files = importFixtures();
    // the window: one file as an argument of the process, two as second launches
    // opened as a session without a keyring, so that the window writes the file the CLI writes and the two can be compared
    const b = await openApp(false, "empty", [files.inline], "none");
    await waitForBody(b, "File: inline.ovpn", "the preview of inline.ovpn never showed");
    await importIt(b, "inline");
    secondLaunch(files.separate);
    await waitForBody(b, "File: separate.ovpn", "the preview of separate.ovpn never showed");
    await importIt(b, "separate");
    secondLaunch(files.office);
    await waitForBody(b, "File: office.conf", "the preview of office.conf never showed");
    await importIt(b, "office");
    // the CLI of the phase 1: the same three files, into another folder of profiles
    const other = mkdtempSync(join(tmpdir(), "galleria-cli-import-"));
    try {
      for (const [file, name] of [[files.inline, "inline"], [files.separate, "separate"], [files.office, "office"]]) runCli(["import", file, "--name", name], other);
      const window = byName(env("XDG_CONFIG_HOME"));
      const cli = byName(other);
      expect(Object.keys(window).sort()).toEqual(["inline", "office", "separate"]);
      expect(Object.keys(cli).sort()).toEqual(["inline", "office", "separate"]);
      for (const name of Object.keys(window)) {
        expect(window[name].profile, `the profile «${name}» is not what the CLI saves`).toBe(cli[name].profile);
        expect(window[name].secrets, `the secrets of «${name}» are not what the CLI saves`).toBe(cli[name].secrets);
        expect(window[name].secrets.length, `«${name}» has secrets to compare`).toBeGreaterThan(0);
      }
    } finally {
      rmSync(other, { recursive: true, force: true });
    }
  });

  it("the_cli_lists_what_the_window_imported", async () => {
    daemon("stop");
    const files = importFixtures();
    const b = await openApp(false, "seeded", [files.inline]);
    await waitForBody(b, "File: inline.ovpn", "the preview of inline.ovpn never showed");
    await importIt(b, "inline");
    secondLaunch(files.office);
    await waitForBody(b, "File: office.conf", "the preview of office.conf never showed");
    await importIt(b, "office");
    const listed = runCli(["list"]);
    const names = listed.trim().split("\n").map((line) => line.split(" ").slice(1).join(" ").replace(/ "?(Open[Vv]pn|Wire[Gg]uard)"?$/, ""));
    expect(names.sort(), `galleria list says: ${listed}`).toEqual(["Cliente Rossi", "Datacenter Milano", "Homelab", "Ufficio Napoli", "VPS Hetzner", "inline", "office"]);
  });

  it("import_works_with_the_daemon_stopped", async () => {
    daemon("stop");
    const files = importFixtures();
    const b = await openApp(false, "empty");
    await waitForBody(b, START_THE_SERVICE, "the «Start the service» panel never appeared");
    await b.$("button=Import a profile…").click();
    await paste(b, readFileSync(files.inline, "utf8"));
    await b.$('input[name="import-name"]').waitForDisplayed({ timeout: TEN_SECONDS });
    await b.$('input[name="import-name"]').setValue("Offline");
    await b.$("button=Import").click();
    await waitForBody(b, "Imported “Offline”.", "the notice of the import never showed");
    await waitFor(b, async () => (await rowNames(b)).join() === "Offline", "the list did not show the new profile");
    // the panel of the missing service is still there next to it, and the files are on the disk
    expect(await bodyText(b)).toContain("Start the service");
    const dir = join(configDir(), "profiles");
    const saved = readdirSync(dir).map((name) => readFileSync(join(dir, name), "utf8"));
    expect(saved).toHaveLength(1);
    expect(saved[0]).toContain('name = "Offline"');
    expect(readdirSync(join(configDir(), "secrets"))).toHaveLength(0);
  });

  it("no_csp_violation_during_the_import_tour", async () => {
    daemon("start");
    const files = importFixtures();
    const b = await openApp();
    await waitForBody(b, CONNECTED, "the bar never showed the active link");
    await watchCsp(b);
    // the text: preview, back, cancel; then the text again and import
    await b.$("button=Import…").click();
    await paste(b, readFileSync(files.malicious, "utf8"));
    await b.$('input[name="import-name"]').waitForDisplayed({ timeout: TEN_SECONDS });
    await b.$("button=Back").click();
    await b.$('textarea[name="import-text"]').waitForDisplayed({ timeout: TEN_SECONDS });
    await b.$("button=Cancel").click();
    await waitFor(b, async () => (await b.$$('[role="dialog"]')).length === 0, "the window did not close");
    // a file handed over by a second launch: the one with the removed directives, saved; then a hook of WireGuard, given up; then an error
    secondLaunch(files.malicious);
    await waitForBody(b, "File: malicious.ovpn", "the preview of malicious.ovpn never showed");
    await importIt(b, "malicious");
    secondLaunch(files.office);
    await waitForBody(b, "File: office.conf", "the preview of office.conf never showed");
    await b.$("button=Cancel").click();
    await waitFor(b, async () => (await b.$$('[role="dialog"]')).length === 0, "the window did not close");
    const notAProfile = join(env("XDG_RUNTIME_DIR"), "not-a-profile.txt");
    writeFileSync(notAProfile, "hello\n");
    secondLaunch(notAProfile);
    await waitForBody(b, "Only .ovpn and .conf files can be imported.", "the error of the file never showed");
    expect(await cspViolations(b)).toEqual([]);
  });
});
