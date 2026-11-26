// WebDriver smoke test of the tray and of the closing of the window (plan B6, T1 to T3, gates G10 and, from T4, G11): the real app in a
// real WebKitGTK window against the fake daemon, with the menu of its tray read and clicked from the session bus by
// lab/tests/tray-probe.py. Under Xvfb there is no desktop to show a tray, so the probe stands in for it (`startTrayWatcher`); a test
// that wants no tray starts no watcher. Run by lab/tests/gui-smoke.sh with the other `*.e2e.ts` files.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { afterEach, describe, expect, it } from "vitest";
import { choose, closeApp, daemon, env, openApp, startTrayWatcher, stopTrayWatcher, appRunning, requestClose, trayClick, trayIcon, trayRegistered, trayTexts, until, waitFor, waitForBody, windowState, type Browser } from "./support";

const CONNECTED = /Connected to the service|Collegato al servizio/;

afterEach(async () => {
  await closeApp();
  stopTrayWatcher();
});

/** The window and its tray, with the fake daemon running the scenario. */
async function openWithTray(scenario = "plain"): Promise<Browser> {
  daemon("stop");
  daemon("start", scenario);
  await startTrayWatcher();
  const b = await openApp();
  await waitForBody(b, CONNECTED, "the bar never showed the active link");
  expect(trayRegistered(), "the app never registered a tray").toBe(true);
  return b;
}

const italian = async (b: Browser) => {
  await b.$("button=Menu").click();
  const item = await b.waitUntil(
    async () => {
      for (const el of await b.$$('[role="menuitemradio"]')) if ((await el.getText()) === "Italiano") return el;
      return false;
    },
    { timeout: 10_000, interval: 250, timeoutMsg: "the menu never offered «Italiano»" },
  );
  await item.click();
};

describe("the menu of the tray, read from the bus (plan B6, T1)", () => {
  it("G10a_the_menu_says_the_state_and_lists_the_favourites", async () => {
    const b = await openWithTray();
    // the state, the two favourites of the seed in the order of the list, «Open», «Quit»
    const english = ["Disconnected", "Connect «Homelab»", "Connect «Ufficio Napoli»", "Open Galleria", "Quit"];
    await until(trayTexts, (texts) => texts.join("|") === english.join("|"), "the menu never said the state and the favourites");
    // the line of the state is text, not a choice; the icon is the one of «not connected»
    expect((await until(() => trayTexts(), (t) => t.length > 0, "no menu")).length).toBe(5);
    expect(await until(trayIcon, (icon) => icon === "off", "the icon is not the one of «disconnected»")).toBe("off");
    // the language changes in the window: the same entries, in Italian
    await italian(b);
    await until(trayTexts, (texts) => texts.join("|") === ["Disconnesso", "Connetti «Homelab»", "Connetti «Ufficio Napoli»", "Apri Galleria", "Esci"].join("|"), "the menu never went to Italian");
    // a star taken off in the window takes the entry off the menu
    await choose(b, "Homelab");
    await b.$('button[aria-label="Preferito"]').click();
    await until(trayTexts, (texts) => !texts.some((t) => t.includes("Homelab")) && texts.some((t) => t.includes("Ufficio Napoli")), "the entry of the profile that is not a favourite any more stayed in the menu");
    expect(await b.$("body").getText()).toContain("Homelab");
  });

  it("the_app_starts_without_the_indicator_library", async () => {
    daemon("stop");
    daemon("start");
    await startTrayWatcher();
    const b = await openApp(false, "seeded", [], "no-indicator");
    await waitForBody(b, CONNECTED, "the window never came up without the library of the indicator");
    // the window is there and works; nothing registered a tray, even with a desktop that would show it
    expect(trayRegistered(3)).toBe(false);
    await waitFor(b, async () => (await b.$("h1").isExisting()) || (await b.$("body").getText()).length > 0, "the window shows nothing");
    const stderr = await until(() => readFileSync(env("GALLERIA_SMOKE_NO_INDICATOR_STDERR"), "utf8"), (text) => text.includes("no tray"), "stderr never said there is no tray");
    expect(stderr).toMatch(/galleria-gui: no tray/);
    expect(stderr).not.toMatch(/panicked/);
  });
});

const cli = (...args: string[]): string => execFileSync(env("GALLERIA_SMOKE_CLI"), ["--socket", env("GALLERIA_SOCKET"), ...args], { encoding: "utf8" }).trim();
const state = () => trayTexts()[0];

describe("connecting and disconnecting from the menu of the tray (plan B6, T2)", () => {
  it("G10b_a_favourite_connects_and_disconnects_from_the_menu_alone", async () => {
    await openWithTray("plain");
    await until(trayTexts, (texts) => texts.includes("Connect «Ufficio Napoli»"), "the menu never offered «Connect «Ufficio Napoli»»");
    // no call to the window between the two clicks: the menu and the page do it all
    trayClick("Connect «Ufficio Napoli»");
    await until(state, (text) => text === "Connected to «Ufficio Napoli»", "the line of the state never said «Connected to «Ufficio Napoli»»");
    expect(await until(trayIcon, (icon) => icon === "on", "the icon never became the one of «connected»")).toBe("on");
    expect(cli("status")).toBe("connected");
    expect(trayTexts()).toContain("Disconnect «Ufficio Napoli»");
    trayClick("Disconnect «Ufficio Napoli»");
    await until(state, (text) => text === "Disconnected", "the line of the state never went back to «Disconnected»");
    expect(cli("status")).toBe("disconnected");
    expect(await until(trayIcon, (icon) => icon === "off", "the icon never went back")).toBe("off");
  });

  it("switching_favourite_from_the_menu_replaces_without_asking", async () => {
    const b = await openWithTray("plain");
    await until(trayTexts, (texts) => texts.includes("Connect «Ufficio Napoli»"), "no entry to connect");
    trayClick("Connect «Ufficio Napoli»");
    await until(state, (text) => text === "Connected to «Ufficio Napoli»", "the first favourite never connected");
    trayClick("Connect «Homelab»");
    await until(state, (text) => text === "Connected to «Homelab»", "the second favourite never replaced the first");
    // one session, of the second profile, and nobody was asked
    expect((JSON.parse(cli("status", "--json")) as { profile: { name: string } }).profile.name).toBe("Homelab");
    expect(await b.$$('[role="dialog"], [role="alertdialog"]')).toHaveLength(0);
    expect(trayTexts()).toEqual(["Connected to «Homelab»", "Disconnect «Homelab»", "Connect «Ufficio Napoli»", "Open Galleria", "Quit (the VPN stays on)"]);
  });
});

describe("closing the window and quitting (plan B6, T3)", () => {
  it("G10c_closing_the_window_hides_it_and_the_session_stays", async () => {
    await openWithTray("plain");
    trayClick("Connect «Ufficio Napoli»");
    await until(state, (text) => text === "Connected to «Ufficio Napoli»", "the session never came up");
    expect(await until(windowState, (shown) => shown === "true", "the window is not shown")).toBe("true");
    // what a window manager would send when the «X» is pressed
    requestClose();
    await until(windowState, (shown) => shown === "false", "the window was not hidden by the request to close it");
    expect(appRunning()).toBe(true);
    expect(cli("status")).toBe("connected");
    expect(trayTexts()[0]).toBe("Connected to «Ufficio Napoli»");
    // G10d: «Open» brings it back, with the page as it was; «Quit» ends the app and the session stays
    trayClick("Open Galleria");
    await until(windowState, (shown) => shown === "true", "«Open Galleria» did not show the window");
    trayClick("Quit (the VPN stays on)");
    await until(appRunning, (running) => !running, "«Quit» did not end the app", 5000);
    expect(cli("status")).toBe("connected");
  });

  it("G10e_without_a_tray_host_closing_quits_and_the_session_stays", async () => {
    daemon("stop");
    daemon("start", "plain");
    // no watcher: nobody shows a tray, so a window that is hidden could not come back (Q53)
    const b = await openApp();
    await waitForBody(b, CONNECTED, "the bar never showed the active link");
    await b.$("button=Connect").click();
    await waitFor(b, async () => cli("status") === "connected", "the session never came up");
    requestClose();
    await until(appRunning, (running) => !running, "the close did not end the app", 5000);
    expect(cli("status")).toBe("connected");
  });
});
