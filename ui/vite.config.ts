import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// The Tauri window loads ui/dist from disk, so every asset path must be relative.
export default defineConfig({
  base: "./",
  plugins: [react()],
  build: { outDir: "dist", emptyOutDir: true, target: "es2022" },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["./src/test/setup.ts"],
  },
});
