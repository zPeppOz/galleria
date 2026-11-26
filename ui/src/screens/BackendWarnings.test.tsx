import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { I18n } from "../i18n";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeView } from "../test/fixtures";
import { BackendWarnings } from "./BackendWarnings";

const show = (view = makeView()) =>
  render(
    <I18n lang="it">
      <BackendWarnings view={view} />
    </I18n>,
  );
const openvpn = (reason: string) => ({
  protocol: "openvpn" as const, available: false, version: null, unavailable_reason: reason,
  capabilities: { handshake: false, interactive_auth: false, server_pushed_config: false },
});
const wireguard = { ...openvpn(""), protocol: "wireguard" as const, available: true, unavailable_reason: null };

describe("BackendWarnings", () => {
  it.each([
    ["openvpn_not_found", /OpenVPN non è installato/],
    ["openvpn_too_old", /troppo vecchio/],
  ])("unavailable_openvpn_shows_a_warning_with_the_reason: %s", (reason, text) => {
    show(makeView({ backends: [openvpn(reason), wireguard] }));
    expect(screen.getByText(text)).toBeTruthy();
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
  });

  it("says_nothing_when_every_backend_is_there", () => {
    const { container } = show(makeView({ backends: [wireguard] }));
    expect(container.textContent).toBe("");
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = render(
      <I18n lang="it">
        <main>
          <BackendWarnings view={makeView({ backends: [openvpn("openvpn_not_found"), wireguard] })} />
        </main>
      </I18n>,
    );
    await expectNoAccessibilityViolations(container);
  });
});
