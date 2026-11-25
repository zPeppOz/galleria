import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoAccessibilityViolations } from "../../test/axe";
import { makeDetail, SAMPLE_PROFILES } from "../../test/fixtures";
import { renderTab, withGoodKey } from "../../test/tabs";
import { General } from "./General";

const [OFFICE, DATACENTER, HOMELAB] = SAMPLE_PROFILES;

describe("General", () => {
  it("openvpn_server_and_transport_are_read_only", () => {
    renderTab(General, makeDetail(OFFICE));
    const server = screen.getByRole("textbox", { name: "Server" }) as HTMLInputElement;
    expect(server.value).toBe("vpn.ufficio.example:1194");
    expect(server.readOnly).toBe(true);
    const transport = screen.getByRole("radiogroup", { name: "Transport" });
    expect(transport.getAttribute("aria-readonly")).toBe("true");
    expect((within(transport).getByRole("radio", { name: "UDP" }) as HTMLInputElement).checked).toBe(true);
    // the name and the notes are the ones to edit
    expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).readOnly).toBe(false);
    expect((screen.getByRole("textbox", { name: "Notes" }) as HTMLTextAreaElement).readOnly).toBe(false);
  });

  it("the_transport_comes_from_the_profile", () => {
    renderTab(General, makeDetail(DATACENTER));
    const transport = screen.getByRole("radiogroup", { name: "Transport" });
    expect((within(transport).getByRole("radio", { name: "TCP" }) as HTMLInputElement).checked).toBe(true);
  });

  it("wireguard_has_the_endpoint_to_edit_and_no_transport", async () => {
    const user = userEvent.setup();
    renderTab(General, withGoodKey(makeDetail(HOMELAB)));
    expect(screen.queryByRole("radiogroup", { name: "Transport" })).toBeNull();
    expect(screen.queryByRole("textbox", { name: "Server" })).toBeNull();
    const endpoint = screen.getByRole("textbox", { name: "Endpoint" }) as HTMLInputElement;
    expect(endpoint.value).toBe("casa.example:51820");
    await user.clear(endpoint);
    await user.type(endpoint, "nuovo.example:51820");
    expect((screen.getByRole("textbox", { name: "Endpoint" }) as HTMLInputElement).value).toBe("nuovo.example:51820");
  });

  it("the_name_the_group_and_the_notes_are_changed_in_place", async () => {
    const user = userEvent.setup();
    renderTab(General, makeDetail(OFFICE));
    const name = screen.getByRole("textbox", { name: "Name" });
    await user.clear(name);
    await user.type(name, "Sede");
    expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("Sede");
    await user.selectOptions(screen.getByRole("combobox", { name: "Group" }), "Clienti");
    expect((screen.getByRole("combobox", { name: "Group" }) as HTMLSelectElement).value).toBe("Clienti");
    await user.type(screen.getByRole("textbox", { name: "Notes" }), "dietro il router");
    expect((screen.getByRole("textbox", { name: "Notes" }) as HTMLTextAreaElement).value).toBe("dietro il router");
  });

  it("the_reconnect_switch_writes_at_once_and_shows_the_saved_value", async () => {
    const user = userEvent.setup();
    const setSwitch = vi.fn();
    renderTab(General, makeDetail(OFFICE), { setSwitch });
    const toggle = screen.getByRole("switch", { name: "Reconnect automatically" });
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    await user.click(toggle);
    expect(setSwitch).toHaveBeenCalledWith("auto_reconnect", false);
  });

  it("a_field_with_an_error_is_linked_to_its_text", () => {
    renderTab(General, makeDetail(OFFICE), { errors: { name: "This field is required." } });
    const name = screen.getByRole("textbox", { name: "Name" });
    expect(name.getAttribute("aria-invalid")).toBe("true");
    expect(document.getElementById(name.getAttribute("aria-describedby") ?? "")?.textContent).toBe("This field is required.");
  });

  it("a_profile_with_several_peers_names_each_endpoint", () => {
    const detail = withGoodKey(makeDetail(HOMELAB));
    const two = { ...detail, wireguard: { ...detail.wireguard!, peers: [detail.wireguard!.peers[0], { ...detail.wireguard!.peers[0], endpoint: "due.example:51820" }] } };
    renderTab(General, two);
    expect(screen.getByRole("textbox", { name: "Endpoint of peer 1" })).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "Endpoint of peer 2" })).toBeTruthy();
  });

  it("has_no_accessibility_violations", async () => {
    const { container, unmount } = renderTab(General, makeDetail(OFFICE), { errors: { name: "This field is required." } });
    await expectNoAccessibilityViolations(container);
    unmount();
    const wireguard = renderTab(General, withGoodKey(makeDetail(HOMELAB)));
    await expectNoAccessibilityViolations(wireguard.container);
  });
});
