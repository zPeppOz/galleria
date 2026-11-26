import { defineConfig } from "vitest/config";

// Level 3: real windows, real daemon, one test at a time. The default `pnpm test` never picks these files up
// (its include is src/**), and they need lab/tests/gui-smoke.sh to set the environment.
//
// `lab-import.e2e.ts` and `lab-connect.e2e.ts` need the laboratory (the profiles of lab/out, the real daemon of a client) and write
// outside the run, so the smoke test does not run them: each runs alone, and only when its script asks for it with
// GALLERIA_LAB_IMPORT (`lab/tests/gui-lab-import.sh`) or GALLERIA_LAB_CONNECT (`lab/tests/gui-lab-connect.sh`).
const labImport = process.env.GALLERIA_LAB_IMPORT === "1";
const labConnect = process.env.GALLERIA_LAB_CONNECT === "1";
const lab = ["e2e/lab-import.e2e.ts", "e2e/lab-connect.e2e.ts"];

export default defineConfig({
  test: {
    environment: "node",
    include: labImport ? ["e2e/lab-import.e2e.ts"] : labConnect ? ["e2e/lab-connect.e2e.ts"] : ["e2e/**/*.e2e.ts"],
    exclude: labImport || labConnect ? [] : lab,
    fileParallelism: false,
    testTimeout: labConnect ? 180_000 : 60_000,
    hookTimeout: 30_000,
  },
});
