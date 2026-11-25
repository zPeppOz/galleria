import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { icons } from "./icons";

describe("icons", () => {
  it("has_the_icons_the_plan_names", () => {
    expect(Object.keys(icons).sort()).toEqual(["check", "chevron", "close", "copy", "error", "info", "more", "star", "warning"]);
  });

  it("icons_are_hidden_from_assistive_technology_by_default", () => {
    for (const [name, node] of Object.entries(icons)) {
      const { container, unmount } = render(<>{node}</>);
      const svg = container.querySelector("svg");
      expect(svg?.getAttribute("aria-hidden"), name).toBe("true");
      expect(svg?.getAttribute("focusable"), name).toBe("false");
      expect(svg?.getAttribute("role"), name).toBeNull();
      expect(container.querySelector("title"), name).toBeNull();
      unmount();
    }
  });
});
