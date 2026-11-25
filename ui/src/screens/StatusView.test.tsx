import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { VPN_STATES } from "../daemon/types";
import { I18n } from "../i18n";
import { messages, type Lang } from "../messages";
import { makeStatus, makeView } from "../test/fixtures";
import { StatusView } from "./StatusView";

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
      expect(screen.getByText(messages[lang][`state.${state}`])).toBeTruthy();
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
    expect(screen.getByRole("alert").textContent).toContain("Authentication failed");
  });

  it("there_is_no_connect_control", () => {
    show();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });
});
