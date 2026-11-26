import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import { useConnection } from "../connection/store";
import { windowShow } from "../daemon/commands";
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

/** What Rust asks of the page when an entry of the menu is chosen: an action and, for a connection, the id of the profile (P6). */
type TrayAction = { action: "connect"; id: string } | { action: "disconnect" };

const isTrayAction = (payload: unknown): payload is TrayAction => {
  const value = payload as { action?: unknown; id?: unknown } | null;
  return value?.action === "disconnect" || (value?.action === "connect" && typeof value.id === "string");
};

/**
 * The page does what the menu of the tray asks (plan B6, Q52): the same functions as the buttons of the window, with their rules (the
 * replacement of a session of ours without a question, the confirmation for a session that is not ours, the window of the credentials). When
 * what comes of it needs the person (a question, an error) the window is shown first, because the page lives on with the window hidden.
 * This is the only file that listens to `galleria:tray-action`.
 */
export function useTrayActions(): void {
  const connection = useConnection();
  const latest = useRef(connection);
  latest.current = connection;
  useEffect(() => {
    let closed = false;
    let unlisten: UnlistenFn | null = null;
    const show = () => {
      windowShow().catch(() => undefined);
    };
    // no backend behind the page (a test, a browser): nothing to hear
    listen<unknown>("galleria:tray-action", (event) => {
      if (!isTrayAction(event.payload)) return;
      const now = latest.current;
      if (event.payload.action === "connect") {
        void now.connect(event.payload.id, { onAttention: show });
      } else if (now.foreign !== null) {
        // the session is not ours: nobody ends it without being asked, and the question is in the window
        now.askToDisconnect();
        show();
      } else {
        void now.disconnect({ onAttention: show });
      }
    }).then(
      (fn) => {
        if (closed) fn();
        else unlisten = fn;
      },
      () => undefined,
    );
    return () => {
      closed = true;
      unlisten?.();
    };
  }, []);
}
