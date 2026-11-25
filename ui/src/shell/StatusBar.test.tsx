import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { AppInfo, DaemonView, LinkState } from "../daemon/types";
import { I18n } from "../i18n";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeInfo, makeStatus, makeView } from "../test/fixtures";
import { StatusBar } from "./StatusBar";

const LINKS: [LinkState, RegExp][] = [
  ["connecting", /Connecting to the service/],
  ["connected", /Connected to the service \(version 0\.1\.0\)/],
  ["daemon_not_running", /The service is not running/],
  ["permission_denied", /No access to the service/],
  ["incompatible_protocol", /another protocol version/],
];

const show = (view: DaemonView | null = makeView(), info: AppInfo | null = makeInfo({ version: "0.3.1" }), lang: "en" | "it" = "en") =>
  render(
    <I18n lang={lang}>
      <StatusBar view={view} info={info} />
    </I18n>,
  );

describe("StatusBar", () => {
  it.each(LINKS)("shows_the_real_state_and_link: %s", (link, text) => {
    show(makeView({ link, status: link === "connected" ? makeStatus({ state: "reconnecting" }) : null }));
    const bar = screen.getByRole("contentinfo");
    expect(bar.textContent).toMatch(text);
    expect(bar.textContent).toContain(link === "connected" ? "Reconnecting" : "No connection state");
    expect(bar.textContent).toContain("App 0.3.1");
  });

  it("the_left_side_is_the_state_of_the_daemon_session_in_the_language_of_the_catalogue", () => {
    show(makeView({ status: makeStatus({ state: "error" }) }), makeInfo(), "it");
    expect(screen.getByRole("contentinfo").textContent).toContain("Errore");
  });

  it("is_a_polite_live_region_and_waits_for_the_app_version", () => {
    show(makeView(), null);
    const bar = screen.getByRole("contentinfo");
    expect(bar.getAttribute("aria-live")).toBe("polite");
    expect(bar.textContent).not.toContain("App ");
  });

  it("works_before_the_first_answer_of_the_service", () => {
    show(null, null);
    expect(screen.getByRole("contentinfo").textContent).toMatch(/Connecting to the service/);
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = show();
    await expectNoAccessibilityViolations(container);
  });
});
