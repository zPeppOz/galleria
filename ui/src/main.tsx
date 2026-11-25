import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./design/tokens.css";
import "./design/base.css";
import { applyTheme, resolveTheme, systemPrefersLight } from "./design/theme";

// The theme goes on the root before React mounts, so the first paint already has the right colours (T3 takes the
// choice over from the preferences).
applyTheme(document.documentElement, resolveTheme("system", systemPrefersLight()));

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
