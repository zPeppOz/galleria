import axe from "axe-core";
import { expect } from "vitest";

/**
 * Runs axe on the rendered screen. The colour-contrast rule is off: jsdom has no layout or computed colours, so
 * it cannot answer. Contrast is checked by hand with the design tokens (B2).
 */
export async function expectNoAccessibilityViolations(container: HTMLElement, options: { landmarks?: boolean } = {}): Promise<void> {
  // `landmarks: false` also switches off «all content in a landmark», for a lone component in a portal (a dialog,
  // a menu); the shell tests keep it on.
  const results = await axe.run(container, {
    rules: { "color-contrast": { enabled: false }, ...(options.landmarks === false ? { region: { enabled: false } } : {}) },
  });
  const summary = results.violations.map((v) => `${v.id}: ${v.help} (${v.nodes.length})`);
  expect(summary).toEqual([]);
}
