/// <reference types="node" />
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { describe, expect, it } from "vitest";

const SRC = join(import.meta.dirname, "..");

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sources(path);
    return /\.(ts|tsx)$/.test(name) ? [path] : [];
  });
}

describe("design boundary", () => {
  it("only_the_design_folder_imports_the_headless_library", () => {
    const offenders = sources(SRC)
      .filter((file) => !relative(SRC, file).startsWith(`design${sep}`))
      .filter((file) => file !== import.meta.filename)
      .filter((file) => /@base-ui\/react|radix-ui/.test(readFileSync(file, "utf8")))
      .map((file) => relative(SRC, file));
    expect(offenders).toEqual([]);
  });
});
