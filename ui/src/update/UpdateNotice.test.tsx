import { emit } from "@tauri-apps/api/event";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ToastProvider } from "../design/Toast";
import { I18n, type Lang } from "../i18n";
import { backend, type Call } from "../test/connection";
import { expectNoAccessibilityViolations } from "../test/axe";
import { UpdateNotice } from "./UpdateNotice";
import { UpdateProvider } from "./store";
import type { UpdateView } from "../daemon/commands";

const URL_OF_9 = "https://github.com/zPeppOz/galleria/releases/tag/v9.9.9";
const NEWER: UpdateView = { current: "0.1.0", latest: "9.9.9", url: URL_OF_9, enabled: true };
const NOTHING: UpdateView = { current: "0.1.0", latest: null, url: null, enabled: true };

/** What Rust keeps: the view it answers, and what the patches of the page do to it. */
function rust(start: UpdateView) {
  let view = start;
  const calls: Call[] = backend((call) => {
    if (call.cmd === "update_view") return view;
    if (call.cmd === "ui_state_set") {
      const patch = call.args.patch as { update_check?: boolean; update_dismissed?: string };
      if (patch.update_check !== undefined) view = patch.update_check ? { ...view, enabled: true } : { ...view, enabled: false, latest: null, url: null };
      if (patch.update_dismissed === view.latest) view = { ...view, latest: null, url: null };
      return {};
    }
    return undefined;
  });
  return { calls, set: (next: UpdateView) => (view = next) };
}

const mount = (lang: Lang = "en") =>
  render(
    <I18n lang={lang}>
      <ToastProvider closeLabel="Dismiss the notice">
        <UpdateProvider>
          <main>
            <UpdateNotice />
          </main>
        </UpdateProvider>
      </ToastProvider>
    </I18n>,
  );

const notice = () => screen.queryByRole("region", { name: /is available$/ });
const toasts = () => document.querySelector<HTMLElement>(".gx-toasts [role=status]");

afterEach(() => vi.restoreAllMocks());

describe("the notice of a newer version (plan B6, T5, P11, Q55)", () => {
  it("the_notice_names_both_versions_and_shows_the_address_as_text", async () => {
    const calls = rust(NEWER).calls;
    mount();
    const region = await screen.findByRole("region", { name: /is available$/ });
    expect(region.textContent).toContain("Galleria 9.9.9 is available");
    expect(region.textContent).toContain("0.1.0");
    // the address is text, to read and to select: no link, and nothing in the page that opens it
    expect(within(region).getByText(URL_OF_9)).toBeTruthy();
    expect(region.querySelector("a")).toBeNull();
    expect(region.querySelector("[href]")).toBeNull();
    // the page asked for the view once, and for nothing else
    expect(calls.map((c) => c.cmd)).toEqual(["update_view"]);
  });

  it("the_notice_comes_in_italian_when_the_language_is", async () => {
    rust(NEWER);
    mount("it");
    const region = await screen.findByRole("region", { name: "È disponibile Galleria 9.9.9" });
    expect(region.textContent).toContain("È disponibile Galleria 9.9.9");
    expect(within(region).getByRole("button", { name: "Copia link" })).toBeTruthy();
    expect(within(region).getByRole("button", { name: "Non ricordarmelo" })).toBeTruthy();
  });

  it("copy_copies_the_address_and_says_so_once", async () => {
    const user = userEvent.setup();
    rust(NEWER);
    const written: string[] = [];
    // `userEvent.setup()` puts its own clipboard in place: the spy goes after it
    vi.spyOn(navigator.clipboard, "writeText").mockImplementation(async (text) => void written.push(text));
    mount();
    await user.click(await screen.findByRole("button", { name: "Copy link" }));
    await waitFor(() => expect(written).toEqual([URL_OF_9]));
    await waitFor(() => expect(toasts()?.textContent).toBe("Link copied."));
    expect(document.querySelectorAll(".gx-toasts [role=status]")).toHaveLength(1);
    // the notice stays: copying is not an answer
    expect(notice()).toBeTruthy();
  });

  it("a_copy_that_fails_says_so", async () => {
    const user = userEvent.setup();
    rust(NEWER);
    vi.spyOn(navigator.clipboard, "writeText").mockRejectedValue(new Error("no clipboard"));
    mount();
    await user.click(await screen.findByRole("button", { name: "Copy link" }));
    await waitFor(() => expect(document.body.textContent).toContain("The link could not be copied."));
  });

  it("dismiss_hides_it_and_writes_the_version", async () => {
    const user = userEvent.setup();
    const { calls } = rust(NEWER);
    mount();
    await user.click(await screen.findByRole("button", { name: "Don't remind me" }));
    await waitFor(() => expect(notice()).toBeNull());
    // what was written is the number of the version, and only that
    expect(calls.filter((c) => c.cmd === "ui_state_set").map((c) => c.args)).toEqual([{ patch: { update_dismissed: "9.9.9" } }]);
  });

  it("no_notice_when_the_check_is_off_or_nothing_is_newer", async () => {
    rust({ ...NEWER, enabled: false });
    const off = mount();
    await waitFor(() => expect(document.querySelector("main")).toBeTruthy());
    await act(async () => undefined);
    expect(notice()).toBeNull();
    off.unmount();
    rust(NOTHING);
    mount();
    await act(async () => undefined);
    expect(notice()).toBeNull();
    // a page that is told nothing (no backend, an answer that is not a view) shows nothing either
    backend(() => "not a view");
    render(
      <I18n lang="en">
        <ToastProvider closeLabel="x">
          <UpdateProvider>
            <UpdateNotice />
          </UpdateProvider>
        </ToastProvider>
      </I18n>,
    );
    await act(async () => undefined);
    expect(notice()).toBeNull();
  });

  it("the_notice_appears_when_rust_says_a_version_was_found_and_goes_when_the_check_is_turned_off", async () => {
    const { set } = rust(NOTHING);
    mount();
    await act(async () => undefined);
    expect(notice()).toBeNull();
    // the event of Rust carries the view, built by Rust
    set(NEWER);
    await act(async () => emit("galleria:update", NEWER));
    await screen.findByRole("region", { name: /is available$/ });
    // an event that is not a view is not heard
    await act(async () => emit("galleria:update", { latest: 9, url: 3 }));
    expect(notice()).toBeTruthy();
    // the check turned off in Rust (the menu of the app does it) takes the notice away at once
    set({ ...NEWER, enabled: false, latest: null, url: null });
    await act(async () => emit("galleria:update", { ...NEWER, enabled: false, latest: null, url: null }));
    await waitFor(() => expect(notice()).toBeNull());
  });

  it("the_notice_is_not_a_live_region", async () => {
    rust(NEWER);
    mount();
    const region = await screen.findByRole("region", { name: /is available$/ });
    // Q27, rule 3: it is text that Tab finds, not something that speaks by itself
    const live = ["[role=status]", "[role=alert]", "[role=log]", "[aria-live]"].flatMap((selector) => [...region.querySelectorAll(selector)]);
    expect(live).toEqual([]);
    for (const attribute of ["role", "aria-live"]) expect(region.getAttribute(attribute)).toBeNull();
  });

  it("has_no_accessibility_violations", async () => {
    rust(NEWER);
    const { container } = mount();
    await screen.findByRole("region", { name: /is available$/ });
    await expectNoAccessibilityViolations(container, { landmarks: false });
  });
});
