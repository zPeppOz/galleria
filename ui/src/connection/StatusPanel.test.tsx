import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { SESSION_ERROR_CODES, VPN_STATES } from "../daemon/types";
import { I18n, type Lang } from "../i18n";
import en from "../i18n/en.json";
import it_ from "../i18n/it.json";
import { backend, renderConnected } from "../test/connection";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeStatus } from "../test/fixtures";
import { StatusPanel } from "./StatusPanel";

const catalogues = { en, it: it_ } as Record<Lang, Record<string, string>>;
const show = (over = {}, lang: Lang = "it") =>
  render(
    <I18n lang={lang}>
      <StatusPanel status={makeStatus({ state: "connecting", ...over })} />
    </I18n>,
  );

describe("StatusPanel", () => {
  it.each(VPN_STATES.flatMap((s) => (["it", "en"] as Lang[]).map((l) => [s, l] as const)))("each_state_has_its_text_in_both_languages: %s in %s", (state, lang) => {
    show({ state }, lang);
    expect(screen.getByText(catalogues[lang][`state.${state}`])).toBeTruthy();
  });

  it("a_known_detail_is_translated_and_an_unknown_one_is_shown_raw", () => {
    const { unmount } = show({ detail: "TCP_CONNECT" }, "en");
    expect(screen.getByText("Reaching the server")).toBeTruthy();
    unmount();
    show({ detail: "SOMETHING_NEW" }, "en");
    expect(screen.getByText("SOMETHING_NEW")).toBeTruthy();
  });

  it("an_unknown_detail_is_shown_raw_in_italian_too", () => {
    show({ detail: "SOMETHING_NEW" });
    expect(screen.getByText("SOMETHING_NEW")).toBeTruthy();
  });

  it("the_attempt_is_numbered_while_reconnecting", () => {
    show({ state: "reconnecting", attempt: 3 }, "en");
    expect(screen.getByText(/Reconnecting · attempt 3/)).toBeTruthy();
  });

  it.each(SESSION_ERROR_CODES)("the_error_has_its_text_in_both_languages: %s", (code) => {
    const { unmount } = show({ state: "error", error: { code, message: "English message" } }, "it");
    expect(screen.getByRole("alert").textContent).toContain(catalogues.it[`error.${code}`]);
    unmount();
    show({ state: "error", error: { code, message: "English message" } }, "en");
    expect(screen.getByRole("alert").textContent).toContain(catalogues.en[`error.${code}`]);
  });

  it("an_unknown_error_code_shows_the_daemon_message", () => {
    show({ state: "error", error: { code: "brand_new_code", message: "Daemon said so" } });
    expect(screen.getByRole("alert").textContent).toContain("Daemon said so");
  });

  it("the_facts_of_the_session_are_read_only_and_there_is_no_button", () => {
    show({
      state: "connected",
      profile: { id: "p", name: "Homelab" },
      protocol: "wireguard",
      local_addresses: ["10.8.0.2"],
      remote: "203.0.113.24:51820",
      net: { interface: "glr-wg0", dns_servers: ["10.8.0.1"], search_domains: [], routes: ["0.0.0.0/0"], full_tunnel: true, ipv6_block: "unreachable" },
    });
    for (const text of ["Homelab", "WireGuard", "10.8.0.2", "203.0.113.24:51820", "glr-wg0", "10.8.0.1", "0.0.0.0/0"]) expect(screen.getByText(text)).toBeTruthy();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  it("a_refused_saved_password_offers_to_forget_it", async () => {
    const user = userEvent.setup();
    const calls = backend();
    const refused = makeStatus({ state: "error", profile: { id: "dc", name: "Datacenter Milano" }, error: { code: "auth_failed", message: "Authentication failed" } });
    const { unmount } = renderConnected(<StatusPanel status={refused} savedCredentials />);
    expect(screen.getByText("The saved credentials were not accepted.")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Forget the saved credentials" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["forget_credentials"]));
    expect(calls[0].args).toEqual({ id: "dc" });
    unmount();
    // credentials that were typed, not saved: nothing to forget; and another error is not a refusal of the password
    renderConnected(<StatusPanel status={refused} />);
    expect(screen.queryByRole("button", { name: "Forget the saved credentials" })).toBeNull();
    const other = makeStatus({ state: "error", profile: { id: "dc", name: "X" }, error: { code: "server_unreachable", message: "x" } });
    renderConnected(<StatusPanel status={other} savedCredentials />);
    expect(screen.queryByRole("button", { name: "Forget the saved credentials" })).toBeNull();
  });

  it("nothing_in_it_is_a_live_region", () => {
    const { container } = show({ state: "connected" });
    expect(container.querySelector("[aria-live]")).toBeNull();
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = render(
      <I18n lang="it">
        <main>
          <StatusPanel status={makeStatus({ state: "error", detail: "AUTH", profile: { id: "p", name: "Homelab" }, protocol: "wireguard", error: { code: "auth_failed", message: "x" } })} />
        </main>
      </I18n>,
    );
    await expectNoAccessibilityViolations(container);
  });
});
