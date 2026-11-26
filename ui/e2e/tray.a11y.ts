// Plan B6, T7: the tray and the closing of the window under a screen reader (Q22 (a): Orca and the AT-SPI tree in the real window, run locally by
// lab/tests/gui-a11y.sh, i.e. `scripts/gui.sh a11y`). The daemon is the fake one; the menu of the tray is read and clicked from the session bus by
// lab/tests/tray-probe.py, which stands in for the desktop that draws it (there is none under Xvfb). What Orca said goes into
// target/gui-a11y/transcript.txt.
//   K6a  a change of state is said, once and polite, by Rust, with the window hidden (S50)
//   K6b  the question that a close opens («Save · Discard · Cancel») is read as an alert dialog, with the focus on «Cancel», and nothing is announced for it
//   K6c  the notice of a newer version is text of the page, not an announcement
import { afterEach, describe, expect, it } from "vitest";
import {
  appRunning,
  atspiTree,
  closeApp,
  daemon,
  findNode,
  focusedNode,
  knownUpdateState,
  openApp,
  openAppWithState,
  orcaDebugLines,
  record,
  requestClose,
  speechMark,
  speechSince,
  startTrayWatcher,
  stopTrayWatcher,
  trayClick,
  trayRegistered,
  trayTexts,
  until,
  waitForBody,
  waitForSpeech,
  waitForTree,
  windowState,
  type AtNode,
  type Browser,
} from "./support";

const settle = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const announced = () => orcaDebugLines(/Dequeued object:announcement/);
const said = (mark: number, text: string | RegExp) => speechSince(mark).filter((line) => (typeof text === "string" ? line.includes(text) : text.test(line)));

afterEach(async () => {
  await closeApp();
  stopTrayWatcher();
});

async function openWithTray(scenario = "plain"): Promise<Browser> {
  daemon("stop");
  daemon("start", scenario);
  await startTrayWatcher();
  const b = await openApp();
  await waitForBody(b, "Connected to the service", "the shell never showed the link");
  expect(trayRegistered(), "the app never registered a tray").toBe(true);
  return b;
}

describe("the tray and the closing of the window under a screen reader", () => {
  it("K6a_a_state_change_is_said_with_the_window_hidden", async () => {
    await openWithTray("plain");
    await until(trayTexts, (texts) => texts.includes("Connect «Ufficio Napoli»"), "the menu never offered «Connect «Ufficio Napoli»»");
    requestClose();
    await until(windowState, (shown) => shown === "false", "the window was not hidden by the request to close it");
    const before = announced().length;
    const mark = speechMark();
    // the menu is the only way in: the page is hidden and nobody touches it
    trayClick("Connect «Ufficio Napoli»");
    const SAID = "Connected to Ufficio Napoli";
    await waitForSpeech(mark, SAID, 15_000);
    await settle(1500);
    record("K6a a state change said by Rust with the window hidden (clicked in the menu of the tray)", speechSince(mark), atspiTree());
    expect(said(mark, SAID), `Orca said it ${said(mark, SAID).length} times: ${JSON.stringify(speechSince(mark))}`).toHaveLength(1);
    // sent by the window of Rust as *polite*, once: `(1, 0, text)`; the window was hidden all the time
    const lines = announced().slice(before).filter((line) => line.includes(SAID));
    expect(lines.length, "Orca dequeued an object:announcement carrying the state").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain(`(1, 0, ${SAID})`);
    expect(windowState()).toBe("false");
    expect(appRunning()).toBe(true);
    // the menu did not announce anything of its own: nothing but the one phrase («Connecting» is not said either, Q27)
    expect(announced().slice(before).filter((line) => !line.includes(SAID))).toEqual([]);
  });

  it("K6b_the_guard_opened_by_a_close_is_read", async () => {
    const b = await openWithTray("plain");
    await b.$('input[name="name"]').click();
    await b.keys(["Control", "a"]);
    await b.keys(Array.from("Sede Napoli"));
    await waitForBody(b, "You have changes that are not saved.", "the save bar never appeared");
    const before = announced().length;
    const mark = speechMark();
    // what a window manager sends when the «X» is pressed: Rust stops it and the page asks
    requestClose();
    const tree = await waitForTree((t) => !!t.find((n) => (n.role === "dialog" || n.role === "alert") && n.name === "Save your changes?"), "the question of the close in the tree");
    const guard = tree.find((n) => (n.role === "dialog" || n.role === "alert") && n.name === "Save your changes?") as AtNode;
    expect(guard, "the question is a dialog with a name").toBeDefined();
    // the focus is inside it, on the choice that does nothing, and the three choices are there
    const focused = await waitForTree((t) => focusedNode(t)?.name === "Cancel", "the focus on «Cancel»");
    expect(focusedNode(focused)?.role).toBe("push button");
    for (const name of ["Cancel", "Discard", "Save"]) expect(findNode(focused, "push button", name), `the button «${name}»`).toBeDefined();
    expect(windowState()).toBe("true");
    await settle(2500);
    record("K6b the question opened by a close (Orca 46.1 in the image may say «blank» as in K5c: the tree and the focus are what is asserted)", speechSince(mark), focused);
    // Rust announces nothing for a question: it takes the focus and the screen reader reads it (Q27, rule 2)
    expect(announced().slice(before), "no announcement was sent for the question of the close").toEqual([]);
    // and the draft is where it was: «Cancel» by the keyboard closes the question and nothing else
    await b.keys(["Enter"]);
    await b.waitUntil(async () => (await b.$$('[role="alertdialog"]')).length === 0, { timeout: 10_000, interval: 250, timeoutMsg: "the question stayed after «Cancel»" });
    expect(await b.$('input[name="name"]').getValue()).toBe("Sede Napoli");
    expect(windowState()).toBe("true");
    expect(appRunning()).toBe(true);
  });

  it("K6c_the_update_notice_is_text_not_an_announcement", async () => {
    daemon("stop");
    daemon("start", "plain");
    // a newer version is already known and the day has not passed: the notice is there at the start, with no request made
    const b = await openAppWithState(knownUpdateState());
    await waitForBody(b, "Galleria 9.9.9 is available", "the notice never showed");
    await settle(1500);
    const mark = speechMark();
    const before = announced().length;
    const tree = atspiTree();
    // it is text of the page, in a region of its own that has the name of the title, and not a live region
    const region = tree.find((n) => /Galleria 9\.9\.9 is available/.test(n.name) && n.role !== "frame");
    record("K6c the notice of a newer version, as the tree has it", speechSince(mark), tree);
    expect(region, "the tree has the notice").toBeDefined();
    const live = await b.execute(() => {
      const found = document.querySelector("section.update-notice");
      const lively = (el: Element) => el.closest('[aria-live]:not([aria-live="off"]), [role="alert"], [role="status"], [role="log"]') !== null;
      return found ? { live: lively(found), alertsInside: found.querySelectorAll('[role="alert"], [role="status"], [aria-live]:not([aria-live="off"])').length } : null;
    });
    expect(live, "the notice is a region of the page that is not a live region").toEqual({ live: false, alertsInside: 0 });
    await settle(1500);
    expect(announced().slice(before), "no announcement was sent when the notice came up").toEqual([]);
    expect(announced().filter((line) => /9\.9\.9|available/.test(line)), "no announcement ever carried the notice").toEqual([]);
    await waitForBody(b, "Copy link", "the notice has no «Copy link»");
  });
});
