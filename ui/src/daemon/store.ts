import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import type { AppInfo, DaemonView, Status } from "./types";

export interface Daemon { view: DaemonView | null; info: AppInfo | null; retry: () => void }

/** The daemon link as the Rust side keeps it: a first snapshot from `daemon_view`, then the events. */
export function useDaemon(): Daemon {
  const [view, setView] = useState<DaemonView | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    let closed = false;
    // What arrives while the snapshot is on its way is newer than the snapshot: the daemon_view answer reflects
    // the moment it was taken, and an event emitted after that can reach the webview before the answer does.
    let waiting = true;
    let linkSeen = false;
    let stateSeen: Status | null = null;
    const unlisten: UnlistenFn[] = [];
    const keep = (fn: UnlistenFn) => (closed ? fn() : unlisten.push(fn));
    void (async () => {
      try {
        // Listen first, then ask for the snapshot, so nothing that happens in between is lost.
        keep(
          await listen<DaemonView>("galleria:link", (e) => {
            if (waiting) linkSeen = true;
            setView(e.payload);
          }),
        );
        keep(
          await listen<Status>("galleria:state", (e) => {
            if (waiting) stateSeen = e.payload;
            setView((v) => (v ? { ...v, status: e.payload } : v));
          }),
        );
        const [first, app] = await Promise.all([invoke<DaemonView>("daemon_view"), invoke<AppInfo>("app_info")]);
        waiting = false;
        if (!closed) {
          // A link event carries the whole view, so it replaces the snapshot; a state event only its status.
          if (!linkSeen) setView(stateSeen ? { ...first, status: stateSeen } : first);
          setInfo(app);
        }
      } catch (e) {
        console.error("galleria-ui: cannot reach the app backend", e);
      }
    })();
    return () => {
      closed = true;
      unlisten.forEach((fn) => fn());
    };
  }, []);

  const retry = useCallback(() => {
    void invoke("daemon_retry");
  }, []);
  return { view, info, retry };
}
