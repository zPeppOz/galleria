import { useEffect, useRef } from "react";
import { InlineAlert } from "../design/InlineAlert";
import { errorText, useT } from "../i18n";
import { useConnection } from "./store";

/**
 * The error of a command about this profile (`gui_secret_missing`, `backend_unavailable`, `gui_keyring_locked`, a session of someone
 * else…): inline in the detail, and it takes the focus, because the person has to do something before trying again (Q27). It is not
 * announced; the focus is. The error state of a session is not here: it is the status panel, and Rust announces it.
 */
export function CommandNotice({ profileId }: { profileId: string }) {
  const t = useT();
  const { error } = useConnection();
  const box = useRef<HTMLDivElement>(null);
  const mine = error && error.id === profileId ? error.error : null;
  useEffect(() => {
    if (mine) box.current?.focus();
  }, [mine]);
  if (!mine) return null;
  return (
    <div className="detail-keyring" ref={box} tabIndex={-1} data-notice="command">
      <InlineAlert tone="error" toneLabel={t("alert.error")}>
        {errorText(t, mine.code, mine.message)}
      </InlineAlert>
    </div>
  );
}
