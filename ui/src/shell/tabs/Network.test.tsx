import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoAccessibilityViolations } from "../../test/axe";
import { makeDetail, SAMPLE_PROFILES } from "../../test/fixtures";
import { renderTab, withGoodKey } from "../../test/tabs";
import { Network } from "./Network";

const [OFFICE, , HOMELAB] = SAMPLE_PROFILES;

describe("Network", () => {
  it("wireguard_has_the_fields_of_d26", () => {
    renderTab(Network, withGoodKey(makeDetail(HOMELAB)));
    for (const name of ["Addresses", "DNS servers", "Search domains", "MTU", "Routed networks", "Keepalive (seconds)"]) {
      expect(screen.getByRole("textbox", { name }), name).toBeTruthy();
    }
    expect((screen.getByRole("textbox", { name: "Addresses" }) as HTMLInputElement).value).toBe("10.8.0.2/32");
    expect((screen.getByRole("textbox", { name: "Keepalive (seconds)" }) as HTMLInputElement).value).toBe("25");
    expect(screen.getByRole("switch", { name: "Block IPv6" })).toBeTruthy();
  });

  it("full_tunnel_is_an_indication", async () => {
    const user = userEvent.setup();
    renderTab(Network, withGoodKey(makeDetail(HOMELAB)));
    // read from the networks, not a control: there is no switch for it and the text follows what is typed
    expect(screen.getByText("All traffic goes through the tunnel.")).toBeTruthy();
    expect(screen.queryByRole("switch", { name: /tunnel|all traffic/i })).toBeNull();
    const routes = screen.getByRole("textbox", { name: "Routed networks" });
    await user.clear(routes);
    await user.type(routes, "10.0.0.0/8");
    expect(screen.getByText("Only the networks listed here go through the tunnel.")).toBeTruthy();
    await user.clear(routes);
    await user.type(routes, "10.0.0.0/8, ::/0");
    expect(screen.getByText("All traffic goes through the tunnel.")).toBeTruthy();
  });

  it("no_kill_switch_autostart_or_dns_leak_control", () => {
    const { container, unmount } = renderTab(Network, withGoodKey(makeDetail(HOMELAB)));
    for (const text of [/kill/i, /start/i, /boot/i, /leak/i, /autoconnect/i, /on login/i]) expect(container.textContent).not.toMatch(text);
    unmount();
    const ovpn = renderTab(Network, makeDetail(OFFICE));
    for (const text of [/kill/i, /start/i, /boot/i, /leak/i, /autoconnect/i, /on login/i]) expect(ovpn.container.textContent).not.toMatch(text);
  });

  it("openvpn_has_its_two_switches_and_says_the_rest_comes_with_the_connection", async () => {
    const user = userEvent.setup();
    const setSwitch = vi.fn();
    renderTab(Network, makeDetail(OFFICE), { setSwitch });
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(screen.getByRole("switch", { name: "Ignore the routes the server pushes" }).getAttribute("aria-checked")).toBe("false");
    expect(screen.getByRole("switch", { name: "Block IPv6" }).getAttribute("aria-checked")).toBe("true");
    expect(screen.getByText("DNS and routes in effect: available while connected.")).toBeTruthy();
    await user.click(screen.getByRole("switch", { name: "Ignore the routes the server pushes" }));
    expect(setSwitch).toHaveBeenCalledWith("ignore_pushed_routes", true);
    await user.click(screen.getByRole("switch", { name: "Block IPv6" }));
    expect(setSwitch).toHaveBeenCalledWith("ipv6_block", false);
  });

  it("each_peer_has_its_own_routes_and_keepalive", () => {
    const detail = withGoodKey(makeDetail(HOMELAB));
    const two = { ...detail, wireguard: { ...detail.wireguard!, peers: [detail.wireguard!.peers[0], { ...detail.wireguard!.peers[0], allowed_ips: ["10.9.0.0/16"] }] } };
    renderTab(Network, two);
    expect(screen.getByRole("textbox", { name: "Routed networks of peer 1" })).toBeTruthy();
    expect((screen.getByRole("textbox", { name: "Routed networks of peer 2" }) as HTMLInputElement).value).toBe("10.9.0.0/16");
    expect(screen.getByRole("textbox", { name: "Keepalive of peer 2 (seconds)" })).toBeTruthy();
  });

  it("an_error_is_linked_to_its_field", () => {
    renderTab(Network, withGoodKey(makeDetail(HOMELAB)), { errors: { mtu: "Use a number from 576 to 9000." } });
    const mtu = screen.getByRole("textbox", { name: "MTU" });
    expect(mtu.getAttribute("aria-invalid")).toBe("true");
    expect(document.getElementById((mtu.getAttribute("aria-describedby") ?? "").split(" ").pop() ?? "")?.textContent).toBe("Use a number from 576 to 9000.");
  });

  it("has_no_accessibility_violations", async () => {
    const { container, unmount } = renderTab(Network, withGoodKey(makeDetail(HOMELAB)), { errors: { mtu: "Use a number from 576 to 9000." } });
    await expectNoAccessibilityViolations(container);
    unmount();
    const ovpn = renderTab(Network, makeDetail(OFFICE));
    await expectNoAccessibilityViolations(ovpn.container);
  });
});
