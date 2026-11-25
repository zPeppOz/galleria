import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { icons } from "./icons";
import "./Toast.css";

type Tone = "info" | "ok" | "error";
type ShowToast = (message: string, opts?: { tone?: Tone }) => void;
/** Tells a screen reader a notice (Q27): the live regions below are silent under Orca, so the app sends it another way. */
export type Announce = (text: string, assertive: boolean) => void;

interface Entry { id: number; message: string; tone: Tone }

const ToastContext = createContext<ShowToast | null>(null);

/** How long a toast stays: the error stays longer. A toast under the pointer or with the focus inside does not leave. */
const STAY_MS: Record<Tone, number> = { info: 5000, ok: 5000, error: 8000 };
const TONE_ICON = { info: icons.info, ok: icons.check, error: icons.error } as const;

/** Somewhere to put the focus when the element that had it goes away: the first stop of the page that is not a notice. */
function firstStopOutsideNotices(): HTMLElement | null {
  const stops = document.querySelectorAll<HTMLElement>('button:not([disabled]), [tabindex="0"], input:not([disabled]), select:not([disabled])');
  return [...stops].find((el) => !el.closest(".gx-toasts")) ?? null;
}

function ToastItem(props: { entry: Entry; closeLabel: string; onDismiss: (id: number) => void }) {
  const { entry, closeLabel, onDismiss } = props;
  const [held, setHeld] = useState(false);
  const box = useRef<HTMLDivElement | null>(null);
  // What had the focus before it came into the notice: where it goes back to when the notice is closed from the keyboard.
  const before = useRef<Element | null>(null);
  const dismiss = () => {
    const focusIsInside = box.current?.contains(document.activeElement) ?? false;
    onDismiss(entry.id);
    if (!focusIsInside) return;
    const back = before.current instanceof HTMLElement && before.current.isConnected && !box.current?.contains(before.current) ? before.current : firstStopOutsideNotices();
    back?.focus();
  };
  useEffect(() => {
    if (held) return undefined;
    const timer = setTimeout(() => onDismiss(entry.id), STAY_MS[entry.tone]);
    return () => clearTimeout(timer);
  }, [held, entry.id, entry.tone, onDismiss]);
  return (
    <div
      ref={box}
      className={`gx-toast gx-toast-${entry.tone}`}
      onMouseEnter={() => setHeld(true)}
      onMouseLeave={() => setHeld(false)}
      onFocus={(event) => {
        if (!box.current?.contains(event.relatedTarget as Node | null)) before.current = event.relatedTarget as Element | null;
        setHeld(true);
      }}
      onBlur={() => setHeld(false)}
    >
      {TONE_ICON[entry.tone]}
      <span className="gx-toast-message">{entry.message}</span>
      <button type="button" className="gx-icon-button" aria-label={closeLabel} onClick={dismiss}>
        {icons.close}
      </button>
    </div>
  );
}

/**
 * Short notices that go away by themselves. The two live regions are in the page from the start and empty, because a
 * screen reader only announces what is added to a region it already knows: `status` (polite) for information, `alert`
 * (assertive) for errors. `aria-live` is written out as well, because that is what WebKitGTK exports as the politeness
 * of the region. A notice never takes the focus. When `announce` is given it is called once for every notice shown, with
 * the visible text (`assertive` for the error tone): the live regions are what a screen reader is meant to read, but
 * Orca does not say them (docs/08, «Spike Orca»), so the app sends the same sentence by the ATK signal as well.
 */
export function ToastProvider({ closeLabel, announce, children }: { closeLabel: string; announce?: Announce; children: ReactNode }) {
  const [entries, setEntries] = useState<Entry[]>([]);
  const next = useRef(0);
  // The latest function, without making `show` change: a notice is announced by the one that is current when it shows.
  const announceRef = useRef(announce);
  announceRef.current = announce;
  const dismiss = useCallback((id: number) => setEntries((list) => list.filter((entry) => entry.id !== id)), []);
  const show = useCallback<ShowToast>((message, opts) => {
    next.current += 1;
    const tone = opts?.tone ?? "info";
    const entry = { id: next.current, message, tone };
    setEntries((list) => [...list, entry]);
    // Once per notice, and outside the state update (which React may run twice). The text is the one on the screen.
    announceRef.current?.(message, tone === "error");
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
        <div role="status" aria-live="polite" aria-atomic="true">{toasts(true)}</div>
        <div role="alert" aria-live="assertive" aria-atomic="true">{toasts(false)}</div>
      </div>
    </ToastContext.Provider>
  );
}

export function useToast(): ShowToast {
  const show = useContext(ToastContext);
  if (!show) throw new Error("useToast needs <ToastProvider> above it");
  return show;
}
