import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { connectProfile, disconnectSession, type CommandError } from "../daemon/commands";
import type { DaemonView } from "../daemon/types";
import { useToast } from "../design/Toast";
import { useT } from "../i18n";
import { ConnectionContext, connectOrReplace, type Connection } from "./store";

/** How long a button stays busy waiting for the first event of a connection that was accepted. */
const PENDING_MS = 10_000;

/**
 * Gives the parts of the window the connection: the daemon link and the session as the events bring them, and the two things the
 * window can do about it. It sits under the notices because a replacement says so once, with one.
 */
export function ConnectionProvider({ view, children }: { view: DaemonView | null; children: ReactNode }) {
  const t = useT();
  const toast = useToast();
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<Connection["error"]>(null);
  const [connectedAt, setConnectedAt] = useState<{ session: number; at: number } | null>(null);
  const status = view?.status ?? null;
  const link = view?.link ?? null;

  // The button is busy until the events show the session of that profile (or its error), the link goes, or too long has passed.
  useEffect(() => {
    if (pending === null) return undefined;
    if (link !== null && link !== "connected") {
      setPending(null);
      return undefined;
    }
    if (status && status.state !== "disconnected" && status.profile?.id === pending) {
      setPending(null);
      return undefined;
    }
    const timer = setTimeout(() => setPending(null), PENDING_MS);
    return () => clearTimeout(timer);
  }, [pending, link, status]);

  const session = status?.state === "connected" ? status.session : null;
  useEffect(() => {
    if (session !== null) setConnectedAt((was) => (was?.session === session ? was : { session, at: Date.now() }));
  }, [session]);

  const latest = useRef({ t, toast });
  latest.current = { t, toast };

  const connect = useCallback(async (id: string) => {
    setError(null);
    setPending(id);
    try {
      const { replaced } = await connectOrReplace(connectProfile, id);
      if (replaced) latest.current.toast(latest.current.t("connect.replaced", { name: replaced }), { tone: "info" });
    } catch (failure) {
      setPending(null);
      setError({ id, error: failure as CommandError });
    }
  }, []);

  const current = useRef(status);
  current.current = status;
  const disconnect = useCallback(async () => {
    setError(null);
    try {
      await disconnectSession();
    } catch (failure) {
      setError({ id: current.current?.profile?.id ?? "", error: failure as CommandError });
    }
  }, []);
  const clearError = useCallback(() => setError(null), []);

  const value = useMemo<Connection>(
    () => ({
      link,
      status,
      backends: view?.backends ?? [],
      pending,
      error,
      connectedAt: session !== null && connectedAt?.session === session ? connectedAt.at : null,
      connect,
      disconnect,
      clearError,
    }),
    [link, status, view?.backends, pending, error, connectedAt, session, connect, disconnect, clearError],
  );
  return <ConnectionContext.Provider value={value}>{children}</ConnectionContext.Provider>;
}
