// WebDriver smoke test of the connection (plan B5, T5 and T6): the real app in a real WebKitGTK window against the fake daemon, which
// serves the real IPC with a fake backend that follows a scenario (`daemon-start <scenario>`, tests/support/scenarios.rs). Run by
// lab/tests/gui-smoke.sh with the other `*.e2e.ts` files. Nothing here touches a route, a DNS or an interface: the backend is fake.
import { afterEach, describe, expect, it } from "vitest";
import { bodyText, closeApp, daemon, openApp, TEN_SECONDS, waitForBody, type Browser } from "./support";

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
