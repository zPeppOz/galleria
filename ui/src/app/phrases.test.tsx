import { mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { act, render, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "../App";
import { SESSION_ERROR_CODES } from "../daemon/types";
import en from "../i18n/en.json";
import it_ from "../i18n/it.json";
import { Preferences, usePrefs } from "../prefs";
import { makeDetail, makeInfo, makeRubricaView, makeStatus, makeView } from "../test/fixtures";
import { usePhrasesForRust } from "./announce";


type Call = { cmd: string; args: Record<string, unknown> };
const record = (): Call[] => {
  const calls: Call[] = [];
  const rubrica = makeRubricaView();
  mockIPC(
    (cmd, args) => {
      calls.push({ cmd, args: (args ?? {}) as Record<string, unknown> });
      if (cmd === "daemon_view") return makeView();
      if (cmd === "app_info") return makeInfo();
      if (cmd === "profiles_list") return rubrica;
      if (cmd === "profile_get") return makeDetail(rubrica.profiles[0]);
      return undefined;
    },
    { shouldMockEvents: true },
  );
  return calls;
};

function Probe({ onReady }: { onReady?: (setLang: (lang: "en" | "it") => Promise<void>) => void }) {
  usePhrasesForRust();
  const { setLang } = usePrefs();
  onReady?.(setLang);
  return null;
}

const sent = (calls: Call[]) => calls.filter((c) => c.cmd === "announce_phrases").map((c) => c.args.phrases as Record<string, unknown>);

describe("the phrases of the state announcements", () => {
  it("phrases_are_sent_at_start_and_on_language_change", async () => {
    const calls = record();
    let change: ((lang: "en" | "it") => Promise<void>) | null = null;
    render(
      <Preferences lang="en">
        <Probe onReady={(setLang) => (change = setLang)} />
      </Preferences>,
    );
    await waitFor(() => expect(sent(calls)).toHaveLength(1));
    const [english] = sent(calls);
    // the name of the profile is Rust's to put in: the placeholder reaches it as it is
    expect(english.connected).toBe("Connected to {name}");
    expect(english.disconnected).toBe("Disconnected");
    expect(english.reconnecting).toBe("Reconnecting to {name}");
    expect(english.error_other).toBe("Connection to {name} failed");
    // a text for each code a session can end with, and no other
    expect(Object.keys(english.error as object).sort()).toEqual([...SESSION_ERROR_CODES].sort());
    expect((english.error as Record<string, string>).auth_failed).toBe(`Connection to {name} failed: ${en["error.auth_failed"]}`);
    await act(async () => {
      await change!("it");
    });
    await waitFor(() => expect(sent(calls)).toHaveLength(2));
    const italian = sent(calls)[1];
    expect(italian.connected).toBe("Connesso a {name}");
    expect((italian.error as Record<string, string>).server_unreachable).toBe(`Connessione a {name} non riuscita: ${it_["error.server_unreachable"]}`);
  });

  it("state_events_do_not_call_announce", async () => {
    const calls = record();
    render(<App lang="en" />);
    await waitFor(() => expect(calls.some((c) => c.cmd === "profile_get")).toBe(true));
    for (const state of ["connecting", "connected", "reconnecting", "error", "disconnected"] as const) {
      await act(async () => {
        await emit("galleria:state", makeStatus({ state, profile: { id: "office", name: "Ufficio Napoli" }, error: state === "error" ? { code: "auth_failed", message: "x" } : null }));
      });
    }
    await new Promise((resolve) => setTimeout(resolve, 20));
    // Rust says the changes of state, from the place that emits the event: the page asks for no announcement of them
    expect(calls.filter((c) => c.cmd === "announce")).toEqual([]);
  });
});
