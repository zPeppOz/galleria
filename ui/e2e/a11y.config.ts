import { defineConfig } from "vitest/config";

// The screen reader checks: real window, Orca and the AT-SPI tree, one test at a time. Run by lab/tests/gui-a11y.sh
// (scripts/gui.sh a11y), which starts Orca and sets the environment; `pnpm test` and `pnpm test:e2e` never pick these up.
export default defineConfig({
  test: {
    environment: "node",
    include: ["e2e/**/*.a11y.ts"],
    fileParallelism: false,
    testTimeout: 90_000,
    hookTimeout: 60_000,
  },
});
