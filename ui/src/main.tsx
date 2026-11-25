import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { loadUiPrefs } from "./app/uiState";
import "./design/tokens.css";
import "./design/base.css";
import { applyTheme, resolveTheme, systemPrefersLight } from "./design/theme";

// The theme goes on the root before React mounts, so the first paint already has the right colours: first the one of
// the system, then the saved one as soon as `state.toml` answers (it is read before mounting, with a limit of 300 ms).
applyTheme(document.documentElement, resolveTheme("system", systemPrefersLight()));

async function start() {
  const prefs = await loadUiPrefs();
  applyTheme(document.documentElement, resolveTheme(prefs.theme, systemPrefersLight()));
  const root = document.getElementById("root");
  if (root) {
    createRoot(root).render(
      <StrictMode>
        <App lang={prefs.lang} theme={prefs.theme} sort={prefs.sort} />
      </StrictMode>,
    );
  }
}

void start();
