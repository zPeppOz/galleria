import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { VPN_STATES } from "../daemon/types";
import { I18n, type Lang } from "../i18n";
import en from "../i18n/en.json";
import it_ from "../i18n/it.json";
import { makeStatus, makeView } from "../test/fixtures";
import { expectNoAccessibilityViolations } from "../test/axe";
import { StatusView } from "./StatusView";

const catalogues = { en, it: it_ } as Record<Lang, Record<string, string>>;
const show = (view = makeView(), lang: Lang = "it") =>
  render(
    <I18n lang={lang}>
      <StatusView view={view} />
    </I18n>,
  );
const openvpn = (reason: string) => ({
  protocol: "openvpn" as const, available: false, version: null, unavailable_reason: reason,
  capabilities: { handshake: false, interactive_auth: false, server_pushed_config: false },
});
const wireguard = { ...openvpn(""), protocol: "wireguard" as const, available: true, unavailable_reason: null };

describe("StatusView", () => {
  it.each(VPN_STATES.flatMap((s) => (["it", "en"] as Lang[]).map((l) => [s, l] as const)))(
    "each_of_the_seven_states_has_a_label: %s in %s",
    (state, lang) => {
      show(makeView({ status: makeStatus({ state }) }), lang);
      expect(screen.getByText(catalogues[lang][`state.${state}`])).toBeTruthy();
    },
  );

  it.each([
    ["openvpn_not_found", /OpenVPN non è installato/],
    ["openvpn_too_old", /troppo vecchio/],
  ])("unavailable_openvpn_shows_a_warning_with_the_reason: %s", (reason, text) => {
    show(makeView({ backends: [openvpn(reason), wireguard] }));
    expect(screen.getByText(text)).toBeTruthy();
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
  });

  it("session_error_shows_code_and_daemon_message", () => {
    show(makeView({ status: makeStatus({ state: "error", error: { code: "auth_failed", message: "Authentication failed" } }) }));
    expect(screen.getByRole("alert").textContent).toContain("auth_failed");
    expect(screen.getByRole("alert").textContent).toContain("Autenticazione fallita"); // the catalogue text, not the daemon message
  });

  it("unknown_session_error_code_shows_the_daemon_message", () => {
    show(makeView({ status: makeStatus({ state: "error", error: { code: "brand_new_code", message: "Daemon said so" } }) }));
    expect(screen.getByRole("alert").textContent).toContain("Daemon said so");
  });

  it("there_is_no_connect_control", () => {
    show();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  it("error_and_warnings_are_inline_alerts_with_a_named_icon", () => {
    show(makeView({ backends: [openvpn("openvpn_not_found"), wireguard], status: makeStatus({ state: "error", error: { code: "auth_failed", message: "x" } }) }));
    expect(screen.getByRole("alert").textContent).toContain("auth_failed");
    expect(screen.getByRole("img", { name: "Errore" })).toBeTruthy();
    expect(screen.getByRole("img", { name: "Attenzione" })).toBeTruthy();
    expect(screen.getByRole("status").textContent).toContain("OpenVPN non è installato");
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = render(
      <I18n lang="it">
        <main>
          <StatusView view={makeView({ backends: [openvpn("openvpn_not_found"), wireguard], status: makeStatus({ state: "error", error: { code: "auth_failed", message: "x" }, profile: { id: "p", name: "Homelab" }, protocol: "wireguard" }) })} />
        </main>
      </I18n>,
    );
    await expectNoAccessibilityViolations(container);
  });
});
