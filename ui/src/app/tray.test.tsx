import { mockIPC } from "@tauri-apps/api/mocks";
import { act, render, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import en from "../i18n/en.json";
import it_ from "../i18n/it.json";
import { Preferences, usePrefs } from "../prefs";
import { useTrayLabelsForRust } from "./tray";

type Call = { cmd: string; args: Record<string, unknown> };
const record = (): Call[] => {
  const calls: Call[] = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args: (args ?? {}) as Record<string, unknown> });
    return undefined;
  });
  return calls;
};

function Probe({ onReady }: { onReady?: (setLang: (lang: "en" | "it") => Promise<void>) => void }) {
  useTrayLabelsForRust();
  const { setLang } = usePrefs();
  onReady?.(setLang);
  return null;
}

const sent = (calls: Call[]) => calls.filter((c) => c.cmd === "tray_labels").map((c) => c.args.labels as Record<string, string>);

describe("the texts of the tray", () => {
  it("labels_are_sent_at_start_and_on_language_change", async () => {
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
    expect(english.connect).toBe("Connect «{name}»");
    expect(english.disconnect).toBe("Disconnect «{name}»");
    expect(english.connected).toBe("Connected to «{name}»");
    expect(english.quit_connected).toBe("Quit (the VPN stays on)");
    expect(Object.keys(english).sort()).toEqual(["connect", "connected", "connecting", "disconnect", "disconnected", "open", "quit", "quit_connected", "unreachable"]);
    for (const text of Object.values(english)) expect(text.length).toBeGreaterThan(0);
    expect(english.open).toBe(en["tray.open"]);
    await act(async () => {
      await change!("it");
    });
    await waitFor(() => expect(sent(calls)).toHaveLength(2));
    const italian = sent(calls)[1];
    expect(italian.quit_connected).toBe("Esci (la VPN resta attiva)");
    expect(italian.connect).toBe("Connetti «{name}»");
    expect(italian.open).toBe(it_["tray.open"]);
  });

  it("a_refused_command_does_not_break_the_page", async () => {
    mockIPC(() => {
      throw { code: "gui_invalid_field", message: "refused" };
    });
    const { container } = render(
      <Preferences lang="en">
        <Probe />
      </Preferences>,
    );
    await waitFor(() => expect(container).toBeTruthy());
  });
});
