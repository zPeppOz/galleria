import { defineConfig } from "vitest/config";

// Level 3: real windows, real daemon, one test at a time. The default `pnpm test` never picks these files up
// (its include is src/**), and they need lab/tests/gui-smoke.sh to set the environment.
//
// `lab-import.e2e.ts` needs the laboratory (the profiles of lab/out) and writes outside the run, so the smoke test does
// not run it: it runs alone, and only when `lab/tests/gui-lab-import.sh` asks for it with GALLERIA_LAB_IMPORT.
const labImport = process.env.GALLERIA_LAB_IMPORT === "1";

export default defineConfig({
  test: {
    environment: "node",
    include: labImport ? ["e2e/lab-import.e2e.ts"] : ["e2e/**/*.e2e.ts"],
    exclude: labImport ? [] : ["e2e/lab-import.e2e.ts"],
    fileParallelism: false,
    testTimeout: 60_000,
    hookTimeout: 30_000,
  },
});
