import { cleanup } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach } from "vitest";

// Vitest runs without globals here, so Testing Library cannot register its own cleanup.
afterEach(() => {
  cleanup();
  clearMocks();
});
