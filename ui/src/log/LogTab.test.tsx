import { emit } from "@tauri-apps/api/event";
import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { LogEntry, VpnState } from "../daemon/types";
import { VPN_STATES } from "../daemon/types";
import { backend, renderConnected } from "../test/connection";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeStatus, makeView } from "../test/fixtures";
import { LogTab } from "./LogTab";
import { MAX_LINES, formatLine, mergeLines } from "./store";

const line = (seq: number, over: Partial<LogEntry> = {}): LogEntry => ({ seq, ts: String(1_795_000_000_000 + seq * 1000), level: "info", source: "daemon", message: `line ${seq}`, ...over });
const session = (state: VpnState, id = "homelab") => makeView({ status: makeStatus({ state, session: 4, profile: { id, name: "Homelab" }, protocol: "wireguard" }) });
/** The service answers the pages of `buffer`, as it does: after a number, at most `limit`. */
const serving = (buffer: LogEntry[]) =>
  backend((call) => {
    if (call.cmd === "logs_get") {
      const { after, limit } = call.args as { after: number | null; limit: number };
      const entries = buffer.filter((e) => after === null || e.seq > after).slice(0, limit);
      return { entries, last: entries.length > 0 ? entries[entries.length - 1].seq : (after ?? 0) };
    }
    return undefined;
  });
const log = () => screen.getByRole("log");
const texts = () => [...log().querySelectorAll(".log-message")].map((el) => el.textContent);

describe("LogTab", () => {
  it.each(VPN_STATES)("the_title_follows_the_state: %s", async (state) => {
    serving([line(1)]);
    const { show } = renderConnected(<LogTab profileId="homelab" />, session(state === "disconnected" ? "connected" : state));
    if (state === "disconnected") {
      show(makeView({ status: makeStatus() }));
      expect(await screen.findByRole("heading", { name: "Last session" })).toBeTruthy();
    } else {
      expect(await screen.findByRole("heading", { name: "Current session" })).toBeTruthy();
    }
    await waitFor(() => expect(texts()).toEqual(["line 1"]));
  });

  it("the_last_session_keeps_its_lines_when_the_session_ends", async () => {
    const calls = serving([line(1), line(2)]);
    const { show } = renderConnected(<LogTab profileId="homelab" />, session("connected"));
    await waitFor(() => expect(texts()).toEqual(["line 1", "line 2"]));
    show(makeView({ status: makeStatus() }));
    expect(await screen.findByRole("heading", { name: "Last session" })).toBeTruthy();
    expect(texts()).toEqual(["line 1", "line 2"]);
    expect(calls.filter((c) => c.cmd === "logs_get")).toHaveLength(1);
  });

  it("live_lines_are_appended_and_duplicates_dropped", async () => {
    serving([line(1), line(2)]);
    renderConnected(<LogTab profileId="homelab" />, session("connected"));
    await waitFor(() => expect(texts()).toEqual(["line 1", "line 2"]));
    await act(async () => {
      await emit("galleria:log", line(3));
      // a line that the page already has comes again: it is not shown twice
      await emit("galleria:log", line(2));
      await emit("galleria:log", line(4));
    });
    await waitFor(() => expect(texts()).toEqual(["line 1", "line 2", "line 3", "line 4"]));
  });

  it("a_line_that_comes_before_the_page_that_has_it_still_ends_up_in_its_place", () => {
    const merged = mergeLines([line(5), line(6)], [line(2), line(6), line(3)]);
    expect(merged.map((e) => e.seq)).toEqual([2, 3, 5, 6]);
    expect(mergeLines(merged, [])).toBe(merged);
    expect(mergeLines(merged, [line(3)])).toBe(merged);
  });

  it("at_most_two_thousand_lines", async () => {
    const buffer = Array.from({ length: 2000 }, (_, at) => line(at + 1));
    const calls = serving(buffer);
    renderConnected(<LogTab profileId="homelab" />, session("connected"));
    await waitFor(() => expect(log().querySelectorAll(".log-line")).toHaveLength(2000));
    // two full pages, then a call that finds nothing more
    expect(calls.filter((c) => c.cmd === "logs_get").map((c) => c.args)).toEqual([
      { after: null, limit: 1000 },
      { after: 1000, limit: 1000 },
      { after: 2000, limit: 1000 },
    ]);
    await act(async () => {
      for (let seq = 2001; seq <= 2010; seq += 1) await emit("galleria:log", line(seq));
    });
    await waitFor(() => expect(texts().at(-1)).toBe("line 2010"));
    expect(texts()).toHaveLength(MAX_LINES);
    expect(texts()[0]).toBe("line 11");
  });

  it("lines_are_not_translated", async () => {
    serving([line(1, { message: "Initialization Sequence Completed" })]);
    renderConnected(<LogTab profileId="homelab" />, session("connected"), "it");
    await waitFor(() => expect(texts()).toEqual(["Initialization Sequence Completed"]));
    expect(screen.getByRole("heading", { name: "Sessione corrente" })).toBeTruthy();
  });

  it("level_and_source_are_text", async () => {
    serving([line(1, { level: "warn", source: "openvpn", message: "x" })]);
    renderConnected(<LogTab profileId="homelab" />, session("connected"));
    await waitFor(() => expect(texts()).toEqual(["x"]));
    expect(log().querySelector(".log-level")?.textContent).toBe("warn");
    expect(log().querySelector(".log-source")?.textContent).toBe("openvpn");
  });

  it("the_log_is_not_a_live_region_that_announces", async () => {
    const calls = serving([line(1)]);
    const { container } = renderConnected(<LogTab profileId="homelab" />, session("connected"));
    await waitFor(() => expect(texts()).toHaveLength(1));
    await act(async () => {
      for (let seq = 2; seq < 8; seq += 1) await emit("galleria:log", line(seq));
    });
    await waitFor(() => expect(texts()).toHaveLength(7));
    expect(log().getAttribute("aria-live")).toBe("off");
    expect(container.querySelector(".log-view")!.querySelector('[aria-live="assertive"], [aria-live="polite"]')).toBeNull();
    expect(container.querySelector(".log-view")!.querySelector('[role="alert"], [role="status"]')).toBeNull();
    expect(calls.some((c) => c.cmd === "announce")).toBe(false);
  });

  it("copy_and_export_say_the_outcome_once", async () => {
    const user = userEvent.setup();
    const calls = serving([line(1, { level: "warn", source: "wireguard", message: "handshake" }), line(2)]);
    renderConnected(<LogTab profileId="homelab" />, session("connected"));
    await waitFor(() => expect(texts()).toHaveLength(2));
    await user.click(screen.getByRole("button", { name: "Copy" }));
    expect(await screen.findAllByText("Log copied.")).toHaveLength(1);
    // what the tab shows, as the file has it
    expect(await navigator.clipboard.readText()).toBe(`${formatLine(line(1, { level: "warn", source: "wireguard", message: "handshake" }))}\n${formatLine(line(2))}\n`);
    expect(formatLine(line(1))).toBe("1795000001000 Info Daemon: line 1");

    // Rust gives the answer `{ saved }`; nothing about the place
    const exporting = backend((call) => (call.cmd === "logs_export" ? { saved: true } : call.cmd === "logs_get" ? { entries: [], last: 0 } : undefined));
    await user.click(screen.getByRole("button", { name: "Export log…" }));
    expect(await screen.findAllByText("Log exported.")).toHaveLength(1);
    expect(exporting.filter((c) => c.cmd === "logs_export")).toEqual([{ cmd: "logs_export", args: {} }]);
    // the export takes nothing from the page
    expect(calls.some((c) => c.cmd === "logs_export")).toBe(false);
  });

  it("a_cancelled_export_says_nothing_and_a_failed_one_says_why", async () => {
    const user = userEvent.setup();
    backend((call) => (call.cmd === "logs_export" ? { saved: false } : { entries: [], last: 0 }));
    renderConnected(<LogTab profileId="homelab" />, session("connected"));
    await user.click(await screen.findByRole("button", { name: "Export log…" }));
    await waitFor(() => expect(screen.queryByText("Log exported.")).toBeNull());
    expect(screen.queryByText("Log exported.")).toBeNull();
    expect(document.querySelector(".gx-toast")).toBeNull();
    backend((call) => {
      if (call.cmd === "logs_export") throw { code: "gui_export_failed", message: "The file could not be written" };
      return { entries: [], last: 0 };
    });
    await user.click(screen.getByRole("button", { name: "Export log…" }));
    expect(await screen.findByText(/could not be written|Could not/i)).toBeTruthy();
  });

  it("another_profile_shows_no_lines", async () => {
    const calls = serving([line(1)]);
    renderConnected(<LogTab profileId="somebody-else" />, session("connected"));
    expect(screen.getByText("No session for this profile.")).toBeTruthy();
    expect(screen.queryByRole("log")).toBeNull();
    expect(screen.queryByRole("button", { name: "Copy" })).toBeNull();
    await act(async () => {
      await emit("galleria:log", line(2));
    });
    expect(calls.filter((c) => c.cmd === "logs_get")).toHaveLength(0);
    expect(screen.queryByText("line 2")).toBeNull();
  });

  it("a_profile_that_never_had_a_session_has_none_to_show", () => {
    serving([]);
    renderConnected(<LogTab profileId="homelab" />, makeView());
    expect(screen.getByText("No session for this profile.")).toBeTruthy();
  });

  it("a_new_session_starts_the_list_again", async () => {
    serving([line(1)]);
    const { show } = renderConnected(<LogTab profileId="homelab" />, session("connected"));
    await waitFor(() => expect(texts()).toEqual(["line 1"]));
    serving([line(10)]);
    show(makeView({ status: makeStatus({ state: "connecting", session: 5, profile: { id: "homelab", name: "Homelab" }, protocol: "wireguard" }) }));
    await waitFor(() => expect(texts()).toEqual(["line 10"]));
  });

  it("the_list_follows_the_end_only_when_the_person_is_at_the_end", async () => {
    serving([line(1)]);
    renderConnected(<LogTab profileId="homelab" />, session("connected"));
    await waitFor(() => expect(texts()).toHaveLength(1));
    const box = log();
    // jsdom has no layout: the numbers are the ones a browser would give
    Object.defineProperty(box, "scrollHeight", { configurable: true, value: 1000 });
    Object.defineProperty(box, "clientHeight", { configurable: true, value: 300 });
    box.scrollTop = 0;
    act(() => {
      box.dispatchEvent(new Event("scroll"));
    });
    await act(async () => {
      await emit("galleria:log", line(2));
    });
    await waitFor(() => expect(texts()).toHaveLength(2));
    expect(box.scrollTop).toBe(0);
    box.scrollTop = 700;
    act(() => {
      box.dispatchEvent(new Event("scroll"));
    });
    await act(async () => {
      await emit("galleria:log", line(3));
    });
    await waitFor(() => expect(texts()).toHaveLength(3));
    expect(box.scrollTop).toBe(1000);
  });

  it("has_no_accessibility_violations", async () => {
    serving([line(1), line(2, { level: "error", source: "openvpn", message: "TLS handshake failed" })]);
    const { container } = renderConnected(<LogTab profileId="homelab" />, session("connected"), "it");
    await waitFor(() => expect(texts()).toHaveLength(2));
    await expectNoAccessibilityViolations(container);
  });
});
