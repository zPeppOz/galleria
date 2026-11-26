// Plan B5, groups C and D: the connection under a screen reader (Q22 (a): Orca and the AT-SPI tree in the real window, run locally by
// lab/tests/gui-a11y.sh, i.e. `scripts/gui.sh a11y`). The daemon is the fake one, with a scenario. What Orca said goes into
// target/gui-a11y/transcript.txt.
//   K5e  «Export log…» through the real «Save» window (GTK, driven over AT-SPI): the file is written, private, and the outcome is said once
import { mkdtempSync, readdirSync, readFileSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { answerFileWindow, atspiTree, bodyText, closeApp, daemon, openApp, orcaDebugLines, record, speechMark, speechSince, TEN_SECONDS, waitForBody, waitForSpeech, type Browser } from "./support";

let b: Browser;

const settle = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const waitFor = (check: () => Promise<boolean>, timeoutMsg: string) => b.waitUntil(check, { timeout: TEN_SECONDS, interval: 250, timeoutMsg });

async function openWith(scenario: string): Promise<void> {
  daemon("stop");
  daemon("start", scenario);
  b = await openApp();
  await waitForBody(b, "Connected to the service", "the shell never showed the link");
}

afterEach(closeApp);

describe("the log of a session under a screen reader", () => {
  beforeEach(async () => {
    await openWith("plain");
    await b.$("button=Connect").click();
    await waitFor(async () => (await b.$('[data-cell="strip.state"] dd').getText()).trim() === "Connected", "the profile never connected");
    await b.$("button=Log").click();
    await waitFor(async () => (await b.$$(".log-line")).length > 0, "the tab «Log» never showed a line");
  });

  it("K5e_the_log_is_exported_through_the_real_save_window", async () => {
    const dir = mkdtempSync(join(tmpdir(), "galleria-log-"));
    const target = join(dir, "session.txt");
    // the focus goes to the button and the key asks; the key is not awaited, because the command answers when the window of Rust closes
    await b.execute(() => [...document.querySelectorAll<HTMLElement>("button")].find((el) => /^Export log/.test(el.textContent ?? ""))?.focus());
    const mark = speechMark();
    void b.keys(["Enter"]).catch(() => undefined);
    const answer = await answerFileWindow({ save: target });
    expect(answer.offered, "the window proposes the name of P12").toMatch(/^galleria-log-\d{8}-\d{4}\.txt$/);
    expect(answer).toMatchObject({ pressed: "Save", closed: true });
    const DONE = "Log exported.";
    await waitForSpeech(mark, DONE, 5000);
    await settle(1500);
    const times = speechSince(mark).filter((line) => line.includes(DONE)).length;
    record("K5e the export of the log, saved through the real window", speechSince(mark), atspiTree());
    expect(times, `Orca said the notice ${times} times: ${JSON.stringify(speechSince(mark))}`).toBe(1);
    const lines = orcaDebugLines(/Dequeued object:announcement/).filter((line) => line.includes(DONE));
    expect(lines.length, "Orca dequeued an object:announcement carrying the notice").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain(`(1, 0, ${DONE})`);
    // the file is private, holds the lines of the session in the format of `galleria logs`, and nothing else was left next to it
    expect(statSync(target).mode & 0o777).toBe(0o600);
    expect(readFileSync(target, "utf8")).toMatch(/^\d+ Info Daemon: .*connect/m);
    expect(readdirSync(dir)).toEqual(["session.txt"]);
    // the page never learns the place
    expect(await bodyText(b)).not.toContain(dir);
    // and Orca did not read the lines of the log as they came (the list is a log with its announcements off)
    expect(speechSince(mark).filter((line) => /Info Daemon|connect$/.test(line))).toEqual([]);
  });
});
