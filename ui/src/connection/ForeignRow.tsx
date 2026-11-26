import { useId } from "react";
import { Button } from "../design/Button";
import { Tabs } from "../design/Tabs";
import { useT } from "../i18n";
import { LogTab } from "../log/LogTab";
import { ProtocolTag } from "../shell/ProtocolTag";
import { Stats } from "./Stats";
import { useConnection } from "./store";
import "./ForeignRow.css";

/**
 * The fixed row of a session that someone else started (D21): above the list, outside the groups and the search, with the name the
 * service gives, the protocol, the state in words and what it is. Choosing it shows its detail; «Disconnect» asks first, because another
 * user of this computer may be using it. There is nothing to edit: the session has no profile in this rubrica.
 */
export function ForeignRow({ selected, onSelect }: { selected: boolean; onSelect: () => void }) {
  const t = useT();
  const { foreign, link, askToDisconnect } = useConnection();
  const id = useId();
  if (!foreign?.profile) return null;
  const name = foreign.profile.name;
  const busy = foreign.state === "disconnecting";
  return (
    <div className="foreign-row" role="group" aria-label={t("foreign.label")}>
      <button type="button" className="foreign-row-main" aria-current={selected ? "true" : undefined} aria-describedby={`${id}-note ${id}-state`} onClick={onSelect}>
        <span className={`profile-dot profile-dot-${foreign.state}`} aria-hidden="true" />
        <span className="foreign-row-text">
          <span className="profile-name">{name}</span>
          <span id={`${id}-note`} className="profile-server">{t("foreign.note")}</span>
          <span id={`${id}-state`} className="profile-state">{t(`state.${foreign.state}`)}</span>
        </span>
        {foreign.protocol ? <ProtocolTag protocol={foreign.protocol} /> : null}
      </button>
      <Button onClick={askToDisconnect} disabled={link !== "connected" || busy} aria-label={t("foreign.disconnect_named", { name })}>
        {t("action.disconnect")}
      </Button>
    </div>
  );
}

/**
 * What the window shows when the foreign session is chosen: the name, the protocol, the numbers and the log, and the one thing to do,
 * to disconnect. The state in words is the panel above the profile (the service area), which every session has.
 */
export function ForeignDetail() {
  const t = useT();
  const { foreign, link, askToDisconnect } = useConnection();
  if (!foreign?.profile) return null;
  const profile = foreign.profile;
  return (
    <div className="detail">
      <div className="detail-head">
        <div className="detail-titlerow">
          <h1 className="detail-title">{profile.name}</h1>
        </div>
        <div className="detail-actions">
          <Button onClick={askToDisconnect} disabled={link !== "connected" || foreign.state === "disconnecting"}>
            {t("action.disconnect")}
          </Button>
        </div>
        <p className="detail-meta">
          {foreign.protocol ? <ProtocolTag protocol={foreign.protocol} /> : null}
          <span>{t("foreign.note")}</span>
        </p>
      </div>
      <Stats profile={{ id: profile.id, protocol: foreign.protocol ?? "openvpn" }} />
      <Tabs label={t("tabs.label")} value="log" onChange={() => undefined} tabs={[{ id: "log", label: t("tab.log"), panel: <LogTab profileId={profile.id} /> }]} />
    </div>
  );
}
