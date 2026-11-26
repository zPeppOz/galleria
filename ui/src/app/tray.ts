import { invoke } from "@tauri-apps/api/core";
import { useEffect } from "react";
import { useT, type Translate } from "../i18n";

/** `{name}` stays as it is in the text: Rust puts the name of the profile there when it draws the menu (P3). */
const NAME = { name: "{name}" };

/** The texts of the menu of the tray, in the language of `t`. */
export function trayLabels(t: Translate) {
  return {
    open: t("tray.open"),
    quit: t("tray.quit"),
    quit_connected: t("tray.quit_connected"),
    connect: t("tray.connect", NAME),
    disconnect: t("tray.disconnect", NAME),
    connected: t("tray.state.connected", NAME),
    disconnected: t("tray.state.disconnected"),
    connecting: t("tray.state.connecting"),
    unreachable: t("tray.state.unreachable"),
  };
}

/**
 * The menu of the tray is drawn by Rust from the state, the favourites and these texts (plan B6, T1): the catalogues are the page's
 * (D22), so the page gives the texts at the start and again at every change of language. The first delivery makes the tray. This is
 * the only file that invokes `tray_labels`. A refusal is swallowed: the tray is an accessory and the window does not depend on it.
 */
export function useTrayLabelsForRust(): void {
  const t = useT();
  useEffect(() => {
    try {
      invoke("tray_labels", { labels: trayLabels(t) }).catch(() => undefined);
    } catch {
      // no backend behind the page: nothing to tell
    }
  }, [t]);
}

