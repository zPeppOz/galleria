import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { DaemonView } from "./daemon/types";
import en from "./i18n/en.json";
import it_ from "./i18n/it.json";
import { detectLang } from "./i18n";
import { makeInfo, makeView } from "./test/fixtures";

const stopped: DaemonView = makeView({ link: "daemon_not_running", status: null });

function backend(view: DaemonView = stopped) {
  mockIPC((cmd) => (cmd === "daemon_view" ? view : cmd === "app_info" ? makeInfo() : undefined), { shouldMockEvents: true });
}

async function choose(user: ReturnType<typeof userEvent.setup>, name: string) {
  await user.click(screen.getByRole("button", { name: "Menu" }));
  await user.click(await screen.findByRole("menuitemradio", { name }));
  await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
}

afterEach(() => {
  vi.restoreAllMocks();
  delete document.documentElement.dataset.theme;
});

describe("preferences", () => {
  it.each([
    ["it-IT", "it"],
    ["it", "it"],
    ["en-GB", "en"],
    ["fr-FR", "en"],
  ])("system_language_is_detected_and_english_is_the_fallback: %s", (navigatorLanguage, expected) => {
    vi.spyOn(navigator, "language", "get").mockReturnValue(navigatorLanguage);
    expect(detectLang()).toBe(expected);
  });

  it("the_system_choice_follows_the_language_of_the_system", async () => {
    vi.spyOn(navigator, "language", "get").mockReturnValue("it-IT");
    backend();
    render(<App />);
    expect(await screen.findByText("Avvia il servizio")).toBeTruthy();
  });

  it("switching_language_changes_every_visible_text", async () => {
    backend();
    const user = userEvent.setup();
    render(<App lang="it" />);
    await screen.findByText("Avvia il servizio");
    const header = document.querySelector("header");
    await choose(user, "English");
    expect(await screen.findByText("Start the service")).toBeTruthy();
    // No text of the Italian catalogue survives, unless it reads the same in English.
    const shown = document.body.textContent ?? "";
    for (const [key, text] of Object.entries(it_)) {
      if (text === (en as Record<string, string>)[key] || text.includes("{")) continue;
      expect(shown, `it:${key}`).not.toContain(text);
    }
    expect(document.querySelector("header"), "App must not be unmounted").toBe(header);
  });

  it("html_lang_follows_the_language", async () => {
    backend();
    const user = userEvent.setup();
    render(<App lang="it" />);
    await screen.findByText("Avvia il servizio");
    expect(document.documentElement.lang).toBe("it");
    await choose(user, "English");
    await waitFor(() => expect(document.documentElement.lang).toBe("en"));
  });

  it("theme_choice_from_the_menu_sets_data_theme", async () => {
    backend();
    const user = userEvent.setup();
    render(<App lang="en" />);
    await screen.findByText("Start the service");
    expect(document.documentElement.dataset.theme).toBe("dark"); // no system preference: dark
    await choose(user, "Light");
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
    await choose(user, "Dark");
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("dark"));
  });

  it("a_light_system_gives_the_light_theme_until_the_user_chooses", async () => {
    const listeners: ((e: { matches: boolean }) => void)[] = [];
    vi.stubGlobal("matchMedia", () => ({
      matches: true,
      addEventListener: (_: string, l: (e: { matches: boolean }) => void) => listeners.push(l),
      removeEventListener: () => {},
    }));
    backend();
    render(<App lang="en" />);
    await screen.findByText("Start the service");
    expect(document.documentElement.dataset.theme).toBe("light");
    vi.unstubAllGlobals();
  });
});
