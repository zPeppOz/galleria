import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { saveUpdatePrefs } from "../app/uiState";
import { updateView, type UpdateView } from "../daemon/commands";

interface UpdateState {
  /** What Rust knows, or `null` while it has not said (or when there is no one to ask). */
  view: UpdateView | null;
  /** Turns the check on or off. Rejects when it cannot be written: the choice goes back and whoever called says so. */
  setCheck: (on: boolean) => Promise<void>;
  /** «Don't remind me» for this version. */
  dismiss: (version: string) => Promise<void>;
}

const INERT: UpdateState = { view: null, setCheck: async () => undefined, dismiss: async () => undefined };
const UpdateContext = createContext<UpdateState>(INERT);
export const useUpdate = (): UpdateState => useContext(UpdateContext);

const isText = (value: unknown): value is string => typeof value === "string" && value.length > 0 && value.length <= 200;

/** What Rust sent, or nothing: an answer that is not a view of this shape is not heard (and never reaches the screen as it is). */
function coerce(raw: unknown): UpdateView | null {
  const value = (typeof raw === "object" && raw !== null ? raw : {}) as Record<string, unknown>;
  if (!isText(value.current) || typeof value.enabled !== "boolean") return null;
  const found = isText(value.latest) && isText(value.url);
  return { current: value.current, latest: found ? (value.latest as string) : null, url: found ? (value.url as string) : null, enabled: value.enabled };
}

/**
 * The version check as the page sees it (plan B6, T5): asks Rust what it knows when the window starts and hears `galleria:update` when a
 * request finds a version. It never asks for a request: that is Rust's, once a day. The only file that listens to the event.
 */
export function UpdateProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<UpdateView | null>(null);
  const live = useRef(true);
  const read = useCallback(async () => {
    try {
      const next = coerce(await updateView());
      if (live.current && next) setView(next);
    } catch {
      // no backend behind the page: nothing is known
    }
  }, []);
  useEffect(() => {
    live.current = true;
    void read();
    let closed = false;
    let unlisten: UnlistenFn | null = null;
    listen<unknown>("galleria:update", (event) => {
      const next = coerce(event.payload);
      if (next) setView(next);
    }).then(
      (fn) => {
        if (closed) fn();
        else unlisten = fn;
      },
      () => undefined,
    );
    return () => {
      live.current = false;
      closed = true;
      unlisten?.();
    };
  }, [read]);
  const setCheck = useCallback(
    async (on: boolean) => {
      // the choice shows at once; if it cannot be written it goes back
      setView((now) => (now ? { ...now, enabled: on, ...(on ? {} : { latest: null, url: null }) } : now));
      try {
        await saveUpdatePrefs({ update_check: on });
      } catch (failure) {
        await read();
        throw failure;
      }
      await read();
    },
    [read],
  );
  const dismiss = useCallback(
    async (version: string) => {
      setView((now) => (now && now.latest === version ? { ...now, latest: null, url: null } : now));
      try {
        await saveUpdatePrefs({ update_dismissed: version });
      } catch (failure) {
        await read();
        throw failure;
      }
      await read();
    },
    [read],
  );
  const value = useMemo<UpdateState>(() => ({ view, setCheck, dismiss }), [view, setCheck, dismiss]);
  return <UpdateContext.Provider value={value}>{children}</UpdateContext.Provider>;
}
