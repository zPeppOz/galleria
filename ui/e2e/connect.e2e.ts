// WebDriver smoke test of the connection (plan B5, T5 and T6): the real app in a real WebKitGTK window against the fake daemon, which
// serves the real IPC with a fake backend that follows a scenario (`daemon-start <scenario>`, tests/support/scenarios.rs). Run by
// lab/tests/gui-smoke.sh with the other `*.e2e.ts` files. Nothing here touches a route, a DNS or an interface: the backend is fake.
import { execFileSync, spawnSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { bodyText, closeApp, daemon, env, openApp, TEN_SECONDS, waitForBody, type Browser } from "./support";

const CONNECTED = /Connected to the service|Collegato al servizio/;

afterEach(closeApp);

const waitFor = (b: Browser, check: () => Promise<boolean>, timeoutMsg: string) => b.waitUntil(check, { timeout: TEN_SECONDS, interval: 250, timeoutMsg });
const rowNames = async (b: Browser) => (await b.execute(() => [...document.querySelectorAll('[role="listbox"] [role="option"] span.profile-name')].map((el) => (el.textContent ?? "").trim()))) as string[];
const cell = async (b: Browser, name: string) => (await b.$(`[data-cell="${name}"] dd`).getText()).trim();
const barState = async (b: Browser) => (await b.$(".status-bar-state").getText()).trim();

/** The fake daemon runs the scenario (the scripts of its backend, one for each connect). The one already running is stopped first: it keeps the scenario it started with. */
async function openWith(scenario: string): Promise<Browser> {
  daemon("stop");
  daemon("start", scenario);
  const b = await openApp();
  await waitForBody(b, CONNECTED, "the bar never showed the active link");
  await waitFor(b, async () => (await rowNames(b)).length === 5, "the list never showed the five profiles");
  return b;
}

async function choose(b: Browser, name: string): Promise<void> {
  const row = await b.waitUntil(
    async () => {
      for (const el of await b.$$('[role="listbox"] [role="option"]')) if ((await el.getText()).trim().startsWith(name)) return el;
      return false;
    },
    { timeout: TEN_SECONDS, interval: 250, timeoutMsg: `the list has no row «${name}»` },
  );
  await row.click();
  await waitFor(b, async () => (await b.$("h1").getText()) === name, `the detail never showed «${name}»`);
}

describe("connecting from the window against the fake daemon (plan B5, T5)", () => {
  it("a_profile_connects_and_disconnects", async () => {
    const b = await openWith("plain");
    expect(await barState(b)).toBe("Disconnected");
    await b.$("button=Connect").click();
    await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", "the strip never said «Connected»");
    // the session is in the bar, with the address the fake tunnel gave
    await waitFor(b, async () => (await b.$(".status-bar-session").isExisting()) && (await b.$(".status-bar-session").getText()).includes("«Ufficio Napoli» · 10.0.0.2"), "the bar never showed «Ufficio Napoli» · 10.0.0.2");
    expect(await barState(b)).toBe("Connected");
    expect(await cell(b, "strip.ip")).toBe("10.0.0.2");
    expect(await b.$("button=Disconnect").isExisting()).toBe(true);
    await b.$("button=Disconnect").click();
    await waitFor(b, async () => (await barState(b)) === "Disconnected", "the bar never went back to «Disconnected»");
    await waitFor(b, async () => (await b.$("button=Connect").isExisting()) && !(await b.$(".status-bar-session").isExisting()), "the button and the bar did not go back to the idle state");
  });

  it("the_numbers_move_while_connected", async () => {
    const b = await openWith("stats");
    await choose(b, "Homelab");
    await b.$("button=Connect").click();
    const traffic = async () => (await b.execute(() => document.querySelector('[data-cell="strip.traffic"] dd span')?.getAttribute("title") ?? "")) as string;
    await waitFor(b, async () => (await traffic()) !== "", "the traffic never showed numbers");
    const first = await traffic();
    await waitFor(
      b,
      async () => {
        const now = await traffic();
        return now !== "" && now !== first;
      },
      `the traffic did not grow: it stayed at ${first}`,
    );
    // WireGuard has a handshake to show, and the fake gave one
    expect(await b.$('[data-cell="strip.handshake"]').isExisting()).toBe(true);
    expect(await cell(b, "strip.handshake")).not.toBe("—");
    await b.$("button=Disconnect").click();
    await waitFor(b, async () => (await barState(b)) === "Disconnected", "the bar never went back to «Disconnected»");
  });

  it("switching_profile_replaces_without_asking", async () => {
    const b = await openWith("plain");
    await b.$("button=Connect").click();
    await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", "the first profile never connected");
    await choose(b, "Homelab");
    await b.$("button=Connect").click();
    await waitFor(b, async () => (await b.$(".status-bar-session").getText()).includes("«Homelab»"), "the bar never showed the second profile");
    // no question was asked, one notice said what was left, and the session is of the second profile only
    expect(await b.$$('[role="dialog"], [role="alertdialog"]')).toHaveLength(0);
    expect((await bodyText(b)).match(/Disconnected from «Ufficio Napoli»/g) ?? []).toHaveLength(1);
    expect(await b.$(".status-bar-session").getText()).not.toContain("Ufficio Napoli");
    await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", "the second profile never connected");
    expect(await b.$("button=Disconnect").isExisting()).toBe(true);
    await choose(b, "Ufficio Napoli");
    expect(await cell(b, "strip.state"), "the first profile has no session any more").not.toBe("Connected");
  });

  it("an_unreachable_server_shows_the_translated_error_in_both_languages", async () => {
    const b = await openWith("unreachable");
    await b.$("button=Connect").click();
    await waitForBody(b, "Server unreachable after all attempts", "the error never showed in English");
    // the language changes while the error is on the screen, with no restart
    await b.$("button=Menu").click();
    const italian = await b.waitUntil(
      async () => {
        for (const item of await b.$$('[role="menuitemradio"]')) if ((await item.getText()) === "Italiano") return item;
        return false;
      },
      { timeout: TEN_SECONDS, interval: 250, timeoutMsg: "the menu never offered «Italiano»" },
    );
    await italian.click();
    await waitForBody(b, "Server irraggiungibile dopo tutti i tentativi", "the error did not follow the language");
    expect(await bodyText(b)).not.toContain("Server unreachable after all attempts");
  });
});

describe("credentials asked from the window (plan B5, T6)", () => {
  const USER = "carla.v";
  const PASSWORD = "PW-smoke-9d41-typed";
  const dialogs = (b: Browser) => b.$$('[role="dialog"]');

  /** The id of the profile of this name: the file of the rubrica is named after it. */
  function profileId(name: string): string {
    const dir = join(env("XDG_CONFIG_HOME"), "galleria", "profiles");
    const file = readdirSync(dir).find((entry) => readFileSync(join(dir, entry), "utf8").includes(`name = "${name}"`));
    if (!file) throw new Error(`no profile named ${name} on the disk`);
    return file.replace(/\.toml$/, "");
  }
  const keyring = (id: string, key: string) => spawnSync("secret-tool", ["lookup", "service", "galleria", "profile", id, "key", key], { encoding: "utf8" });
  const keysOf = (id: string) => `${spawnSync("secret-tool", ["search", "--all", "service", "galleria", "profile", id], { encoding: "utf8" }).stdout}`;

  async function waitForDialog(b: Browser, title: RegExp): Promise<void> {
    await waitFor(b, async () => (await dialogs(b)).length === 1 && title.test(await (await dialogs(b))[0].getText()), `the window of the credentials never showed ${String(title)}`);
  }
  const send = (b: Browser) => b.$('[role="dialog"] button[type="submit"]').click();

  it("credentials_are_asked_once_and_remembered", async () => {
    const b = await openWith("credentials");
    await choose(b, "Datacenter Milano");
    const id = profileId("Datacenter Milano");
    await b.$("button=Connect").click();
    await waitForDialog(b, /Credentials for «Datacenter Milano»/);
    // only the user, the password and the box; the focus is on the first field
    expect(await b.$$('[role="dialog"] input:not([type="checkbox"])')).toHaveLength(2);
    expect(await b.execute(() => document.activeElement?.getAttribute("name"))).toBe("username");
    await b.$("#credentials-username").setValue(USER);
    await b.$("#credentials-password").setValue(PASSWORD);
    await b.$('[role="dialog"] input[name="remember"]').click();
    await send(b);
    await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", "the profile never connected");
    // the keyring has the two entries, and the profile lists their names
    await waitFor(b, async () => keyring(id, "openvpn.password").stdout === PASSWORD, `the keyring has no password for the profile: ${keysOf(id)}`);
    expect(keyring(id, "openvpn.username").stdout).toBe(USER);
    const names = readFileSync(join(env("XDG_CONFIG_HOME"), "galleria", "profiles", `${id}.toml`), "utf8");
    expect(names).toContain("openvpn.username");
    expect(names).toContain("openvpn.password");
    expect(names, "the profile file names the credentials and has none of them").not.toContain(PASSWORD);
    // the tab «Authentication» says they are saved, and offers to forget them
    await b.$("button=Authentication").click();
    // the headings of the tabs are drawn in capitals
    await waitForBody(b, /credentials saved in the keyring/i, "the tab never said the credentials are saved");
    // the second connection does not ask
    await b.$("button=Disconnect").click();
    await waitFor(b, async () => (await barState(b)) === "Disconnected", "the bar never went back to «Disconnected»");
    await b.$("button=Connect").click();
    await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", "the second connection never came up");
    expect(await dialogs(b), "no window asked for the credentials the second time").toHaveLength(0);
    // and forgetting takes them out of the keyring and out of the profile
    await b.$("button=Forget").click();
    await waitFor(b, async () => keyring(id, "openvpn.password").stdout === "", "the password is still in the keyring after «Forget»");
    expect(keyring(id, "openvpn.username").stdout).toBe("");
    await waitFor(b, async () => !/credentials saved in the keyring/i.test(await bodyText(b)), "the tab still says the credentials are saved");
  });

  it("the_otp_is_asked_every_time_and_never_saved", async () => {
    const b = await openWith("otp");
    await choose(b, "Cliente Rossi");
    const id = profileId("Cliente Rossi");
    for (const turn of [1, 2]) {
      await b.$("button=Connect").click();
      await waitForDialog(b, /Code for «Cliente Rossi»/);
      // the prompt of the server is the label, and a code is not offered to be remembered
      expect(await b.$('[role="dialog"] label[for="credentials-otp"]').getText()).toBe("Enter OTP");
      expect(await b.$$('[role="dialog"] input[type="checkbox"]'), `turn ${turn}`).toHaveLength(0);
      await b.$("#credentials-otp").setValue("482913");
      await send(b);
      await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", `the connection ${turn} never came up`);
      await b.$("button=Disconnect").click();
      await waitFor(b, async () => (await barState(b)) === "Disconnected", "the bar never went back to «Disconnected»");
    }
    expect(keysOf(id)).not.toContain("openvpn.otp");
    expect(keysOf(id)).not.toContain("openvpn.password");
    expect(readFileSync(join(env("XDG_CONFIG_HOME"), "galleria", "profiles", `${id}.toml`), "utf8")).not.toContain("openvpn.password");
  });

  it("no_typed_value_is_left_in_the_page", async () => {
    const b = await openWith("credentials");
    await choose(b, "Datacenter Milano");
    await b.$("button=Connect").click();
    await waitForDialog(b, /Credentials for «Datacenter Milano»/);
    await b.$("#credentials-username").setValue(USER);
    await b.$("#credentials-password").setValue(PASSWORD);
    // before it is sent the value is in the field, and nowhere else (S43: WebDriver writes in the fields and sees them emptied)
    expect(await b.$("#credentials-password").getValue()).toBe(PASSWORD);
    await send(b);
    await waitFor(b, async () => (await dialogs(b)).length === 0, "the window of the credentials never closed");
    const left = () =>
      b.execute(() => ({
        values: [...document.querySelectorAll("input, textarea")].map((el) => (el as HTMLInputElement).value),
        attributes: [...document.querySelectorAll("*")].flatMap((el) => [...el.attributes].map((a) => a.value)),
        html: document.documentElement.outerHTML,
        text: document.body.innerText,
      }));
    for (const when of ["just sent", "connected"]) {
      if (when === "connected") await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", "the profile never connected");
      const page = await left();
      expect(page.values.filter((v) => v.includes(PASSWORD)), `a field holds the password (${when})`).toEqual([]);
      expect(page.attributes.filter((v) => v.includes(PASSWORD)), `an attribute holds the password (${when})`).toEqual([]);
      expect(page.html.includes(PASSWORD), `the page holds the password (${when})`).toBe(false);
      expect(page.text.includes(PASSWORD), `the text holds the password (${when})`).toBe(false);
      expect(await b.$$('input[type="password"]'), `no password field is left (${when})`).toHaveLength(0);
    }
    // nothing was asked to be remembered: the keyring has nothing of it
    expect(keysOf(profileId("Datacenter Milano"))).not.toContain("openvpn.password");
  });
});

describe("the log of the session (plan B5, T7)", () => {
  const lines = (b: Browser) => b.execute(() => [...document.querySelectorAll(".log-line")].map((el) => (el.textContent ?? "").trim())) as Promise<string[]>;

  it("the_log_tab_shows_the_session_and_copies_it", async () => {
    const b = await openWith("plain");
    // a profile that has no session shows no lines
    await b.$("button=Log").click();
    await waitForBody(b, "No session for this profile.", "the tab «Log» of a profile without a session said nothing");
    await b.$("button=Connect").click();
    await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", "the profile never connected");
    await waitFor(b, async () => (await lines(b)).length > 0, "the tab «Log» never showed a line of the session");
    expect(await bodyText(b)).toMatch(/current session/i);
    // a list with its announcements off, and the level and the source are words
    expect(await b.$('[role="log"]').getAttribute("aria-live")).toBe("off");
    expect((await lines(b)).some((line) => /info\s+daemon.*connect/.test(line)), `lines: ${JSON.stringify(await lines(b))}`).toBe(true);
    // «Copy» fills the system clipboard with what the tab shows, in the format of `galleria logs`
    await b.$("button=Copy").click();
    await waitForBody(b, "Log copied.", "the outcome of the copy was never said");
    const clipboard = () => {
      try {
        return execFileSync("xclip", ["-o", "-selection", "clipboard"], { encoding: "utf8" });
      } catch {
        return "";
      }
    };
    await b.waitUntil(async () => /^\d+ Info Daemon: .*connect/m.test(clipboard()), { timeout: TEN_SECONDS, interval: 250, timeoutMsg: `the clipboard did not receive the log: ${clipboard()}` });
    expect(clipboard().endsWith("\n")).toBe(true);
    // when the session ends the same lines are the last session
    await b.$("button=Disconnect").click();
    await waitFor(b, async () => (await barState(b)) === "Disconnected", "the bar never went back to «Disconnected»");
    await waitForBody(b, /last session/i, "the tab did not become «Last session»");
    expect((await lines(b)).length).toBeGreaterThan(0);
  });
});
