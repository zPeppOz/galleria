import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { expectNoAccessibilityViolations } from "../test/axe";
import { InlineAlert } from "./InlineAlert";

const LABELS = { info: "Information", ok: "Success", warn: "Warning", error: "Error" } as const;

describe("InlineAlert", () => {
  it("has_no_accessibility_violations", async () => {
    const { container } = render(
      <main>
        <InlineAlert tone="error" toneLabel="Error" title="Cannot connect" actions={<button type="button">Retry</button>}>Server unreachable</InlineAlert>
        <InlineAlert tone="warn" toneLabel="Warning">OpenVPN is missing</InlineAlert>
      </main>,
    );
    await expectNoAccessibilityViolations(container);
  });

  it.each(["info", "ok", "warn", "error"] as const)("error_is_an_alert_and_the_other_tones_are_status: %s", (tone) => {
    render(<InlineAlert tone={tone} toneLabel={LABELS[tone]}>Something happened</InlineAlert>);
    expect(screen.getByRole(tone === "error" ? "alert" : "status").textContent).toContain("Something happened");
    expect(screen.queryByRole(tone === "error" ? "status" : "alert")).toBeNull();
  });

  it.each(["info", "ok", "warn", "error"] as const)("tone_is_not_conveyed_by_colour_alone: %s", (tone) => {
    render(<InlineAlert tone={tone} toneLabel={LABELS[tone]}>Something happened</InlineAlert>);
    expect(screen.getByRole("img", { name: LABELS[tone] })).toBeTruthy();
  });

  it("shows_title_body_and_actions", () => {
    render(<InlineAlert tone="info" toneLabel="Information" title="Heads up" actions={<button type="button">Do it</button>}>Body text</InlineAlert>);
    expect(screen.getByText("Heads up")).toBeTruthy();
    expect(screen.getByText("Body text")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Do it" })).toBeTruthy();
  });
});
