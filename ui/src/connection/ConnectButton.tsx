import { useId } from "react";
import { Button } from "../design/Button";
import { useT } from "../i18n";
import { useConnection, type Connection } from "./store";

/** What the main button of a profile does, from the state of its session; `none` when there is nothing to do (it is going away). */
export type Primary = "connect" | "cancel" | "disconnect" | "retry" | "none";

export function primaryOf(connection: Connection, id: string): Primary {
  const status = connection.status;
  if (!status || status.profile?.id !== id || status.state === "disconnected") return "connect";
  switch (status.state) {
    case "connecting":
    case "needs_input":
    case "reconnecting":
      return "cancel";
    case "connected":
      return "disconnect";
    case "error":
      return "retry";
    default:
      return "none";
  }
}

/** Does what the main button does, for the row that has the focus (`Enter`, T5): `null` when the button is off or has nothing to do. */
export function activate(connection: Connection, id: string): void {
  if (connection.link !== "connected" || connection.pending !== null) return;
  const primary = primaryOf(connection, id);
  if (primary === "connect" || primary === "retry") void connection.connect(id);
  else if (primary === "cancel" || primary === "disconnect") void connection.disconnect();
}

/**
 * The main button of a profile: «Connect», «Cancel» while it connects, «Disconnect» when it is up, off while it disconnects, and
 * «Retry» with «Close» after an error. The button follows the events and not the answers (spec §3.3): between the click and the first
 * event it is busy. On the other profiles «Connect» stays on, and it replaces the session (D28). Without the service it is off, and
 * says why.
 */
export function ConnectButton({ profile }: { profile: { id: string; name: string } }) {
  const t = useT();
  const connection = useConnection();
  const reason = useId();
  const reachable = connection.link === "connected";
  const primary = primaryOf(connection, profile.id);
  const busy = connection.pending === profile.id;
  const why = reachable ? undefined : t(connection.link === null ? "bar.connecting" : `bar.${connection.link}`);
  const gone = !reachable ? { disabled: true, "aria-describedby": reason } : {};
  const note = why ? (
    <span id={reason} className="visually-hidden">
      {why}
    </span>
  ) : null;

  if (primary === "retry") {
    return (
      <>
        <Button variant="primary" onClick={() => void connection.connect(profile.id)} busy={busy} {...gone}>
          {t("action.retry")}
        </Button>
        <Button onClick={() => void connection.disconnect()} {...gone}>
          {t("action.dismiss")}
        </Button>
        {note}
      </>
    );
  }
  if (primary === "none") {
    return (
      <Button disabled>
        {t("state.disconnecting")}
      </Button>
    );
  }
  const label = { connect: t("action.connect"), cancel: t("action.cancel"), disconnect: t("action.disconnect") }[primary];
  return (
    <>
      <Button
        variant={primary === "connect" ? "primary" : "default"}
        busy={primary === "connect" && busy}
        onClick={() => void (primary === "connect" ? connection.connect(profile.id) : connection.disconnect())}
        {...gone}
      >
        {primary === "connect" && busy ? t("state.connecting") : label}
      </Button>
      {note}
    </>
  );
}
