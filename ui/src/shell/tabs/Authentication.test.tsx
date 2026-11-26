import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { backend, renderConnected } from "../../test/connection";
import { expectNoAccessibilityViolations } from "../../test/axe";
import { toDraft } from "../../rubrica/draft";
import { makeDetail, SAMPLE_PROFILES } from "../../test/fixtures";
import { renderTab, withGoodKey } from "../../test/tabs";
import type { ProfileDetail } from "../../rubrica/types";
import { Authentication } from "./Authentication";

const [OFFICE, , HOMELAB, , ROSSI] = SAMPLE_PROFILES;
const withSecrets = (detail: ProfileDetail, secret_refs: string[]): ProfileDetail => ({ ...detail, secret_refs });

describe("Authentication", () => {
  it("method_is_a_label_not_a_selector", () => {
    renderTab(Authentication, makeDetail(OFFICE));
    expect(screen.getByRole("heading", { name: "Method" })).toBeTruthy();
    expect(screen.getByText("Certificate")).toBeTruthy();
    expect(screen.queryByRole("combobox")).toBeNull();
    expect(screen.queryByRole("radiogroup")).toBeNull();
    expect(screen.queryByRole("radio")).toBeNull();
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("the_method_follows_what_the_profile_declares", () => {
    const rossi = { ...makeDetail(ROSSI), openvpn: { ...makeDetail(ROSSI).openvpn!, auth: { certificate: false, user_pass: true, otp: true } } };
    const { unmount } = renderTab(Authentication, rossi);
    expect(screen.getByText("User name, password and a code")).toBeTruthy();
    unmount();
    const both = { ...makeDetail(OFFICE), openvpn: { ...makeDetail(OFFICE).openvpn!, auth: { certificate: true, user_pass: true, otp: false } } };
    renderTab(Authentication, both);
    expect(screen.getByText("Certificate, User name and password")).toBeTruthy();
  });

  it("the_names_of_the_secrets_are_listed_and_never_a_value", () => {
    renderTab(Authentication, withSecrets(makeDetail(OFFICE), ["openvpn.block.key", "openvpn.block.tls-crypt"]));
    expect(screen.getAllByRole("listitem").map((li) => li.textContent)).toEqual(["openvpn.block.key", "openvpn.block.tls-crypt"]);
  });

  it("a_profile_without_secrets_says_none", () => {
    renderTab(Authentication, makeDetail(OFFICE));
    expect(screen.getByText("None")).toBeTruthy();
  });

  it("private_key_is_never_rendered", () => {
    const { container } = renderTab(Authentication, withSecrets(withGoodKey(makeDetail(HOMELAB)), ["wireguard.private_key", "wireguard.peer.0.preshared_key"]));
    expect(screen.getByText("Private key")).toBeTruthy();
    expect(screen.getByText("In the secrets store. It is never shown.")).toBeTruthy();
    // no field for it, not even a masked one, and not the names of the secrets
    expect(container.querySelector("input[type=password]")).toBeNull();
    expect(screen.queryByRole("textbox", { name: /private/i })).toBeNull();
    expect(container.textContent).not.toContain("wireguard.private_key");
  });

  it("the_public_key_of_a_peer_is_a_field", () => {
    renderTab(Authentication, withGoodKey(makeDetail(HOMELAB)));
    const key = screen.getByRole("textbox", { name: "Public key" }) as HTMLInputElement;
    expect(key.value).toBe("c2VlZC1wdWJsaWMta2V5LWZvci10aGUtdGVzdHMtMDA=");
    expect(key.readOnly).toBe(false);
  });

  it("has_no_accessibility_violations", async () => {
    const { container, unmount } = renderTab(Authentication, withSecrets(makeDetail(OFFICE), ["openvpn.block.key"]));
    await expectNoAccessibilityViolations(container);
    unmount();
    const wireguard = renderTab(Authentication, withGoodKey(makeDetail(HOMELAB)), { errors: { "peers.0.public_key": "A WireGuard key is 44 characters of base64 that end with =." } });
    await expectNoAccessibilityViolations(wireguard.container);
  });

  it("saved_credentials_are_named_and_can_be_forgotten", async () => {
    const user = userEvent.setup();
    const calls = backend();
    const detail = withSecrets(makeDetail(ROSSI), ["openvpn.block.key", "openvpn.username", "openvpn.password"]);
    renderConnected(<Authentication detail={detail} draft={toDraft(detail)} edit={() => undefined} errors={{}} groups={[]} settings={detail.settings} setSwitch={() => undefined} />);
    expect(screen.getByRole("heading", { name: "Credentials saved in the keyring" })).toBeTruthy();
    // the names are listed, the values never
    expect(screen.getAllByRole("listitem").map((li) => li.textContent)).toContain("openvpn.password");
    await user.click(screen.getByRole("button", { name: "Forget" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["forget_credentials"]));
    expect(calls[0].args).toEqual({ id: detail.id });
  });

  it("without_saved_credentials_there_is_nothing_to_forget", () => {
    backend();
    const detail = withSecrets(makeDetail(ROSSI), ["openvpn.block.key"]);
    renderConnected(<Authentication detail={detail} draft={toDraft(detail)} edit={() => undefined} errors={{}} groups={[]} settings={detail.settings} setSwitch={() => undefined} />);
    expect(screen.queryByRole("button", { name: "Forget" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "Credentials saved in the keyring" })).toBeNull();
  });
});
