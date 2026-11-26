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
import { appendFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { afterAll, afterEach, beforeAll, describe, expect, it } from "vitest";
import { barState, bodyText, cell, choose, closeApp, env, openApp, TEN_SECONDS, waitFor, waitForBody, type Browser } from "./support";

afterEach(closeApp);

const CARLA = env("GALLERIA_LAB_SOCKET");
const USED_OTP = env("GALLERIA_LAB_USED_OTP");
const RUN_OUT = env("GALLERIA_LAB_RUN_OUT");
const THIRTY_SECONDS = 30_000;

const labFile = (name: string): string => {
  const path = join(env("GALLERIA_LAB_OUT"), name);
  if (!existsSync(path)) throw new Error(`${path} is not there: the laboratory makes it (lab/gen-pki.sh writes the .ovpn files, lab/up.sh starts the WireGuard server that writes wg-split.conf)`);
  return path;
};

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
});

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

