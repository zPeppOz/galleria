import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { createContext, useContext, useEffect, useState } from "react";
import type { CommandError, InputAnswer } from "../daemon/commands";
import type { BackendInfo, InputKind, LinkState, Stats, Status } from "../daemon/types";

/** The question the service is waiting on and nobody has answered yet: what the window of the credentials draws. */
export interface Question {
  session: number;
  request: number;
  kind: InputKind;
  /** The text of the server, as it came. */
  prompt: string;
  echo: boolean;
  profile: { id: string; name: string };
  /** The time, in milliseconds, when the question runs out: from the moment the event arrived, not from the moment it was drawn. */
  expiresAt: number;
}

/** A question about a session that is not of this rubrica, waiting for the answer (D21): ending it, or replacing it with the profile `target`. `name` is the name of that session. */
export interface Confirmation { kind: "disconnect" | "replace"; name: string; target?: string }

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
  /** The question of the service that waits for an answer; `null` when there is none or it was answered (an answered one does not come back). */
  question: Question | null;
  /** The session of the service that was last seen up, with the profile it was for: the log of a profile that is not connected any more is its last session. `null` until a session has been seen. */
  lastSession: { id: string; name: string; session: number | null } | null;
  /** The session of the service when it is not one of a profile of the rubrica (started by the CLI of another rubrica, or of a profile that was deleted since): `null` when there is none, or while the rubrica has not been read. */
  foreign: Status | null;
  /** The question about a foreign session that waits for an answer; nothing is done before it is answered. */
  confirmation: Confirmation | null;
  /** Asks whether to end the foreign session. */
  askToDisconnect: () => void;
  /** The answer: `true` does what was asked (ends it, or replaces it), `false` drops the question and does nothing. */
  answerConfirmation: (yes: boolean) => Promise<void>;
  connect: (id: string) => Promise<void>;
  disconnect: () => Promise<void>;
  /** Answers the open question. What was typed goes into this one call and nowhere else: the caller empties its fields before it waits. */
  provide: (values: InputAnswer, remember: boolean) => Promise<void>;
  /** Takes the remembered credentials of a profile out of the keyring. */
  forget: (id: string) => Promise<void>;
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
  question: null,
  lastSession: null,
  foreign: null,
  confirmation: null,
  askToDisconnect: () => undefined,
  answerConfirmation: async () => undefined,
  connect: async () => undefined,
  disconnect: async () => undefined,
  provide: async () => undefined,
  forget: async () => undefined,
  clearError: () => undefined,
};

export const ConnectionContext = createContext<Connection>(INERT);
export const useConnection = (): Connection => useContext(ConnectionContext);

/**
 * D28: connecting another profile while a session of the rubrica is up replaces it, with no question. The first call is asked without
 * `replace`; when the service says a session is active and Rust says it is one of ours (`foreign: false`), the second call asks for
 * the replacement. A session that is not ours (`foreign: true`) is never replaced here: the error goes up. The name of the profile
 * that was left is returned, so that the caller can say it once. `onReplacing` is called with that name just before the second call:
 * the notice about the session that was left has to be sent before the service replaces it, because the state of the new one is
 * announced by Rust as soon as the service says it (a notice sent when the call returns can come after «Connected to …»).
 */
export async function connectOrReplace(
  call: (id: string, replace: boolean) => Promise<void>,
  id: string,
  onReplacing?: (left: string) => void,
): Promise<{ replaced: string | null }> {
  try {
    await call(id, false);
    return { replaced: null };
  } catch (failure) {
    const error = failure as CommandError;
    const profile = error.data?.profile as { name?: string } | undefined;
    if (error.code === "already_active" && error.data?.foreign === false) {
      onReplacing?.(profile?.name ?? "");
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

/**
 * The seconds left until `expiresAt`, counted down while it is in the future: refreshed every 10 seconds and, in the last minute, every
 * second. A text, not a live region. `0` when there is nothing to count.
 */
export function useCountdown(expiresAt: number | null): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (expiresAt === null) return undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const tick = () => {
      const at = Date.now();
      setNow(at);
      const left = (expiresAt - at) / 1000;
      if (left > 0) timer = setTimeout(tick, left > 60 ? 10_000 : 1000);
    };
    tick();
    return () => clearTimeout(timer);
  }, [expiresAt]);
  return expiresAt === null ? 0 : Math.max(0, Math.ceil((expiresAt - now) / 1000));
}
