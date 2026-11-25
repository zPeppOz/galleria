import { defineConfig } from "vitest/config";

// Level 3: real windows, real daemon, one test at a time. The default `pnpm test` never picks these files up
// (its include is src/**), and they need lab/tests/gui-smoke.sh to set the environment.
export default defineConfig({
  test: {
    environment: "node",
    include: ["e2e/**/*.e2e.ts"],
    fileParallelism: false,
    testTimeout: 60_000,
    hookTimeout: 30_000,
  },
});
