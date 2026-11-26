import { useRef } from "react";
import { Button } from "../design/Button";
import { icons } from "../design/icons";
import { useToast } from "../design/Toast";
import { useT } from "../i18n";
import { useUpdate } from "./store";
import "../design/InlineAlert.css";
import "./UpdateNotice.css";

/** The first stop of the page that comes after `from` and is not inside it: where the focus goes when the notice that held it goes away. */
function nextStopAfter(from: HTMLElement, inside: HTMLElement): HTMLElement | null {
  const stops = [...document.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled]), [tabindex="0"]')];
  const after = stops.filter((stop) => !inside.contains(stop) && (from.compareDocumentPosition(stop) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0);
  return after[0] ?? stops.find((stop) => !inside.contains(stop)) ?? null;
}

/**
 * A newer version, in line at the top of the page (P11). It is text that Tab finds, **not** a live region and not announced (Q27, rule 3): the
 * address is text to read and to select (Q55 (a)), with «Copy link» and «Don't remind me». Nothing in it opens an address: the window makes no
 * request but the one of the check, and that is Rust's.
 */
export function UpdateNotice() {
  const { view } = useUpdate();
  if (!view || !view.enabled || !view.latest || !view.url) return null;
  return <Notice current={view.current} latest={view.latest} url={view.url} />;
}

function Notice({ current, latest, url }: { current: string; latest: string; url: string }) {
  const t = useT();
  const toast = useToast();
  const { dismiss } = useUpdate();
  const box = useRef<HTMLElement>(null);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(url);
      toast(t("update.copied"), { tone: "ok" });
    } catch {
      toast(t("update.copy_failed"), { tone: "error" });
    }
  };
  const dismissIt = async () => {
    // the button that has the focus goes away with the notice: the focus goes on to the next stop of the page
    const from = document.activeElement;
    const next = from instanceof HTMLElement && box.current ? nextStopAfter(from, box.current) : null;
    try {
      await dismiss(latest);
      setTimeout(() => (document.activeElement && document.activeElement !== document.body ? undefined : next?.focus()), 0);
    } catch {
      toast(t("update.dismiss_failed"), { tone: "error" });
    }
  };
  return (
    <section ref={box} className="update-notice gx-alert gx-alert-info" aria-labelledby="update-notice-title">
      <span className="gx-alert-icon" aria-hidden="true">{icons.info}</span>
      <div className="gx-alert-body">
        <p id="update-notice-title" className="gx-alert-title">{t("update.title", { version: latest })}</p>
        <div className="gx-alert-text">
          <p className="update-notice-text">{t("update.body", { current })}</p>
          <code className="update-notice-url">{url}</code>
        </div>
        <div className="gx-alert-actions">
          <Button icon={icons.copy} onClick={() => void copy()}>{t("update.copy")}</Button>
          <Button onClick={() => void dismissIt()}>{t("update.dismiss")}</Button>
        </div>
      </div>
    </section>
  );
}
