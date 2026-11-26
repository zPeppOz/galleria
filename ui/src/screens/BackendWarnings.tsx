import { PROTOCOL_NAME } from "../daemon/protocol";
import type { DaemonView } from "../daemon/types";
import { InlineAlert } from "../design/InlineAlert";
import { hasKey, useT } from "../i18n";

/** The VPN backends the service cannot use (OpenVPN missing or too old…), as one warning; nothing when all are available. */
export function BackendWarnings({ view }: { view: DaemonView }) {
  const t = useT();
  const warnings = view.backends.filter((b) => !b.available);
  if (warnings.length === 0) return null;
  return (
    <InlineAlert tone="warn" toneLabel={t("alert.warn")} title={t("status.warnings")}>
      <ul>
        {warnings.map((b) => {
          const key = `backend.${b.unavailable_reason ?? ""}`;
          return (
            <li key={b.protocol}>
              {hasKey(key) ? t(key) : t("backend.unavailable", { protocol: PROTOCOL_NAME[b.protocol], reason: b.unavailable_reason ?? "?" })}
            </li>
          );
        })}
      </ul>
    </InlineAlert>
  );
}
