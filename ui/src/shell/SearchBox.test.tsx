import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { ToastProvider } from "../design/Toast";
import { I18n } from "../i18n";
import { expectNoAccessibilityViolations } from "../test/axe";
import { SearchBox } from "./SearchBox";

function Harness({ count = 5, onArrowDown = () => undefined, announce }: { count?: number; onArrowDown?: () => void; announce?: (text: string, assertive: boolean) => void }) {
  const [value, setValue] = useState("");
  return (
    <I18n lang="en">
      <ToastProvider closeLabel="Dismiss the notice" announce={announce}>
        <button type="button">before</button>
        <SearchBox value={value} onChange={setValue} count={count} onArrowDown={onArrowDown} />
        <button type="button">after</button>
      </ToastProvider>
    </I18n>
  );
}

const field = () => screen.getByRole("searchbox", { name: "Search connections" }) as HTMLInputElement;

describe("SearchBox", () => {
  it("has_a_label_that_only_a_screen_reader_reads", () => {
    render(<Harness />);
    expect(field().getAttribute("aria-keyshortcuts")).toBe("Control+K");
    expect(document.querySelector("label.visually-hidden")?.textContent).toBe("Search connections");
  });

  it("ctrl_k_focuses_the_field", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    screen.getByRole("button", { name: "after" }).focus();
    await user.keyboard("{Control>}k{/Control}");
    expect(document.activeElement).toBe(field());
    // from the field itself it stays there, and a plain «k» is only a letter
    await user.keyboard("k");
    expect(field().value).toBe("k");
  });

  it("escape_clears_it", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.type(field(), "casa");
    expect(field().value).toBe("casa");
    await user.keyboard("{Escape}");
    expect(field().value).toBe("");
    expect(document.activeElement).toBe(field());
  });

  it("arrow_down_moves_to_the_list", async () => {
    const user = userEvent.setup();
    const onArrowDown = vi.fn();
    render(<Harness onArrowDown={onArrowDown} />);
    await user.click(field());
    await user.keyboard("{ArrowDown}");
    expect(onArrowDown).toHaveBeenCalledTimes(1);
  });

  it("result_count_is_a_status_and_is_not_announced", async () => {
    const user = userEvent.setup();
    const announce = vi.fn();
    const { rerender } = render(<Harness announce={announce} count={3} />);
    const region = screen.getAllByRole("status").find((el) => el.classList.contains("search-count")) as HTMLElement;
    expect(region.textContent).toBe("");
    await user.type(field(), "ca");
    expect(region.textContent).toBe("3 results");
    rerender(<Harness announce={announce} count={1} />);
    expect(region.textContent).toBe("1 result");
    // visible text and nothing else: Q27 leaves the live regions silent and does not announce the count (spec §12.1, rule 3)
    expect(announce).not.toHaveBeenCalled();
  });

  it("the_count_is_not_shown_for_an_empty_query", () => {
    render(<Harness />);
    expect(screen.queryByText(/results?$/)).toBeNull();
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = render(<main><Harness /></main>);
    await expectNoAccessibilityViolations(container);
  });
});
