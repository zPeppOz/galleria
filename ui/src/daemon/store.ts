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
    const unlisten: UnlistenFn[] = [];
    const keep = (fn: UnlistenFn) => (closed ? fn() : unlisten.push(fn));
    void (async () => {
      try {
        // Listen first, then ask for the snapshot, so nothing that happens in between is lost.
        keep(await listen<DaemonView>("galleria:link", (e) => setView(e.payload)));
        keep(await listen<Status>("galleria:state", (e) => setView((v) => (v ? { ...v, status: e.payload } : v))));
        const [first, app] = await Promise.all([invoke<DaemonView>("daemon_view"), invoke<AppInfo>("app_info")]);
        if (!closed) {
          setView(first);
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
