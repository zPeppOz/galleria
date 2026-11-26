// WebDriver smoke test of the tray and of the closing of the window (plan B6, T1 to T3, gates G10 and, from T4, G11): the real app in a
// real WebKitGTK window against the fake daemon, with the menu of its tray read and clicked from the session bus by
// lab/tests/tray-probe.py. Under Xvfb there is no desktop to show a tray, so the probe stands in for it (`startTrayWatcher`); a test
// that wants no tray starts no watcher. Run by lab/tests/gui-smoke.sh with the other `*.e2e.ts` files.
import { execFileSync, spawnSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { bodyText, choose, closeApp, daemon, env, importFixtures, knownUpdateState, openApp, openAppWithState, readState, startUpdateServer, type UpdateServer, startTrayWatcher, stopTrayWatcher, appRunning, requestClose, trayClick, trayIcon, trayRegistered, trayTexts, until, waitFor, waitForBody, windowState, type Browser } from "./support";

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
    // the window is hidden in the tray, as after a click on its «X»: from here the menu and the page do it all, hidden (S49)
    requestClose();
    await until(windowState, (shown) => shown === "false", "the window was not hidden by the request to close it");
    // no call to the window between the two clicks
    trayClick("Connect «Ufficio Napoli»");
    await until(state, (text) => text === "Connected to «Ufficio Napoli»", "the line of the state never said «Connected to «Ufficio Napoli»» with the window hidden");
    expect(await until(trayIcon, (icon) => icon === "on", "the icon never became the one of «connected»")).toBe("on");
    expect(cli("status")).toBe("connected");
    expect(trayTexts()).toContain("Disconnect «Ufficio Napoli»");
    // a connection that asks for nothing does not bring the window back
    expect(windowState()).toBe("false");
    trayClick("Disconnect «Ufficio Napoli»");
    await until(state, (text) => text === "Disconnected", "the line of the state never went back to «Disconnected»");
    expect(cli("status")).toBe("disconnected");
    expect(await until(trayIcon, (icon) => icon === "off", "the icon never went back")).toBe("off");
    expect(windowState()).toBe("false");
    expect(appRunning()).toBe(true);
  });

  it("a_favourite_connects_and_disconnects_from_the_menu_with_the_window_shown", async () => {
    await openWithTray("plain");
    await until(trayTexts, (texts) => texts.includes("Connect «Ufficio Napoli»"), "the menu never offered «Connect «Ufficio Napoli»»");
    trayClick("Connect «Ufficio Napoli»");
    await until(state, (text) => text === "Connected to «Ufficio Napoli»", "the line of the state never said «Connected to «Ufficio Napoli»»");
    expect(await until(trayIcon, (icon) => icon === "on", "the icon never became the one of «connected»")).toBe("on");
    expect(cli("status")).toBe("connected");
    trayClick("Disconnect «Ufficio Napoli»");
    await until(state, (text) => text === "Disconnected", "the line of the state never went back to «Disconnected»");
    expect(cli("status")).toBe("disconnected");
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

describe("the menu asks and the window comes back (plan B6, T2, S49)", () => {
  it("credentials_asked_after_a_menu_connect_bring_the_window_back", async () => {
    const b = await openWithTray("credentials");
    await until(trayTexts, (texts) => texts.includes("Connect «Ufficio Napoli»"), "the menu never offered «Connect «Ufficio Napoli»»");
    requestClose();
    await until(windowState, (shown) => shown === "false", "the window was not hidden by the request to close it");
    trayClick("Connect «Ufficio Napoli»");
    // the daemon asks for the credentials: the page cannot ask in a window that is hidden, so Rust shows it
    await until(windowState, (shown) => shown === "true", "the window did not come back when the credentials were asked", 15_000);
    const dialogs = () => b.$$('[role="dialog"]');
    await waitFor(b, async () => (await dialogs()).length === 1 && /Credentials for «Ufficio Napoli»/.test(await (await dialogs())[0].getText()), "the window of the credentials never showed");
    expect(await b.execute(() => document.activeElement?.getAttribute("name"))).toBe("username");
    // nothing connected without them
    expect(cli("status")).not.toBe("connected");
    await b.$("#credentials-username").setValue("carla.v");
    await b.$("#credentials-password").setValue("PW-tray-smoke-4471");
    await b.$('[role="dialog"] button[type="submit"]').click();
    await until(state, (text) => text === "Connected to «Ufficio Napoli»", "the session never came up after the credentials");
    expect(cli("status")).toBe("connected");
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
  });

  it("G10d_open_shows_the_window_and_quit_ends_the_app", async () => {
    const b = await openWithTray("plain");
    // a profile chosen in the page: «Open Galleria» must bring the page back as it was, not a new one
    await choose(b, "Ufficio Napoli");
    trayClick("Connect «Ufficio Napoli»");
    await until(state, (text) => text === "Connected to «Ufficio Napoli»", "the session never came up");
    requestClose();
    await until(windowState, (shown) => shown === "false", "the window was not hidden by the request to close it");
    trayClick("Open Galleria");
    await until(windowState, (shown) => shown === "true", "«Open Galleria» did not show the window");
    expect(await b.$("h1=Ufficio Napoli").isExisting(), "the page is not the one that was hidden").toBe(true);
    expect(cli("status")).toBe("connected");
    // «Quit» ends the process, and the session is not the app's to close
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
    // the form of the chosen profile is mounted again about 25 ms after the bar says the link (the cause of the stale clicks of the screen reader checks of
    // B5, docs/08, group B): a click on «Connect» in that window finds a button that is gone («stale element reference», 1 round of 3 in group C)
    await new Promise((resolve) => setTimeout(resolve, 600));
    await b.$("button=Connect").click();
    await waitFor(b, async () => cli("status") === "connected", "the session never came up");
    requestClose();
    await until(appRunning, (running) => !running, "the close did not end the app", 5000);
    expect(cli("status")).toBe("connected");
  });
});

describe("closing with changes that are not saved, and a second launch (plan B6, T3)", () => {
  const profilesDir = () => join(env("XDG_CONFIG_HOME"), "galleria", "profiles");
  /** How many profile files on the disk hold this name. */
  const filesNamed = (name: string): number => readdirSync(profilesDir()).filter((file) => readFileSync(join(profilesDir(), file), "utf8").includes(`name = "${name}"`)).length;
  /** A name typed over the one of the profile in the window, without saving it. */
  async function typeAName(b: Browser, name: string): Promise<void> {
    await b.$('input[name="name"]').click();
    await b.keys(["Control", "a"]);
    await b.keys(Array.from(name));
    await waitForBody(b, "You have changes that are not saved.", "the save bar never appeared");
  }
  const guard = (b: Browser) => b.$('[role="alertdialog"]');
  const focused = (b: Browser) => b.execute(() => document.activeElement?.textContent?.trim() ?? "");
  /**
   * A choice of the question that hides the window or ends the app. A WebDriver click waits for the page to answer, and a webview that is
   * hidden (or gone) never does: the command hangs. So the button is clicked from the page, which returns before the window goes.
   */
  const pressGuard = (b: Browser, label: string) =>
    b.execute((text) => [...document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')].find((button) => button.textContent?.trim() === text)?.click(), label);

  it("G10f_unsaved_changes_stop_the_close_and_the_quit", async () => {
    const b = await openWithTray("plain");
    await waitFor(b, async () => (await b.$('input[name="name"]').getValue()) === "Ufficio Napoli", "the first profile never showed");
    await typeAName(b, "Sede Napoli");
    // the close does not hide: the window stays, with the question, and the focus is on «Cancel»
    requestClose();
    await waitFor(b, async () => guard(b).isExisting(), "the question of the changes never showed after the close");
    expect(windowState()).toBe("true");
    await waitFor(b, async () => (await focused(b)) === "Cancel", "the focus is not on «Cancel»");
    // «Cancel» keeps everything where it was
    await guard(b).$("button=Cancel").click();
    await waitFor(b, async () => !(await guard(b).isExisting()), "the question stayed after «Cancel»");
    expect(windowState()).toBe("true");
    expect(await b.$('input[name="name"]').getValue()).toBe("Sede Napoli");
    expect(appRunning()).toBe(true);
    // «Discard» hides the window, and the file on the disk has the name it had
    requestClose();
    await waitFor(b, async () => guard(b).isExisting(), "the question never showed the second time");
    await pressGuard(b, "Discard");
    await until(windowState, (shown) => shown === "false", "«Discard» did not hide the window");
    expect(filesNamed("Ufficio Napoli")).toBe(1);
    expect(filesNamed("Sede Napoli")).toBe(0);
    expect(appRunning()).toBe(true);
    // again, and now «Quit» from the tray: the question comes up in a window that is shown, «Save» writes the file and the process ends
    trayClick("Open Galleria");
    await until(windowState, (shown) => shown === "true", "«Open Galleria» did not show the window");
    await waitFor(b, async () => (await b.$('input[name="name"]').getValue()) === "Ufficio Napoli", "the discarded name came back");
    await typeAName(b, "Sede Napoli");
    trayClick("Quit");
    await waitFor(b, async () => guard(b).isExisting(), "the question never showed after «Quit»");
    expect(windowState()).toBe("true");
    expect(appRunning()).toBe(true);
    await pressGuard(b, "Save");
    await until(appRunning, (running) => !running, "«Save» and the quit did not end the app", 5000);
    expect(filesNamed("Sede Napoli")).toBe(1);
  });

  it("a_second_launch_shows_the_hidden_window", async () => {
    const b = await openWithTray("plain");
    const files = importFixtures();
    requestClose();
    await until(windowState, (shown) => shown === "false", "the window was not hidden by the request to close it");
    // the second launch hands the file over and ends; the window that was hidden is shown, with the preview
    const second = spawnSync(env("GALLERIA_SMOKE_APP"), [files.inline], { encoding: "utf8", timeout: 5000 });
    expect(second.status, `the second launch ended with ${second.status}: ${second.stderr}`).toBe(0);
    await until(windowState, (shown) => shown === "true", "the second launch did not show the hidden window");
    await waitForBody(b, "File: inline.ovpn", "the preview of the file of the second launch never showed");
    expect(appRunning()).toBe(true);
  });
});

// ---- G11: the version check (plan B6, T4 and T5) -------------------------------------------------------------------------------------------

describe("the version check of the built app, against a server that counts (plan B6, T5, G11)", () => {
  let server: UpdateServer | undefined;
  afterEach(() => {
    server?.stop();
    server = undefined;
  });
  const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

  it("G11a_the_built_app_asks_once_and_never_when_disabled", async () => {
    server = await startUpdateServer();
    daemon("stop");
    daemon("start", "plain");
    // the app of the release, with its first look at the clock ten seconds after the start: one request, to the server that stands in for GitHub
    let b = await openApp(false, "seeded", [], "update");
    await until(() => server?.requests() ?? [], (requests) => requests.length >= 1, "the app never asked for the version", 25_000);
    const [first] = server.requests();
    expect(first.line).toBe("GET /zPeppOz/galleria/releases/latest HTTP/1.1");
    const names = first.headers.map(([name]) => name.toLowerCase()).sort();
    // what leaves the machine: these three and nothing else (S52); no `User-Agent`, no identifier
    expect(names).toEqual(["accept-encoding", "connection", "host"]);
    expect(first.headers.map(([, value]) => value).join(" ")).not.toMatch(/galleria|0\.1\.0|Mozilla|WebKit/i);
    // the window shows the notice, with the version the server named
    await waitFor(b, async () => (await b.$$("section")).length > 0 && (await bodyText(b)).includes("Galleria 9.9.9 is available"), "the notice never showed", 15_000);
    expect(await b.$("body").getText()).toContain("https://github.com/zPeppOz/galleria/releases/tag/v9.9.9");
    expect(await b.$$("a[href]")).toHaveLength(0);
    expect(readState()).toContain('latest = "9.9.9"');
    // the app is closed and opened again: the day has not passed, still one request
    await closeApp();
    b = await openApp(true, "seeded", [], "update");
    await waitForBody(b, "Galleria 9.9.9 is available", "the notice did not come back with the window");
    await wait(14_000);
    expect(server.requests(), "a second start asked again within the day").toHaveLength(1);
    await closeApp();
    // the check is off in `state.toml`: thirty seconds, no request at all (and no notice, whatever was known)
    const before = server.requests().length;
    b = await openAppWithState(`[update]\ncheck = false\nlatest = "9.9.9"\n`, "update");
    await waitForBody(b, CONNECTED, "the bar never showed the active link");
    await wait(30_000);
    expect(server.requests().length, "the app asked with the check turned off").toBe(before);
    expect(await bodyText(b)).not.toContain("is available");
  }, 150_000);

  it("turning_the_check_off_removes_the_notice", async () => {
    // a version is already known and the day has not passed: the notice is there with no request
    server = await startUpdateServer();
    daemon("stop");
    daemon("start", "plain");
    const b = await openAppWithState(knownUpdateState(), "update");
    await waitForBody(b, "Galleria 9.9.9 is available", "the notice never showed");
    await b.$("button=Menu").click();
    const item = await b.waitUntil(async () => (await b.$('[role="menuitemcheckbox"]').isExisting()) && b.$('[role="menuitemcheckbox"]'), { timeout: 10_000, interval: 250, timeoutMsg: "the menu has no switch for the check" });
    expect(await item.getText()).toBe("Check for a newer version");
    expect(await item.getAttribute("aria-checked")).toBe("true");
    await item.click();
    await waitFor(b, async () => !(await bodyText(b)).includes("is available"), "the notice stayed after the check was turned off");
    expect(readState()).toContain("check = false");
    // the same in Italian, and the choice survives the window: it is on the disk
    await closeApp();
    const again = await openApp(true, "seeded", [], "update");
    await waitForBody(again, CONNECTED, "the bar never showed the active link");
    await again.$("button=Menu").click();
    const back = await again.waitUntil(async () => (await again.$('[role="menuitemcheckbox"]').isExisting()) && again.$('[role="menuitemcheckbox"]'), { timeout: 10_000, interval: 250, timeoutMsg: "the menu has no switch for the check" });
    expect(await back.getAttribute("aria-checked")).toBe("false");
    expect(server.requests(), "turning it off asked nothing").toHaveLength(0);
  });
});
