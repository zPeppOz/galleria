/// <reference types="node" />
import { mockIPC } from "@tauri-apps/api/mocks";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { announce } from "./announce";

const SRC = join(import.meta.dirname, "..");

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sources(path);
    return /\.(ts|tsx)$/.test(name) ? [path] : [];
  });
}

afterEach(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe("announce", () => {
  it("only_one_file_invokes_announce", () => {
    // `invoke("announce"…)`, with or without a type argument, anywhere in ui/src outside this test
    const call = new RegExp(`invoke\\s*(<[^>]*>)?\\s*\\(\\s*["'\`]${"announce"}["'\`]`);
    const users = sources(SRC)
      .filter((file) => file !== import.meta.filename)
      .filter((file) => call.test(readFileSync(file, "utf8")))
      .map((file) => relative(SRC, file));
    expect(users).toEqual([join("app", "announce.ts")]);
  });

  it("sends_the_text_and_the_urgency_to_the_command", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
    });
    announce("Saved «Homelab»", false);
    announce("Could not save", true);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(calls).toEqual([
      { cmd: "announce", args: { text: "Saved «Homelab»", assertive: false } },
      { cmd: "announce", args: { text: "Could not save", assertive: true } },
    ]);
  });

  it("a_rejected_announce_does_not_throw", async () => {
    mockIPC(() => {
      throw new Error("the command is refused");
    });
    expect(() => announce("anything", false)).not.toThrow();
    // nothing is left unhandled either: the rejection is swallowed
    await new Promise((resolve) => setTimeout(resolve, 0));
  });

  it("without_a_backend_behind_the_page_it_does_not_throw", async () => {
    expect(() => announce("anything", true)).not.toThrow();
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
});
