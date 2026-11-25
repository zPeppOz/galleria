import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { expectNoAccessibilityViolations } from "../test/axe";
import { Field, type FieldProps } from "./Field";

function Harness(props: Partial<FieldProps>) {
  const [value, setValue] = useState(props.value ?? "");
  return <Field label="Name" {...props} value={value} onChange={setValue} />;
}

describe("Field", () => {
  it("has_no_accessibility_violations", async () => {
    const { container } = render(
      <main>
        <Harness label="Name" hint="As it shows in the list" />
        <Harness label="Server" error="Not a valid address" />
        <Harness label="Notes" as="textarea" mono />
        <Harness label="Protocol" as="select" value="wg" options={[{ value: "wg", label: "WireGuard" }, { value: "ovpn", label: "OpenVPN" }]} />
        <Harness label="Password" type="password" />
        <Harness label="Port" inputMode="numeric" value="51820" readOnly />
      </main>,
    );
    await expectNoAccessibilityViolations(container);
  });

  it("is_reachable_and_operable_from_the_keyboard", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    expect(document.activeElement).toBe(screen.getByLabelText("Name"));
    await user.keyboard("home");
    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe("home");
  });

  it("label_hint_and_error_are_linked_to_the_control", () => {
    render(<Harness hint="Shown in the list" error="Required" />);
    const input = screen.getByLabelText("Name");
    const described = (input.getAttribute("aria-describedby") ?? "").split(" ").map((id) => document.getElementById(id)?.textContent);
    expect(described).toEqual(["Shown in the list", "Required"]);
    expect(input.getAttribute("aria-invalid")).toBe("true");
  });

  it("a_field_without_error_is_not_marked_invalid", () => {
    render(<Harness />);
    expect(screen.getByLabelText("Name").getAttribute("aria-invalid")).toBeNull();
    expect(screen.getByLabelText("Name").getAttribute("aria-describedby")).toBeNull();
  });

  it("read_only_field_is_focusable_but_not_editable", async () => {
    const user = userEvent.setup();
    render(<Harness value="fixed" readOnly />);
    await user.tab();
    const input = screen.getByLabelText("Name") as HTMLInputElement;
    expect(document.activeElement).toBe(input);
    await user.keyboard("more");
    expect(input.value).toBe("fixed");
  });

  it("select_lists_its_options", async () => {
    const user = userEvent.setup();
    render(<Harness as="select" label="Protocol" value="wg" options={[{ value: "wg", label: "WireGuard" }, { value: "ovpn", label: "OpenVPN" }]} />);
    const select = screen.getByRole("combobox", { name: "Protocol" }) as HTMLSelectElement;
    expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual(["WireGuard", "OpenVPN"]);
    await user.selectOptions(select, "ovpn");
    expect(select.value).toBe("ovpn");
  });

  it("a_read_only_select_shows_only_the_chosen_option", () => {
    render(<Harness as="select" label="Protocol" value="wg" readOnly options={[{ value: "wg", label: "WireGuard" }, { value: "ovpn", label: "OpenVPN" }]} />);
    expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual(["WireGuard"]);
  });

  it("password_field_never_renders_its_value_as_text", async () => {
    const user = userEvent.setup();
    const { container } = render(<Harness label="Password" type="password" />);
    await user.type(screen.getByLabelText("Password"), "s3cret!");
    const input = screen.getByLabelText("Password") as HTMLInputElement;
    expect(input.type).toBe("password");
    expect(input.getAttribute("autocomplete")).toBe("off");
    expect(container.textContent).not.toContain("s3cret!");
    expect(container.innerHTML).not.toContain('type="text"');
  });

  it("the_name_is_on_the_control_so_a_form_can_find_it", () => {
    render(<Harness name="peers.0.endpoint" />);
    expect(screen.getByLabelText("Name").getAttribute("name")).toBe("peers.0.endpoint");
  });
});
