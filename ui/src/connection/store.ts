import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { createContext, useContext, useEffect, useState } from "react";
import type { CommandError } from "../daemon/commands";
import type { BackendInfo, LinkState, Stats, Status } from "../daemon/types";

/** What the page knows of the connection and can do about it. The state is the flow of events (spec §3.3), never the answer of a call. */
export interface Connection {
  /** The link to the service; `null` before the first answer. */
  link: LinkState | null;
  /** The session as the service last reported it. */
  status: Status | null;
  backends: BackendInfo[];
  /** The profile whose connection was asked for and has not shown up in the events yet: its button is busy, not «Connected». */
  pending: string | null;
  /** The error of the last command (not the error state of a session), with the profile it was about. */
  error: { id: string; error: CommandError } | null;
  /** When the page first saw the session connected: the service gives no `since`, so the duration counts from here. */
  connectedAt: number | null;
  connect: (id: string) => Promise<void>;
  disconnect: () => Promise<void>;
  clearError: () => void;
}

/** What a part of the page that is drawn without a provider sees: no service, nothing to do. */
export const INERT: Connection = {
  link: null,
  status: null,
  backends: [],
  pending: null,
  error: null,
  connectedAt: null,
  connect: async () => undefined,
  disconnect: async () => undefined,
  clearError: () => undefined,
};

export const ConnectionContext = createContext<Connection>(INERT);
export const useConnection = (): Connection => useContext(ConnectionContext);

/**
 * D28: connecting another profile while a session of the rubrica is up replaces it, with no question. The first call is asked without
 * `replace`; when the service says a session is active and Rust says it is one of ours (`foreign: false`), the second call asks for
 * the replacement. A session that is not ours (`foreign: true`) is never replaced here: the error goes up. The name of the profile
 * that was left is returned, so that the caller can say it once.
 */
export async function connectOrReplace(call: (id: string, replace: boolean) => Promise<void>, id: string): Promise<{ replaced: string | null }> {
  try {
    await call(id, false);
    return { replaced: null };
  } catch (failure) {
    const error = failure as CommandError;
    const profile = error.data?.profile as { name?: string } | undefined;
    if (error.code === "already_active" && error.data?.foreign === false) {
      await call(id, true);
      return { replaced: profile?.name ?? "" };
    }
    throw failure;
  }
}

/** The state that makes the button of a profile «Cancel», «Disconnect»… for a session of it: anything but `disconnected`. */
export const isLive = (status: Status | null): status is Status => status !== null && status.state !== "disconnected";

/**
 * The counters of the session, live. They come from `galleria:stats` (and the snapshot of the status when there is one) and live in
 * the component that shows them, so a second of traffic redraws that component and not the window. Gone with the session.
 */
export function useStats(status: Status | null): Stats | null {
  const [stats, setStats] = useState<Stats | null>(null);
  const session = status?.session ?? null;
  const connected = status?.state === "connected";
  useEffect(() => {
    if (!connected) {
      setStats(null);
      return undefined;
    }
    setStats(status?.stats ?? null);
    let closed = false;
    let unlisten: UnlistenFn | null = null;
    // no backend behind the page (a test, a browser): the numbers stay empty and nothing is raised
    listen<Stats>("galleria:stats", (event) => setStats(event.payload)).then((fn) => {
      if (closed) fn();
      else unlisten = fn;
    }, () => undefined);
    return () => {
      closed = true;
      unlisten?.();
    };
    // the snapshot is read when the session starts; after that only the events count
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [connected, session]);
  return stats;
}

/** A clock for the things that count seconds (the duration, the age of the handshake): the time of now, once a second while `on`. */
export function useNow(on: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!on) return undefined;
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [on]);
  return now;
}
