import { useState } from "react";
import { Field } from "../design/Field";
import { icons } from "../design/icons";
import { InlineAlert } from "../design/InlineAlert";
import { Menu, type MenuEntry } from "../design/Menu";
import { Segmented } from "../design/Segmented";
import { Tabs } from "../design/Tabs";
import { useToast } from "../design/Toast";
import { errorText, useT } from "../i18n";
import type { CmdError, ProfileDetail as Detail, ProfileSummary } from "../rubrica/types";
import "./ProfileDetail.css";
import { ProtocolTag } from "./ProtocolTag";

const TABS = ["general", "auth", "network", "config", "log"] as const;
type TabId = (typeof TABS)[number];

/** The «General» tab, for now only to read: the fields of D26 as the profile has them. The tabs that write come with C. */
function GeneralForm({ detail }: { detail: Detail }) {
  const t = useT();
  const groupLabel = detail.group ?? t("group.none");
  const transport = detail.openvpn?.remotes[0]?.transport ?? null;
  return (
    <div className="detail-form">
      <h2 className="detail-section">{t("form.connection")}</h2>
      <Field label={t("field.name")} value={detail.name} readOnly />
      <Field label={t("field.group")} as="select" value={detail.group ?? ""} readOnly options={[{ value: detail.group ?? "", label: groupLabel }]} />
      <Field label={t("field.server")} mono value={detail.server ?? ""} readOnly />
      {detail.protocol === "openvpn" && transport ? (
        <div className="detail-transport">
          <span className="detail-transport-label" aria-hidden="true">{t("field.transport")}</span>
          <Segmented
            label={t("field.transport")}
            value={transport}
            readOnly
            options={[{ value: "udp", label: "UDP" }, { value: "tcp", label: "TCP" }]}
            onChange={() => undefined}
          />
          <p className="detail-hint">{t("field.transport.hint")}</p>
        </div>
      ) : null}
      <h2 className="detail-section">{t("form.behaviour")}</h2>
      <Field
        label={t("field.auto_reconnect")}
        hint={t("field.auto_reconnect.hint")}
        value={detail.settings.auto_reconnect ? t("value.yes") : t("value.no")}
        readOnly
      />
    </div>
  );
}

function Empty() {
  const t = useT();
  return (
    <>
      <span aria-hidden="true">—</span>
      <span className="visually-hidden">{t("strip.empty")}</span>
    </>
  );
}

/**
 * The profile that is chosen: header, «More actions», status strip and the five tabs. The header is drawn from the row
 * of the list, which is already there; the form waits for the detail. The strip is empty (the live state is B5) and
 * the three actions say that they arrive later (P10).
 */
export function ProfileDetail({ profile, detail, detailError }: { profile: ProfileSummary; detail: Detail | null; detailError: CmdError | null }) {
  const t = useT();
  const toast = useToast();
  const [tab, setTab] = useState<TabId>("general");
  const later = () => toast(t("action.arrives_later"));

  const entries: MenuEntry[] = [
    { kind: "item", id: "duplicate", label: t("detail.duplicate"), onSelect: later },
    { kind: "item", id: "export", label: t("detail.export"), onSelect: later },
    { kind: "separator" },
    { kind: "item", id: "delete", label: t("detail.delete"), danger: true, onSelect: later },
  ];
  const cells: [string, string][] = [
    ["strip.state", t("strip.state")],
    ["strip.ip", t("strip.ip")],
    ["strip.traffic", t("strip.traffic")],
    profile.protocol === "wireguard" ? ["strip.handshake", t("strip.handshake")] : ["strip.duration", t("strip.duration")],
  ];
  const general = detailError ? (
    <InlineAlert tone="error" toneLabel={t("alert.error")}>{errorText(t, detailError.code, detailError.message)}</InlineAlert>
  ) : detail ? (
    <GeneralForm key={detail.id} detail={detail} />
  ) : (
    <p className="detail-later" aria-busy="true">{t("detail.loading")}</p>
  );
  return (
    <div className="detail">
      <div className="detail-head">
        <div className="detail-titlerow">
          <h1 className="detail-title">{profile.name}</h1>
          {profile.favorite ? <span className="detail-star" role="img" aria-label={t("list.favourite")}>{icons.star}</span> : null}
        </div>
        <div className="detail-actions">
          <Menu label={t("detail.more")} trigger={<>{icons.more}{t("detail.more")}</>} entries={entries} />
        </div>
        <p className="detail-meta">
          <ProtocolTag protocol={profile.protocol} />
          <span>{profile.group ?? t("group.none")}</span>
          {profile.server ? <span className="detail-server">{profile.server}</span> : null}
        </p>
      </div>
      <dl className="detail-strip">
        {cells.map(([key, label]) => (
          <div key={key} className="detail-strip-cell">
            <dt>{label}</dt>
            <dd><Empty /></dd>
          </div>
        ))}
      </dl>
      <Tabs
        label={t("tabs.label")}
        value={tab}
        onChange={(next) => setTab(next as TabId)}
        tabs={TABS.map((id) => ({
          id,
          label: t(`tab.${id}`),
          panel: id === "general" ? general : <p className="detail-later">{t("pane.arrives_later")}</p>,
        }))}
      />
    </div>
  );
}
