import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { icons } from "./icons";
import "./Toast.css";

type Tone = "info" | "ok" | "error";
type ShowToast = (message: string, opts?: { tone?: Tone }) => void;

interface Entry { id: number; message: string; tone: Tone }

const ToastContext = createContext<ShowToast | null>(null);

/** How long a toast stays: the error stays longer. A toast under the pointer or with the focus inside does not leave. */
const STAY_MS: Record<Tone, number> = { info: 5000, ok: 5000, error: 8000 };
const TONE_ICON = { info: icons.info, ok: icons.check, error: icons.error } as const;

function ToastItem(props: { entry: Entry; closeLabel: string; onDismiss: (id: number) => void }) {
  const { entry, closeLabel, onDismiss } = props;
  const [held, setHeld] = useState(false);
  useEffect(() => {
    if (held) return undefined;
    const timer = setTimeout(() => onDismiss(entry.id), STAY_MS[entry.tone]);
    return () => clearTimeout(timer);
  }, [held, entry.id, entry.tone, onDismiss]);
  return (
    <div className={`gx-toast gx-toast-${entry.tone}`} onMouseEnter={() => setHeld(true)} onMouseLeave={() => setHeld(false)} onFocus={() => setHeld(true)} onBlur={() => setHeld(false)}>
      {TONE_ICON[entry.tone]}
      <span className="gx-toast-message">{entry.message}</span>
      <button type="button" className="gx-icon-button" aria-label={closeLabel} onClick={() => onDismiss(entry.id)}>
        {icons.close}
      </button>
    </div>
  );
}

/**
 * Short notices that go away by themselves. The two live regions are in the page from the start and empty, because a
 * screen reader only announces what is added to a region it already knows: `status` (polite) for information, `alert`
 * (assertive) for errors. A toast never takes the focus.
 */
export function ToastProvider({ closeLabel, children }: { closeLabel: string; children: ReactNode }) {
  const [entries, setEntries] = useState<Entry[]>([]);
  const next = useRef(0);
  const dismiss = useCallback((id: number) => setEntries((list) => list.filter((entry) => entry.id !== id)), []);
  const show = useCallback<ShowToast>((message, opts) => {
    next.current += 1;
    const entry = { id: next.current, message, tone: opts?.tone ?? "info" };
    setEntries((list) => [...list, entry]);
  }, []);
  const toasts = (polite: boolean) =>
    entries
      .filter((entry) => (entry.tone === "error") !== polite)
      .map((entry) => <ToastItem key={entry.id} entry={entry} closeLabel={closeLabel} onDismiss={dismiss} />);
  const value = useMemo(() => show, [show]);
  return (
    <ToastContext.Provider value={value}>
      {children}
      <div className="gx-toasts">
        <div role="status">{toasts(true)}</div>
        <div role="alert">{toasts(false)}</div>
      </div>
    </ToastContext.Provider>
  );
}

export function useToast(): ShowToast {
  const show = useContext(ToastContext);
  if (!show) throw new Error("useToast needs <ToastProvider> above it");
  return show;
}
