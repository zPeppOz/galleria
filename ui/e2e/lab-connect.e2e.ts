// G4, G5, G6, G7 and G9 in the laboratory (plan B5, T9 and T10, Q42 (b)): the real window, in the `galleria-gui-dev` image, drives the REAL
// daemon of a client of the laboratory through the bridge of lab/tests/gui-daemon-relay.py. The window is `carla`, a user of the group
// `galleria` that the laboratory makes at test time; `alice`, the user that is already there, is the other one (G9). The profiles of the
// laboratory (`lab/out`) enter through the window, as in B4, except the one that G6a imports from the CLI.
//
// Not part of `scripts/gui.sh smoke`: vitest.config.ts runs this file only when GALLERIA_LAB_CONNECT is set (`scripts/gui.sh
// lab-connect`, lab/tests/gui-lab-connect.sh), because it needs the laboratory up and the bridges. What the client saw while this ran
// is checked by crates/galleria-e2e/tests/t18_gui_connect.rs, and `scripts/check-gates.py --job lab-connect` wants every proof of the
// manifest here with the status `passed`.
//
// Nothing here puts a secret in an argument or in a variable of a process: the credentials go into the fields of the window, the CLI
// takes the password on stdin.
import { execFileSync, spawnSync } from "node:child_process";
import { createHmac } from "node:crypto";
import { appendFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, afterEach, beforeAll, describe, expect, it } from "vitest";
import { answerFileWindow, appRunning, barState, bodyText, cell, choose, closeApp, env, keyringItems, keyringLookup, openApp, profileId, requestClose, startTrayWatcher, stopTrayWatcher, TEN_SECONDS, trayClick, trayRegistered, trayTexts, until, waitFor, waitForBody, windowState, type Browser } from "./support";

afterEach(async () => {
  await closeApp();
  stopTrayWatcher();
});

const CARLA = env("GALLERIA_LAB_SOCKET");
const ALICE = env("GALLERIA_LAB_SOCKET_OTHER");
const USED_OTP = env("GALLERIA_LAB_USED_OTP");
const RUN_OUT = env("GALLERIA_LAB_RUN_OUT");
const THIRTY_SECONDS = 30_000;

const labFile = (name: string): string => {
  const path = join(env("GALLERIA_LAB_OUT"), name);
  if (!existsSync(path)) throw new Error(`${path} is not there: the laboratory makes it (lab/gen-pki.sh writes the .ovpn files, lab/up.sh starts the WireGuard server that writes wg-split.conf)`);
  return path;
};
const configDir = () => join(env("XDG_CONFIG_HOME"), "galleria");

/** The `galleria` of the phase 1, as a user of the rubrica `home` through the socket (the bridge of `carla` or of `alice`). */
function cli(socket: string, home: string, args: string[], input = ""): string {
  return execFileSync(env("GALLERIA_SMOKE_CLI"), ["--socket", socket, ...args], { encoding: "utf8", input, env: { ...process.env, XDG_CONFIG_HOME: home } });
}
const carlaCli = (...args: string[]) => cli(CARLA, env("XDG_CONFIG_HOME"), args);
const carlaStatus = () => carlaCli("status").trim();

/** Every session of the client closed, whoever started it: the next proof starts from the same point. */
async function ensureDisconnected(): Promise<void> {
  try {
    carlaCli("disconnect");
  } catch {
    // no session: nothing to close
  }
  for (let n = 0; n < 40 && carlaStatus() !== "disconnected"; n++) await new Promise((resolve) => setTimeout(resolve, 250));
  expect(carlaStatus()).toBe("disconnected");
}

// ---- The code of the second factor: the secret of the laboratory is a constant (t13, verify.sh), so the code is computed here --------

const TOTP_SECRET = "JBSWY3DPEHPK3PXP";
function base32(text: string): Buffer {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
  const bits = [...text].map((c) => alphabet.indexOf(c).toString(2).padStart(5, "0")).join("");
  const bytes = bits.match(/.{8}/g) ?? [];
  return Buffer.from(bytes.map((byte) => parseInt(byte, 2)));
}
function totp(): string {
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30_000)));
  const hash = createHmac("sha1", base32(TOTP_SECRET)).update(counter).digest();
  const offset = hash[hash.length - 1] & 0x0f;
  const value = ((hash[offset] & 0x7f) << 24) | (hash[offset + 1] << 16) | (hash[offset + 2] << 8) | hash[offset + 3];
  return String(value % 1_000_000).padStart(6, "0");
}
const usedCodes = (): string[] => (existsSync(USED_OTP) ? readFileSync(USED_OTP, "utf8").split("\n").filter((l) => l.trim() !== "") : []);

// ---- The window of the credentials ---------------------------------------------------------------------------------------------

const dialogs = (b: Browser) => b.$$('[role="dialog"]');

interface Asked {
  kind: "credentials" | "otp" | "passphrase";
  /** The `name` of every field of the window, the box «Remember» not counted. */
  fields: string[];
  remember: boolean;
  /** The label of the first field. */
  label: string;
}
interface Answers {
  user?: string;
  password?: string;
  remember?: boolean;
  passphrase?: string;
}

/**
 * Answers every window of credentials that comes up until the bar says `outcome` («Connected», or «Error» for a refusal), and
 * reports what each of them asked. A connection that asks nothing gives an empty list. The codes of the second factor are computed
 * here and written to the list of the codes used, for t18 (G7).
 */
async function answerUntil(b: Browser, outcome: "Connected" | "Error", answers: Answers = {}): Promise<Asked[]> {
  const seen: Asked[] = [];
  const deadline = Date.now() + 45_000;
  for (;;) {
    const open = await dialogs(b);
    if (open.length === 1) {
      const text = await open[0].getText();
      const kind: Asked["kind"] = /Credentials for/.test(text) ? "credentials" : /Code for/.test(text) ? "otp" : "passphrase";
      const fields = (await b.execute(() => [...document.querySelectorAll<HTMLInputElement>('[role="dialog"] input:not([type="checkbox"])')].map((el) => el.name))) as string[];
      const remember = (await b.$$('[role="dialog"] input[type="checkbox"]')).length > 0;
      const label = ((await b.execute(() => document.querySelector('[role="dialog"] label[for^="credentials-"]')?.textContent ?? "")) as string).trim();
      seen.push({ kind, fields, remember, label });
      if (kind === "credentials") {
        await b.$("#credentials-username").setValue(answers.user ?? "alice");
        await b.$("#credentials-password").setValue(answers.password ?? "alice-secret");
        if (answers.remember) await b.$('[role="dialog"] input[name="remember"]').click();
      } else if (kind === "otp") {
        const code = totp();
        appendFileSync(USED_OTP, `${code}\n`);
        await b.$("#credentials-otp").setValue(code);
      } else {
        await b.$("#credentials-passphrase").setValue(answers.passphrase ?? "key-passphrase");
      }
      await b.$('[role="dialog"] button[type="submit"]').click();
      // the window closes, or the next question takes its place with another title
      await waitFor(b, async () => (await dialogs(b)).length === 0 || (await (await dialogs(b))[0].getText()) !== text, "the window of the credentials stayed after the answer", THIRTY_SECONDS);
    } else if ((await barState(b)) === outcome) {
      return seen;
    }
    if (Date.now() > deadline) {
      const panel = (await b.$$(".status-view")).length > 0 ? (await b.$(".status-view").getText()).replace(/\s+/g, " ") : "";
      throw new Error(`the bar never said «${outcome}»; it says «${await barState(b)}» (${panel}). Asked so far: ${JSON.stringify(seen)}`);
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
}

const connectButton = (b: Browser) => b.$("button=Connect");
async function disconnectFromWindow(b: Browser): Promise<void> {
  await b.$("button=Disconnect").click();
  await waitFor(b, async () => (await barState(b)) === "Disconnected", "the bar never went back to «Disconnected»", THIRTY_SECONDS);
}

/** Chooses the profile, presses «Connect» and answers what is asked. */
async function connectProfile(b: Browser, name: string, answers: Answers = {}): Promise<Asked[]> {
  await choose(b, name);
  await connectButton(b).click();
  const seen = await answerUntil(b, "Connected", answers);
  await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", `the strip of «${name}» never said «Connected»`);
  return seen;
}

const secretFiles = (): string[] => {
  const dir = join(configDir(), "secrets");
  return existsSync(dir) ? readdirSync(dir).map((name) => readFileSync(join(dir, name), "utf8")) : [];
};

// ---- The profiles of the laboratory enter through the window -------------------------------------------------------------------

function secondLaunch(path: string): void {
  const second = spawnSync(env("GALLERIA_SMOKE_APP"), [path], { encoding: "utf8", timeout: 5000 });
  expect(second.status, `the second launch ended with ${second.status}: ${second.stderr}`).toBe(0);
}

beforeAll(async () => {
  const b = await openApp(false, "empty", [labFile("base.ovpn")]);
  await waitForBody(b, "File: base.ovpn", "the preview of base.ovpn never showed");
  const importIt = async (name: string) => {
    await b.$('input[name="import-name"]').waitForDisplayed({ timeout: TEN_SECONDS });
    expect(await b.$('input[name="import-name"]').getValue()).toBe(name);
    await b.$("button=Import").click();
    await waitForBody(b, `Imported “${name}”.`, `the notice of the import of ${name} never showed`);
  };
  await importIt("base");
  for (const [file, name] of [["wg-split.conf", "wg-split"], ["otp.ovpn", "otp"], ["enckey.ovpn", "enckey"], ["unreachable.ovpn", "unreachable"]]) {
    secondLaunch(labFile(file));
    await waitForBody(b, `File: ${file}`, `the preview of ${file} never showed`);
    await importIt(name);
  }
  await closeApp();
  mkdirSync(RUN_OUT, { recursive: true });
}, 180_000);

afterAll(async () => {
  await closeApp();
  await ensureDisconnected();
  try {
    cli(ALICE, aliceHome(), ["disconnect"]);
  } catch {
    // nothing of alice's was left
  }
});

let aliceRubrica: string | undefined;
/** The rubrica of `alice`: another `XDG_CONFIG_HOME`, so the window does not know her profile. */
const aliceHome = (): string => (aliceRubrica ??= mkdtempSync(join(tmpdir(), "galleria-alice-")));

// ================================ G4 ========================================================================================

describe("G4: connect, disconnect and replace from the window", () => {
  it("G4a_wireguard_connects_and_the_numbers_move", async () => {
    await ensureDisconnected();
    const b = await openApp(true);
    await choose(b, "wg-split");
    await connectButton(b).click();
    expect(await answerUntil(b, "Connected")).toEqual([]);
    await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", "the strip never said «Connected»");
    // the address of wg-split.conf
    const address = /^Address = ([\d.]+)\//m.exec(readFileSync(labFile("wg-split.conf"), "utf8"))?.[1] ?? "";
    expect(address).not.toBe("");
    expect(await cell(b, "strip.ip")).toContain(address);
    // the bytes, read twice three seconds apart, grow: the watch of the client pings through the tunnel
    const bytes = async () => {
      const title = (await b.execute(() => document.querySelector('[data-cell="strip.traffic"] dd span')?.getAttribute("title") ?? "")) as string;
      return title.split(" / ").map((side) => Number(side.replace(/\D/g, "")));
    };
    await waitFor(b, async () => (await bytes()).length === 2, "the traffic never showed numbers");
    const first = await bytes();
    await new Promise((resolve) => setTimeout(resolve, 3000));
    await waitFor(b, async () => {
      const now = await bytes();
      return now[0] + now[1] > first[0] + first[1];
    }, `the bytes did not grow in 3 s: they stayed at ${first.join(" / ")}`);
    // WireGuard has a handshake to show, and it has a value
    expect(await b.$('[data-cell="strip.handshake"]').isExisting()).toBe(true);
    expect(await cell(b, "strip.handshake")).not.toBe("—");
    await disconnectFromWindow(b);
  });

  it("G4b_openvpn_connects_after_the_credentials", async () => {
    await ensureDisconnected();
    const b = await openApp(true);
    const asked = await connectProfile(b, "base");
    expect(asked.map((a) => a.kind)).toEqual(["credentials"]);
    expect(await cell(b, "strip.ip")).toMatch(/^10\.8\.0\.\d+/);
    // OpenVPN has no handshake to show
    expect(await b.$('[data-cell="strip.handshake"]').isExisting()).toBe(false);
    await disconnectFromWindow(b);
  });

  it("G4c_switching_profile_replaces_without_asking", async () => {
    await ensureDisconnected();
    const b = await openApp(true);
    await connectProfile(b, "wg-split");
    // to `base` and back: one notice for each change, no question, and in the end a single session
    await choose(b, "base");
    await connectButton(b).click();
    await waitForBody(b, "Disconnected from «wg-split»", "the notice of the first change never showed");
    expect((await bodyText(b)).match(/Disconnected from «wg-split»/g) ?? []).toHaveLength(1);
    await answerUntil(b, "Connected");
    expect(await b.$$('[role="alertdialog"]'), "a confirmation was asked").toHaveLength(0);
    await choose(b, "wg-split");
    await connectButton(b).click();
    await waitForBody(b, "Disconnected from «base»", "the notice of the second change never showed");
    expect((await bodyText(b)).match(/Disconnected from «base»/g) ?? []).toHaveLength(1);
    await answerUntil(b, "Connected");
    expect(await b.$$('[role="alertdialog"]'), "a confirmation was asked").toHaveLength(0);
    await waitFor(b, async () => (await b.$(".status-bar-session").getText()).includes("«wg-split»"), "the bar never showed the profile of the last change");
    await choose(b, "base");
    expect(await cell(b, "strip.state"), "the first profile has no session any more").not.toBe("Connected");
    expect(carlaStatus()).toBe("connected");
    await ensureDisconnected();
  });

  it("G4d_disconnect_returns_to_disconnected", async () => {
    await ensureDisconnected();
    const b = await openApp(true);
    await connectProfile(b, "wg-split");
    expect(carlaStatus()).toBe("connected");
    await disconnectFromWindow(b);
    // the CLI of the image, through the same bridge, says the same
    expect(carlaStatus()).toBe("disconnected");
  });

  it("G4e_cancel_while_connecting_is_a_disconnect", async () => {
    await ensureDisconnected();
    const b = await openApp(true);
    await choose(b, "unreachable");
    await connectButton(b).click();
    await waitFor(b, async () => (await barState(b)) === "Connecting", "the bar never said «Connecting»");
    await b.$("button=Cancel").click();
    await waitFor(b, async () => (await barState(b)) === "Disconnected", "«Cancel» did not end in «Disconnected»", TEN_SECONDS);
    expect(carlaStatus()).toBe("disconnected");
  });
});

// ================================ G5 ========================================================================================

describe("G5: the credentials, only what is asked, and the keyring", () => {
  it("G5a_only_the_asked_field_is_shown", async () => {
    await ensureDisconnected();
    const b = await openApp(true);
    // base: the user and the password, no code
    const base = await connectProfile(b, "base");
    expect(base).toHaveLength(1);
    expect(base[0]).toMatchObject({ kind: "credentials", fields: ["username", "password"], remember: true });
    await disconnectFromWindow(b);
    // otp: the credentials, and then a second window with the code and nothing else, labelled with the prompt of the server
    const otp = await connectProfile(b, "otp", { user: "bob", password: "bob-secret" });
    expect(otp.map((a) => a.kind)).toEqual(["credentials", "otp"]);
    expect(otp[1]).toMatchObject({ fields: ["otp"], remember: false, label: "Enter OTP" });
    await disconnectFromWindow(b);
    // enckey: the credentials, and the passphrase alone
    const enckey = await connectProfile(b, "enckey");
    expect(enckey.map((a) => a.kind).sort()).toEqual(["credentials", "passphrase"]);
    expect(enckey.find((a) => a.kind === "passphrase")).toMatchObject({ fields: ["passphrase"], remember: false });
    await disconnectFromWindow(b);
  });

  it("G5b_remember_saves_to_the_keyring_and_the_next_connection_does_not_ask", async () => {
    await ensureDisconnected();
    const b = await openApp(true);
    const id = profileId("base");
    await connectProfile(b, "base", { remember: true });
    await waitFor(b, async () => keyringLookup(id, "openvpn.password") === "alice-secret", `the keyring has no password for the profile: ${keyringItems(id)}`);
    expect(keyringLookup(id, "openvpn.username")).toBe("alice");
    // no file of the rubrica holds the password, and the profile only names the two entries
    expect(secretFiles().filter((text) => text.includes("alice-secret"))).toEqual([]);
    const profile = readFileSync(join(configDir(), "profiles", `${id}.toml`), "utf8");
    expect(profile).toContain("openvpn.password");
    expect(profile).not.toContain("alice-secret");
    await disconnectFromWindow(b);
    // the next one does not ask
    await connectButton(b).click();
    expect(await answerUntil(b, "Connected"), "a window asked for what the keyring holds").toEqual([]);
    await disconnectFromWindow(b);
  });

  it("G5c_the_otp_is_never_saved", async () => {
    await ensureDisconnected();
    const b = await openApp(true);
    const id = profileId("otp");
    await connectProfile(b, "otp", { user: "bob", password: "bob-secret", remember: true });
    await waitFor(b, async () => keyringLookup(id, "openvpn.password") === "bob-secret", `the keyring has no password for the profile: ${keyringItems(id)}`);
    expect(keyringLookup(id, "openvpn.username")).toBe("bob");
    const items = keyringItems(id);
    expect(usedCodes().length).toBeGreaterThan(0);
    for (const code of usedCodes()) expect(items, `a code that was used is in the keyring: ${code}`).not.toContain(code);
    expect(keyringLookup(id, "openvpn.otp")).toBe("");
    await disconnectFromWindow(b);
    // the connection after asks only the code
    await connectButton(b).click();
    expect((await answerUntil(b, "Connected")).map((a) => a.kind)).toEqual(["otp"]);
    await disconnectFromWindow(b);
  });

  it("G5d_the_passphrase_is_never_saved", async () => {
    await ensureDisconnected();
    const b = await openApp(true);
    const id = profileId("enckey");
    for (const turn of [1, 2]) {
      let asked: Asked[];
      if (turn === 1) {
        asked = await connectProfile(b, "enckey");
      } else {
        await connectButton(b).click();
        asked = await answerUntil(b, "Connected");
      }
      expect(asked.map((a) => a.kind), `turn ${turn}`).toContain("passphrase");
      await disconnectFromWindow(b);
    }
    expect(keyringItems(id)).not.toContain("key-passphrase");
    expect(`${spawnSync("secret-tool", ["search", "--all", "service", "galleria"], { encoding: "utf8" }).stdout}`).not.toContain("key-passphrase");
  });

  it("G5e_a_refused_saved_password_offers_to_forget_it", async () => {
    await ensureDisconnected();
    const b = await openApp(true);
    const id = profileId("base");
    // G5b left the credentials in the keyring: «Forget» takes them out, and the connection asks again
    await choose(b, "base");
    await b.$("button=Authentication").click();
    await b.$("button=Forget").click();
    await waitFor(b, async () => keyringLookup(id, "openvpn.password") === "" && keyringLookup(id, "openvpn.username") === "", "«Forget» left the credentials in the keyring");
    // a wrong password with «Remember»: the error, translated, and nothing is saved
    await connectButton(b).click();
    expect((await answerUntil(b, "Error", { password: "not-the-password", remember: true })).map((a) => a.kind)).toEqual(["credentials"]);
    await waitForBody(b, "Authentication failed", "the error of the refused password was never shown");
    expect(keyringLookup(id, "openvpn.password"), "a password that failed was saved").toBe("");
    await b.$("button=Close").click();
    await waitFor(b, async () => (await barState(b)) === "Disconnected", "«Close» did not leave the error");
    // the right one is remembered, and a wrong one is put in its place by hand, as secret-tool would (from stdin)
    await connectButton(b).click();
    await answerUntil(b, "Connected", { remember: true });
    await waitFor(b, async () => keyringLookup(id, "openvpn.password") === "alice-secret", "the password was not saved");
    await disconnectFromWindow(b);
    const stored = spawnSync("secret-tool", ["store", "--label=galleria lab: a password that the server refuses", "service", "galleria", "profile", id, "key", "openvpn.password"], { input: "wrong-saved-password", encoding: "utf8" });
    expect(stored.status, stored.stderr).toBe(0);
    expect(keyringLookup(id, "openvpn.password")).toBe("wrong-saved-password");
    // the saved one is sent without asking, refused, and the error offers to forget it
    await connectButton(b).click();
    expect(await answerUntil(b, "Error"), "a window asked while the keyring held the credentials").toEqual([]);
    await waitForBody(b, "Authentication failed", "the error of the refused saved password was never shown");
    await b.$("button=Forget the saved credentials").click();
    await waitFor(b, async () => keyringLookup(id, "openvpn.password") === "", "the password is still in the keyring after «Forget the saved credentials»");
    // and the connection after asks
    await b.$("button=Retry").click();
    expect((await answerUntil(b, "Connected")).map((a) => a.kind)).toEqual(["credentials"]);
    await disconnectFromWindow(b);
  });
});

// ================================ G6 ========================================================================================

describe("G6: a profile imported on one side connects from the other", () => {
  it("G6a_a_profile_imported_by_the_cli_connects_from_the_window", async () => {
    await ensureDisconnected();
    // the CLI of the image, in the rubrica of the window: the secrets go to the file
    const imported = carlaCli("import", labFile("wg-split.conf"), "--name", "wg-from-cli");
    const id = imported.trim().split(/\s+/)[0];
    expect(id, `the CLI said: ${imported}`).toMatch(/^[0-9a-f-]{36}$/);
    expect(secretFiles().some((text) => text.includes("PrivateKey") || text.includes("wireguard.private_key")), "the secrets of the profile are not in the file").toBe(true);
    expect(keyringItems(id), "the CLI wrote to the keyring").toBe("");
    // the window lists it and connects it
    const b = await openApp(true);
    await connectProfile(b, "wg-from-cli");
    expect(carlaStatus()).toBe("connected");
    await disconnectFromWindow(b);
  });

  it("G6b_a_profile_imported_by_the_window_connects_from_the_cli", async () => {
    await ensureDisconnected();
    const id = profileId("base");
    // imported by the window: the key is in the keyring and the files of the rubrica have none
    expect(keyringItems(id)).toContain("openvpn.block.key");
    expect(secretFiles().filter((text) => text.includes("PRIVATE KEY"))).toEqual([]);
    expect(existsSync(join(configDir(), "secrets", `${id}.toml`))).toBe(false);
    expect(cli(CARLA, env("XDG_CONFIG_HOME"), ["connect", "base", "--username", "alice", "--password-stdin"], "alice-secret\n").length).toBeGreaterThan(0);
    expect(carlaStatus()).toBe("connected");
    // the window shows the session as its own
    const b = await openApp(true);
    await choose(b, "base");
    await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", "the window never showed the session of the CLI");
    expect(await b.$('[role="group"][aria-label="Session started outside this rubrica"]').isExisting(), "the session of the CLI was shown as foreign").toBe(false);
    expect(await bodyText(b)).not.toContain("started outside this rubrica");
    await disconnectFromWindow(b);
  });
});

// ================================ G7 ========================================================================================

/** Every process of this user: what it was started with and its environment. */
function processText(): string[] {
  const out: string[] = [];
  for (const pid of readdirSync("/proc").filter((name) => /^\d+$/.test(name))) {
    for (const file of ["cmdline", "environ"]) {
      try {
        out.push(readFileSync(join("/proc", pid, file), "latin1"));
      } catch {
        // not ours, or gone
      }
    }
  }
  return out;
}

describe("G7: no known secret leaves the keyring and the connection", () => {
  it("G7a_no_known_secret_reaches_the_log_the_export_or_the_stderr", async () => {
    await ensureDisconnected();
    const b = await openApp(true);
    const lines = async () => (await b.execute(() => [...document.querySelectorAll(".log-line")].map((el) => (el.textContent ?? "").trim()))) as string[];
    const page: string[] = [];
    // the profile with the second factor and the one with the passphrase: the log of each, while it is up and after
    await connectProfile(b, "otp", { user: "bob", password: "bob-secret" });
    await b.$("button=Log").click();
    await waitFor(b, async () => (await lines()).length > 0, "the tab «Log» of otp never showed a line");
    page.push(...(await lines()));
    await disconnectFromWindow(b);
    await waitFor(b, async () => (await lines()).length > 0, "the tab «Log» of otp shows nothing after the session");
    page.push(...(await lines()));
    await connectProfile(b, "enckey");
    await b.$("button=Log").click();
    await waitFor(b, async () => (await lines()).length > 0, "the tab «Log» of enckey never showed a line");
    page.push(...(await lines()));
    // «Export log…» through the real window «Save», driven over AT-SPI without Orca (S41)
    const exported = join(RUN_OUT, "exported-log.txt");
    await b.execute(() => [...document.querySelectorAll<HTMLElement>("button")].find((el) => /^Export log/.test(el.textContent ?? ""))?.focus());
    void b.keys(["Enter"]).catch(() => undefined);
    expect(await answerFileWindow({ save: exported })).toMatchObject({ pressed: "Save", closed: true });
    await waitForBody(b, "Log exported.", "the export was never said to be done");
    expect(statSync(exported).mode & 0o777).toBe(0o600);
    const file = readFileSync(exported, "utf8");
    expect(file, "the exported file is empty").toMatch(/^\d+ \w+ \w+: /m);
    await disconnectFromWindow(b);

    // what the laboratory knows and must not find
    const ovpn = readFileSync(labFile("base.ovpn"), "utf8");
    const keyLine = ovpn.split("<key>")[1]?.split("\n").find((l) => l.length > 40 && !l.startsWith("-")) ?? "";
    const wgKey = /^PrivateKey = (.+)$/m.exec(readFileSync(labFile("wg-split.conf"), "utf8"))?.[1]?.trim() ?? "";
    expect(keyLine).not.toBe("");
    expect(wgKey).not.toBe("");
    expect(usedCodes().length, "no code was used: the profile with the second factor never connected").toBeGreaterThan(0);
    const secrets = ["alice-secret", "bob-secret", "key-passphrase", ...usedCodes(), keyLine, wgKey];
    const stderr = existsSync(join(RUN_OUT, "app.stderr")) ? readFileSync(join(RUN_OUT, "app.stderr"), "utf8") : "";
    expect(existsSync(join(RUN_OUT, "app.stderr")), "the stderr of the app was not kept: lab/tests/gui-lab-connect.sh starts it through a shim that does").toBe(true);
    const places: [string, string][] = [
      ["the tab «Log»", page.join("\n")],
      ["the file of «Export log…»", file],
      ["the stderr of the app", stderr],
      ...processText().map((text): [string, string] => ["the arguments or the environment of a process", text]),
    ];
    expect(page.length, "the tab «Log» had no line to look at").toBeGreaterThan(2);
    for (const [where, text] of places) {
      for (const secret of secrets) expect(text.includes(secret), `a known secret is in ${where}`).toBe(false);
    }
  });
});

// ================================ G9 ========================================================================================

describe("G9: a session of another user of the group", () => {
  const GROUP = '[role="group"][aria-label="Session started outside this rubrica"]';
  const alertDialogs = (b: Browser) => b.$$('[role="alertdialog"]');
  const aliceStatus = () => cli(ALICE, aliceHome(), ["status"]).trim();
  /** The name of the profile of the session that is up, whoever started it. */
  const active = () => (JSON.parse(cli(ALICE, aliceHome(), ["status", "--json"])) as { profile: { name: string } | null }).profile?.name ?? null;
  const aliceConnects = () => {
    cli(ALICE, aliceHome(), ["connect", "alice-wg"]);
    expect(aliceStatus()).toBe("connected");
    expect(active()).toBe("alice-wg");
  };

  it("G9a_a_session_started_by_another_user_is_shown_as_foreign", async () => {
    await ensureDisconnected();
    // `alice`, from her own rubrica and through her own bridge: the daemon sees another uid of the group
    cli(ALICE, aliceHome(), ["import", labFile("wg-split.conf"), "--name", "alice-wg"]);
    aliceConnects();
    const b = await openApp(true);
    await waitFor(b, async () => await b.$(GROUP).isExisting(), "the list never showed the fixed row of the session of someone else", THIRTY_SECONDS);
    const text = (await b.execute((sel: string) => document.querySelector(sel)?.textContent ?? "", GROUP)) as string;
    expect(text).toContain("alice-wg");
    expect(text).toMatch(/started outside this rubrica/i);
    await waitFor(b, async () => (await b.$(".status-bar-session").isExisting()) && (await b.$(".status-bar-session").getText()).includes("Active connection: «alice-wg» — started outside this rubrica"), "the bar never said whose the session is");
    // the detail is a session and not a profile; its log has lines
    await b.$(`${GROUP} button[aria-describedby]`).click();
    await waitFor(b, async () => (await b.$("h1").getText()) === "alice-wg", "the detail never showed the session of alice");
    await b.$("button=Log").click();
    await waitFor(b, async () => (await b.$$(".log-line")).length > 0, "the log of the session of someone else has no line");
  });

  it("G9b_disconnecting_it_asks_first", async () => {
    // the session of G9a is still up: a window of its own, and no one closes it before
    expect(aliceStatus()).toBe("connected");
    const b = await openApp(true);
    await waitFor(b, async () => await b.$(GROUP).isExisting(), "the list never showed the fixed row of the session of someone else", THIRTY_SECONDS);
    await b.$(`${GROUP} button[aria-label^="Disconnect"]`).click();
    await waitFor(b, async () => (await alertDialogs(b)).length === 1, "the confirmation never showed");
    await b.$('[role="alertdialog"]').$("button=Cancel").click();
    await waitFor(b, async () => (await alertDialogs(b)).length === 0, "the confirmation never closed");
    expect(aliceStatus(), "«Cancel» ended the session of alice").toBe("connected");
    expect(active()).toBe("alice-wg");
    await b.$(`${GROUP} button[aria-label^="Disconnect"]`).click();
    await waitFor(b, async () => (await alertDialogs(b)).length === 1, "the confirmation never showed the second time");
    await b.$('[role="alertdialog"]').$("button=Disconnect").click();
    await waitFor(b, async () => aliceStatus() === "disconnected", "the session of alice was never ended", THIRTY_SECONDS);
  });

  it("G9c_replacing_it_asks_first", async () => {
    aliceConnects();
    const b = await openApp(true);
    await waitFor(b, async () => await b.$(GROUP).isExisting(), "the list never showed the fixed row of the session of someone else", THIRTY_SECONDS);
    await choose(b, "wg-split");
    await connectButton(b).click();
    await waitFor(b, async () => (await alertDialogs(b)).length === 1, "the confirmation of the replacement never showed");
    expect(await (await alertDialogs(b))[0].getText()).toMatch(/Replace «alice-wg»\?/);
    expect(active(), "the session of alice ended before the answer").toBe("alice-wg");
    await b.$('[role="alertdialog"]').$("button=Replace").click();
    await waitFor(b, async () => (await cell(b, "strip.state")) === "Connected", "the profile of carla never connected after the replacement", THIRTY_SECONDS);
    expect(active(), "the session that is up is not the one of carla").toBe("wg-split");
    await disconnectFromWindow(b);
  });
});

// ================================ G10 =======================================================================================

/**
 * What the window says to the watch of the client (lab/tests/gui-lab-watch.sh) and, through it, to t18 (`gui_closed_window_kept_the_tunnel`): a line
 * `<ms> <name>` in `marks`, in the folder that the client sees as `/lab/target/gui-lab`. The watch copies it to its register as `mark <ms> <name>`;
 * the time is the one of this machine, which the container of the client shares.
 */
const mark = (name: string): void => appendFileSync(join(RUN_OUT, "marks"), `${Date.now()} ${name}\n`);
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

describe("G10: the tunnel of the real daemon, with the window closed", () => {
  it("G10g_the_tunnel_survives_the_closed_window", async () => {
    await ensureDisconnected();
    // the probe stands in for the desktop that shows the tray, and starts before the app: without it a closed window is a quit (Q53)
    await startTrayWatcher();
    const b = await openApp(true);
    expect(trayRegistered(), "the app never registered a tray").toBe(true);
    await connectProfile(b, "wg-split");
    expect(carlaStatus()).toBe("connected");
    await until(trayTexts, (texts) => texts[0] === "Connected to «wg-split»", "the menu of the tray never said the session of the real daemon");

    mark("closed");
    requestClose();
    await until(windowState, (shown) => shown === "false", "the window was not hidden by the request to close it");
    // ten seconds, looked at every second from three sides: the daemon says connected, the app is alive and hidden, the menu says so
    for (let second = 1; second <= 10; second++) {
      await sleep(1000);
      expect(carlaStatus(), `the daemon lost the session ${second} s after the window was closed`).toBe("connected");
      expect(appRunning(), `the app ended ${second} s after the window was closed`).toBe(true);
      expect(windowState()).toBe("false");
    }
    expect(trayTexts()[0]).toBe("Connected to «wg-split»");

    // the tunnel is a real one: with the window shown again the bytes of the strip grow (the watch of the client pings through it)
    trayClick("Open Galleria");
    await until(windowState, (shown) => shown === "true", "«Open Galleria» did not show the window");
    const bytes = async () => {
      const title = (await b.execute(() => document.querySelector('[data-cell="strip.traffic"] dd span')?.getAttribute("title") ?? "")) as string;
      return title.split(" / ").map((side) => Number(side.replace(/\D/g, "")));
    };
    await waitFor(b, async () => (await bytes()).length === 2, "the traffic never showed numbers after the window was shown again");
    const first = await bytes();
    await waitFor(b, async () => {
      const now = await bytes();
      return now[0] + now[1] > first[0] + first[1];
    }, `the bytes did not grow after the window came back: they stayed at ${first.join(" / ")}`, THIRTY_SECONDS);

    // the end is the menu alone, with the window hidden again
    requestClose();
    await until(windowState, (shown) => shown === "false", "the window was not hidden the second time");
    mark("disconnect");
    trayClick("Disconnect «wg-split»");
    await until(carlaStatus, (status) => status === "disconnected", "the session did not end from the menu of the tray", THIRTY_SECONDS);
    mark("disconnected");
    expect(appRunning()).toBe(true);
  });
});
