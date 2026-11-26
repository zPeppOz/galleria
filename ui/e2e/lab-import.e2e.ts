// G3 in the laboratory (plan B4, T6, Q37 (a)): the profiles of the laboratory (`lab/out`, the ones `t12` and `t16` connect
// with) are imported through the real window, the way a file manager would hand them over: the first as an argument of the
// process, the next two as a second launch that hands the file to the window that is already there. The folder of profiles
// the window wrote is left in `target/gui-lab/config/galleria`, where `crates/galleria-e2e/tests/t17_gui_import.rs` takes it
// to the clients, to be listed and connected by the real CLI and the real daemon.
//
// Not part of `scripts/gui.sh smoke`: vitest.config.ts runs this file only when GALLERIA_LAB_IMPORT is set
// (`scripts/gui.sh lab-import`, lab/tests/gui-lab-import.sh), because it needs the laboratory to be up.
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { bodyText, closeApp, daemon, env, openApp, TEN_SECONDS, waitForBody, type Browser } from "./support";

afterEach(async () => {
  await closeApp();
});

const labFile = (name: string): string => {
  const path = join(env("GALLERIA_LAB_OUT"), name);
  if (!existsSync(path)) {
    throw new Error(`${path} is not there: the laboratory makes it (lab/gen-pki.sh writes the .ovpn files, lab/up.sh starts the WireGuard server that writes wg-split.conf)`);
  }
  return path;
};
const configDir = () => join(env("XDG_CONFIG_HOME"), "galleria");
const savedProfiles = (): string[] => readdirSync(join(configDir(), "profiles")).map((name) => readFileSync(join(configDir(), "profiles", name), "utf8"));
const D19_SCRIPT = "Galleria does not run the scripts of profiles. If the profile depends on it, it may not work fully.";

/** The next file, as a second launch of the app: it hands the file to the window that is open and ends. */
function secondLaunch(path: string): void {
  const second = spawnSync(env("GALLERIA_SMOKE_APP"), [path], { encoding: "utf8", timeout: 5000 });
  expect(second.status, `the second launch ended with ${second.status}: ${second.stderr}`).toBe(0);
}

/** Presses «Import» on the preview that is open and waits for the notice. */
async function importIt(b: Browser, name: string): Promise<void> {
  await b.$('input[name="import-name"]').waitForDisplayed({ timeout: TEN_SECONDS });
  expect(await b.$('input[name="import-name"]').getValue()).toBe(name);
  await b.$("button=Import").click();
  await waitForBody(b, `Imported “${name}”.`, `the notice of the import of ${name} never showed`);
}

describe("the profiles of the laboratory, imported through the window (G3)", () => {
  it("lab_profiles_are_imported_through_the_window", async () => {
    daemon("stop");
    // the first file is an argument of the process (S30), the rest are second launches (T5)
    const b = await openApp(false, "empty", [labFile("base.ovpn")]);
    await waitForBody(b, "File: base.ovpn", "the preview of base.ovpn never showed");
    expect(await bodyText(b), "the secrets of the profile are named and never shown").not.toContain("PRIVATE KEY");
    await importIt(b, "base");
    secondLaunch(labFile("wg-split.conf"));
    await waitForBody(b, "File: wg-split.conf", "the preview of wg-split.conf never showed");
    await importIt(b, "wg-split");
    expect(savedProfiles()).toHaveLength(2);
    expect(savedProfiles().filter((text) => text.includes('name = "base"'))).toHaveLength(1);
    expect(savedProfiles().filter((text) => text.includes('name = "wg-split"'))).toHaveLength(1);
  });

  it("the_lab_malicious_profile_shows_the_d19_text", async () => {
    daemon("stop");
    // the folder of the first test is kept: the three profiles go to the same store
    const b = await openApp(true, "empty", [labFile("malicious.ovpn")]);
    await waitForBody(b, "File: malicious.ovpn", "the preview of malicious.ovpn never showed");
    const items = (await b.execute(() => [...document.querySelectorAll(".import-notice")].map((el) => (el.textContent ?? "").trim()))) as string[];
    expect(items.find((line) => line.includes(" up ")), `the notices were ${JSON.stringify(items)}`).toContain(D19_SCRIPT);
    expect(items.find((line) => line.includes(" plugin "))).toContain(D19_SCRIPT);
    // one notice for each line that was taken out (up, plugin, status, writepid, setenv), and the notice of the import counts the same
    expect(items).toHaveLength(5);
    await importIt(b, "malicious");
    await waitForBody(b, `Imported “malicious”. ${items.length} directives removed.`, "the notice with the number of directives never showed");
    // the import runs nothing: the files the script of the profile would have made are not there
    expect(readdirSync("/tmp").filter((name) => name.startsWith("PWNED-"))).toEqual([]);
    const saved = savedProfiles().find((text) => text.includes('name = "malicious"')) ?? "";
    expect(saved).not.toMatch(/PWNED|script-security|plugin|writepid|setenv/);
  });

  it("the_window_left_the_three_profiles_for_the_clients", async () => {
    expect(savedProfiles()).toHaveLength(3);
    const profiles = join(configDir(), "profiles");
    for (const name of readdirSync(profiles)) expect(statSync(join(profiles, name)).mode & 0o777, `${name} is private`).toBe(0o600);
    // the folder goes where `t17` finds it: target/gui-lab/config/galleria, next to a copy of nothing else
    const store = env("GALLERIA_LAB_STORE");
    rmSync(dirname(store), { recursive: true, force: true });
    mkdirSync(dirname(store), { recursive: true });
    cpSync(configDir(), store, { recursive: true });
    expect(readdirSync(join(store, "profiles"))).toHaveLength(3);
    expect(readdirSync(join(store, "secrets"))).toHaveLength(3);
  });
});
