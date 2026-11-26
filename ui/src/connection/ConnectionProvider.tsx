import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { connectProfile, disconnectSession, forgetCredentials, provideInput, type CommandError, type InputAnswer } from "../daemon/commands";
import type { DaemonView } from "../daemon/types";
import { useToast } from "../design/Toast";
import { useT } from "../i18n";
import { ConnectionContext, connectOrReplace, type Connection, type Question } from "./store";

/** How long a button stays busy waiting for the first event of a connection that was accepted. */
const PENDING_MS = 10_000;

/**
 * Gives the parts of the window the connection: the daemon link and the session as the events bring them, and the two things the
 * window can do about it. It sits under the notices because a replacement says so once, with one. It is also the one place that
 * answers the questions of the service (credentials, a code, a passphrase): the window of the credentials draws `question` and
 * calls `provide`, which marks the question answered before anything is awaited, so that it neither stays nor comes back.
 * `onCredentialsChanged` says that what Rust remembers of the profile may have changed (a connection that was asked to remember
 * its credentials is up; the credentials were forgotten).
 */
export function ConnectionProvider({ view, onCredentialsChanged, children }: { view: DaemonView | null; onCredentialsChanged?: () => void; children: ReactNode }) {
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

  const latest = useRef({ t, toast, onCredentialsChanged });
  latest.current = { t, toast, onCredentialsChanged };

  // Rust writes the credentials it was asked to remember when the session is up, before it tells the page: the page reads again.
  useEffect(() => {
    if (session !== null) latest.current.onCredentialsChanged?.();
  }, [session]);

  // The question of the service, until it is answered. Its clock starts when the event arrives (this memo runs once for it).
  const asked = status?.state === "needs_input" && status.input_request && status.session !== null && status.profile ? status : null;
  const questionKey = asked?.input_request ? `${asked.session}:${asked.input_request.request}` : null;
  const arrivedAt = useMemo(() => Date.now(), [questionKey]); // eslint-disable-line react-hooks/exhaustive-deps
  const [answered, setAnswered] = useState<string | null>(null);
  const answeredNow = useRef<string | null>(null);
  const question = useMemo<Question | null>(() => {
    if (!asked?.input_request || asked.session === null || !asked.profile || questionKey === answered) return null;
    const request = asked.input_request;
    return { session: asked.session, request: request.request, kind: request.kind, prompt: request.prompt, echo: request.echo, profile: asked.profile, expiresAt: arrivedAt + request.expires_in_secs * 1000 };
  }, [asked, questionKey, answered, arrivedAt]);
  const open = useRef<Question | null>(null);
  open.current = question;

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
  const provide = useCallback(async (values: InputAnswer, remember: boolean) => {
    const now = open.current;
    if (!now) return;
    const key = `${now.session}:${now.request}`;
    // once: a second submit of the same question finds it answered, even before the page has been drawn again
    if (answeredNow.current === key) return;
    answeredNow.current = key;
    setAnswered(key);
    setError(null);
    try {
      await provideInput(now.session, now.request, values, remember);
    } catch (failure) {
      setError({ id: now.profile.id, error: failure as CommandError });
    }
  }, []);
  const forget = useCallback(async (id: string) => {
    setError(null);
    try {
      await forgetCredentials(id);
    } catch (failure) {
      setError({ id, error: failure as CommandError });
    }
    latest.current.onCredentialsChanged?.();
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
      question,
      connect,
      disconnect,
      provide,
      forget,
      clearError,
    }),
    [link, status, view?.backends, pending, error, connectedAt, session, question, connect, disconnect, provide, forget, clearError],
  );
  return <ConnectionContext.Provider value={value}>{children}</ConnectionContext.Provider>;
}
