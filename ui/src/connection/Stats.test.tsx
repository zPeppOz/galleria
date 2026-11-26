import { emit } from "@tauri-apps/api/event";
import { act, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { backend, OPENVPN, renderConnected, WIREGUARD_WITH_HANDSHAKE } from "../test/connection";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeStatus, makeView } from "../test/fixtures";
import { formatBytes, formatDuration, formatHandshakeAge } from "./format";
import { Stats } from "./Stats";

const WG = { id: "homelab", protocol: "wireguard" as const };
const OVPN = { id: "office", protocol: "openvpn" as const };
const connected = (id: string, extra = {}) =>
  makeView({ backends: [WIREGUARD_WITH_HANDSHAKE, OPENVPN], status: makeStatus({ state: "connected", session: 2, profile: { id, name: "X" }, local_addresses: ["10.8.0.2"], ...extra }) });
const cell = (key: string) => document.querySelector(`[data-cell="${key}"] dd`)!;

describe("Stats", () => {
  it("bytes_and_handshake_follow_the_stats_events", async () => {
    backend();
    renderConnected(<Stats profile={WG} />, connected("homelab"));
    expect(cell("strip.ip").textContent).toBe("10.8.0.2");
    expect(cell("strip.state").textContent).toBe("Connected");
    await act(async () => {
      await emit("galleria:stats", { rx_bytes: 2048, tx_bytes: 512, last_handshake: String(Math.floor(Date.now() / 1000) - 12) });
    });
    await waitFor(() => expect(cell("strip.traffic").textContent).toBe("2.00 KiB received, 512 B sent"));
    expect(cell("strip.handshake").textContent).toMatch(/^1[0-4] sec\. ago$/);
    // the exact value is in the title, for who wants it
    expect(cell("strip.traffic").querySelector("[title]")?.getAttribute("title")).toBe("2,048 B / 512 B");
    await act(async () => {
      await emit("galleria:stats", { rx_bytes: 3 * 1024 * 1024, tx_bytes: 1024, last_handshake: String(Math.floor(Date.now() / 1000) - 2) });
    });
    await waitFor(() => expect(cell("strip.traffic").textContent).toBe("3.00 MiB received, 1.00 KiB sent"));
  });

  it("no_handshake_row_without_the_capability", async () => {
    backend();
    const view = makeView({ backends: [{ ...WIREGUARD_WITH_HANDSHAKE, capabilities: { ...WIREGUARD_WITH_HANDSHAKE.capabilities, handshake: false } }], status: connected("homelab").status });
    renderConnected(<Stats profile={WG} />, view);
    expect(document.querySelector('[data-cell="strip.handshake"]')).toBeNull();
    expect(document.querySelector('[data-cell="strip.duration"]')).not.toBeNull();
    // OpenVPN has no handshake to show, whatever its backend says
    const { unmount } = renderConnected(<Stats profile={OVPN} />, connected("office"));
    expect(document.querySelectorAll('[data-cell="strip.handshake"]')).toHaveLength(0);
    unmount();
  });

  it("a_profile_without_a_session_has_empty_cells", () => {
    backend();
    renderConnected(<Stats profile={WG} />, connected("somebody-else"));
    for (const key of ["strip.state", "strip.ip", "strip.traffic", "strip.handshake"]) expect(cell(key).textContent).toMatch(/not available$/);
  });

  it("the_duration_counts_from_the_first_time_the_window_saw_the_session_connected", async () => {
    backend();
    renderConnected(<Stats profile={OVPN} />, connected("office"));
    await waitFor(() => expect(cell("strip.duration").textContent).toMatch(/^0:0[0-2]$/));
  });

  it("the_numbers_are_not_a_live_region", async () => {
    backend();
    const { container } = renderConnected(<Stats profile={WG} />, connected("homelab"));
    await act(async () => {
      await emit("galleria:stats", { rx_bytes: 1, tx_bytes: 1, last_handshake: null });
    });
    // the notices of the window have their regions; the strip has none, inside it or around it
    const strip = container.querySelector(".detail-strip")!;
    expect(strip.querySelector("[aria-live], [role=status], [role=alert], [role=log], [role=timer]")).toBeNull();
    expect(strip.closest("[aria-live]")).toBeNull();
    expect(screen.getAllByRole("term").length).toBe(4);
  });

  it("has_no_accessibility_violations", async () => {
    backend();
    const { container } = renderConnected(<Stats profile={WG} />, connected("homelab"));
    await expectNoAccessibilityViolations(container);
  });
});

describe("the formats", () => {
  it("bytes_are_shown_in_units_people_read", () => {
    expect(formatBytes(0, "en")).toBe("0 B");
    expect(formatBytes(1023, "en")).toBe("1,023 B");
    expect(formatBytes(1536, "en")).toBe("1.50 KiB");
    expect(formatBytes(12 * 1024 * 1024, "en")).toBe("12.0 MiB");
    expect(formatBytes(1536, "it")).toBe("1,50 KiB");
  });

  it("a_duration_is_minutes_and_seconds_and_hours_from_the_hour", () => {
    expect(formatDuration(0)).toBe("0:00");
    expect(formatDuration(75)).toBe("1:15");
    expect(formatDuration(3600 + 125)).toBe("1:02:05");
    expect(formatDuration(-4)).toBe("0:00");
  });

  it("a_handshake_is_an_age_in_the_language_of_the_window", () => {
    const now = Date.UTC(2026, 10, 26, 10, 0, 0);
    expect(formatHandshakeAge(String(now / 1000 - 30), now, "en")).toBe("30 sec. ago");
    expect(formatHandshakeAge("2026-11-26T09:58:00Z", now, "en")).toBe("2 min. ago");
    expect(formatHandshakeAge("not a time", now, "en")).toBe("not a time");
  });
});
