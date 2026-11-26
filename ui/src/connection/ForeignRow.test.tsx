import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { backend, renderConnected, type Call } from "../test/connection";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeRubricaView, makeStatus, makeView } from "../test/fixtures";
import { renderShell } from "../test/shell";
import { ForeignRow } from "./ForeignRow";

/** A session of a profile that the rubrica does not have: the CLI of another rubrica started it. */
const foreignView = (state: "connected" | "connecting" = "connected") =>
  makeView({ status: makeStatus({ state, session: 7, profile: { id: "outside-id", name: "VPN Cliente" }, protocol: "openvpn", local_addresses: ["10.9.0.2"] }) });
const row = () => screen.getByRole("group", { name: "Session started outside this rubrica" });
const main = () => within(screen.getByRole("main"));
const calls = (made: Call[], cmd: string) => made.filter((call) => call.cmd === cmd);

describe("the session of someone else", () => {
  it("a_session_outside_the_rubrica_gets_the_fixed_row_and_the_bar_text", () => {
    backend();
    renderShell(foreignView());
    expect(within(row()).getByText("VPN Cliente")).toBeTruthy();
    expect(within(row()).getByText("started outside this rubrica")).toBeTruthy();
    expect(within(row()).getByText("Connected")).toBeTruthy();
    expect(within(row()).getByText("OpenVPN")).toBeTruthy();
    expect(screen.getByRole("contentinfo").textContent).toContain("Active connection: «VPN Cliente» — started outside this rubrica");
    // it is not a profile of the list: the five of the rubrica are there, and it is not one of them
    expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(5);
    expect(within(screen.getByRole("listbox")).queryByText("VPN Cliente")).toBeNull();
  });

  it("a_session_of_a_profile_of_the_rubrica_is_not_foreign_and_neither_is_one_before_the_rubrica_is_read", () => {
    backend();
    const own = makeView({ status: makeStatus({ state: "connected", session: 1, profile: { id: "office", name: "Ufficio Napoli" }, protocol: "openvpn" }) });
    const first = renderShell(own);
    expect(screen.queryByRole("group", { name: "Session started outside this rubrica" })).toBeNull();
    expect(screen.getByRole("contentinfo").textContent).not.toContain("outside this rubrica");
    first.unmount();
    // the rubrica that has not answered yet says nothing about whose a session is: the row waits
    renderShell(foreignView(), { rubrica: null });
    expect(screen.queryByRole("group", { name: "Session started outside this rubrica" })).toBeNull();
  });

  it("the_row_ignores_search_and_groups", async () => {
    backend();
    const user = userEvent.setup();
    renderShell(foreignView());
    await user.type(screen.getByRole("searchbox"), "no profile is called like this");
    expect(within(screen.getByRole("listbox")).queryAllByRole("option")).toHaveLength(0);
    expect(within(row()).getByText("VPN Cliente")).toBeTruthy();
    // it is above the list and outside every group
    expect(row().closest("[role=\"group\"][aria-label]:not([aria-label=\"Session started outside this rubrica\"])")).toBeNull();
    expect(screen.queryAllByRole("group").filter((group) => group.getAttribute("aria-label") !== "Session started outside this rubrica")).toHaveLength(0);
  });

  it("its_detail_has_the_state_and_the_log_only", async () => {
    backend();
    const user = userEvent.setup();
    renderShell(foreignView());
    await user.click(within(row()).getByRole("button", { name: /^VPN Cliente/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("VPN Cliente");
    // the state of the session (the panel above) and the numbers
    expect(main().getByText("Connected", { selector: ".status-view-state" })).toBeTruthy();
    expect(document.querySelector('[data-cell="strip.ip"] dd')?.textContent).toBe("10.9.0.2");
    // a single tab, the log; nothing of a profile
    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual(["Log"]);
    expect(await screen.findByRole("log")).toBeTruthy();
    for (const name of ["More actions", "Duplicate", "Export", "Delete", "Save", "Connect"]) expect(main().queryByRole("button", { name })).toBeNull();
    expect(main().queryByRole("textbox", { name: "Name" })).toBeNull();
    // the one thing to do is to disconnect
    expect(main().getByRole("button", { name: "Disconnect" })).toBeTruthy();
    // and the row of the list is the one that is chosen
    expect(within(row()).getByRole("button", { name: /^VPN Cliente/ }).getAttribute("aria-current")).toBe("true");
  });

  it("disconnect_asks_first_and_cancel_does_nothing", async () => {
    const made = backend();
    const user = userEvent.setup();
    renderShell(foreignView());
    await user.click(within(row()).getByRole("button", { name: "Disconnect «VPN Cliente»" }));
    const confirm = await screen.findByRole("alertdialog", { name: "Disconnect «VPN Cliente»?" });
    expect(confirm.textContent).toContain("This connection was not started from this rubrica. Another user of this computer may be using it. Disconnect it?");
    expect(calls(made, "disconnect")).toHaveLength(0);
    await user.click(within(confirm).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    expect(calls(made, "disconnect")).toHaveLength(0);
    // the same from the detail; only the answer of the confirmation ends the session
    await user.click(within(row()).getByRole("button", { name: /^VPN Cliente/ }));
    await user.click(main().getByRole("button", { name: "Disconnect" }));
    const again = await screen.findByRole("alertdialog");
    expect(calls(made, "disconnect")).toHaveLength(0);
    await user.click(within(again).getByRole("button", { name: "Disconnect" }));
    await waitFor(() => expect(calls(made, "disconnect")).toHaveLength(1));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  });

  it("the_confirmation_focuses_cancel", async () => {
    backend();
    const user = userEvent.setup();
    renderShell(foreignView());
    await user.click(within(row()).getByRole("button", { name: "Disconnect «VPN Cliente»" }));
    const confirm = await screen.findByRole("alertdialog");
    await waitFor(() => expect((document.activeElement as HTMLElement).textContent).toBe("Cancel"));
    expect(confirm.contains(document.activeElement)).toBe(true);
    // nothing is announced for it: it takes the focus (Q27, rule 2)
    expect(document.querySelector(".gx-toasts")?.textContent).toBe("");
  });

  it("connecting_over_it_asks_first_then_replaces", async () => {
    const made = backend((call) => {
      if (call.cmd === "connect_profile" && call.args.replace === false) throw { code: "already_active", message: "A session is active", data: { foreign: true, profile: { id: "outside-id", name: "VPN Cliente" } } };
      return undefined;
    });
    const user = userEvent.setup();
    renderShell(foreignView());
    await user.click(screen.getByRole("button", { name: "Connect" }));
    const confirm = await screen.findByRole("alertdialog", { name: "Replace «VPN Cliente»?" });
    expect(confirm.textContent).toContain("Another user of this computer may be using it");
    await waitFor(() => expect((document.activeElement as HTMLElement).textContent).toBe("Cancel"));
    // one call without `replace`, and no second one until the answer
    expect(calls(made, "connect_profile").map((call) => call.args)).toEqual([{ id: "office", replace: false }]);
    await user.click(within(confirm).getByRole("button", { name: "Replace" }));
    await waitFor(() => expect(calls(made, "connect_profile").map((call) => call.args)).toEqual([{ id: "office", replace: false }, { id: "office", replace: true }]));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  });

  it("cancelling_the_replacement_connects_nothing", async () => {
    const made = backend((call) => {
      if (call.cmd === "connect_profile") throw { code: "already_active", message: "A session is active", data: { foreign: true, profile: { id: "outside-id", name: "VPN Cliente" } } };
      return undefined;
    });
    const user = userEvent.setup();
    renderShell(foreignView());
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await user.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    expect(calls(made, "connect_profile")).toHaveLength(1);
    // no error is shown for the question: it was not a failure
    expect(document.querySelector('[data-notice="command"]')).toBeNull();
  });

  it("when_the_profile_appears_in_the_rubrica_the_row_goes", () => {
    backend();
    const { show } = renderConnected(<ForeignRow selected={false} onSelect={() => undefined} />, foreignView(), "en", ["office"]);
    expect(screen.getByText("VPN Cliente")).toBeTruthy();
    show(foreignView(), ["office", "outside-id"]);
    expect(screen.queryByText("VPN Cliente")).toBeNull();
    // and when it is deleted from the rubrica again, the session is foreign again
    show(foreignView(), ["office"]);
    expect(screen.getByText("VPN Cliente")).toBeTruthy();
  });

  it("the_row_goes_with_the_session_and_the_detail_with_it", async () => {
    backend();
    const user = userEvent.setup();
    const shown = renderShell(foreignView());
    await user.click(within(row()).getByRole("button", { name: /^VPN Cliente/ }));
    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual(["Log"]);
    shown.unmount();
    renderShell(makeView());
    expect(screen.queryByRole("group", { name: "Session started outside this rubrica" })).toBeNull();
    // the chosen profile is a profile of the rubrica again
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Ufficio Napoli");
  });

  it("an_empty_rubrica_still_shows_the_row", () => {
    backend();
    renderShell(foreignView(), { rubrica: makeRubricaView({ profiles: [], groups: [] }) });
    expect(within(row()).getByText("VPN Cliente")).toBeTruthy();
  });

  it("has_no_accessibility_violations", async () => {
    backend();
    const user = userEvent.setup();
    const { container } = renderShell(foreignView(), { lang: "it" });
    await user.click(within(screen.getByRole("group", { name: "Sessione avviata fuori da questa rubrica" })).getByRole("button", { name: /^VPN Cliente/ }));
    await screen.findByRole("log");
    await expectNoAccessibilityViolations(container);
  });
});
