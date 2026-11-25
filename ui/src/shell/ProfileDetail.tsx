import { useState } from "react";
import { ConfirmDialog } from "../design/ConfirmDialog";
import { Field } from "../design/Field";
import { icons } from "../design/icons";
import { Menu, type MenuEntry } from "../design/Menu";
import { Segmented } from "../design/Segmented";
import { Switch } from "../design/Switch";
import { Tabs } from "../design/Tabs";
import { useToast } from "../design/Toast";
import { useT } from "../i18n";
import { DEMO_GROUPS, type DemoProfile } from "./demo-data";
import "./ProfileDetail.css";
import { ProtocolTag } from "./ProtocolTag";

const TABS = ["general", "auth", "network", "config", "log"] as const;
type TabId = (typeof TABS)[number];

/** The form of the «General» tab: the fields of D26 as an example. It keeps what is typed in memory and saves nothing. */
function GeneralForm({ profile }: { profile: DemoProfile }) {
  const t = useT();
  const [name, setName] = useState(profile.name);
  const [group, setGroup] = useState<string>(profile.group);
  const [server, setServer] = useState(profile.server);
  const [autoReconnect, setAutoReconnect] = useState(true);
  return (
    <div className="detail-form">
      <h2 className="detail-section">{t("form.connection")}</h2>
      <Field label={t("field.name")} value={name} onChange={setName} />
      <Field label={t("field.group")} as="select" value={group} onChange={setGroup} options={DEMO_GROUPS.map((g) => ({ value: g, label: t(`group.${g}`) }))} />
      <Field label={t("field.server")} mono value={server} onChange={setServer} />
      {profile.protocol === "openvpn" && profile.transport ? (
        <div className="detail-transport">
          <span className="detail-transport-label" aria-hidden="true">{t("field.transport")}</span>
          <Segmented
            label={t("field.transport")}
            value={profile.transport}
            readOnly
            options={[{ value: "UDP", label: "UDP" }, { value: "TCP", label: "TCP" }]}
            onChange={() => undefined}
          />
          <p className="detail-hint">{t("field.transport.hint")}</p>
        </div>
      ) : null}
      <h2 className="detail-section">{t("form.behaviour")}</h2>
      <Switch label={t("field.auto_reconnect")} hint={t("field.auto_reconnect.hint")} checked={autoReconnect} onChange={setAutoReconnect} />
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
 * The profile that is chosen: header, «More actions», status strip and the five tabs. The strip is empty (the live state
 * is B5) and the actions change nothing (B3): each says so with a notice.
 */
export function ProfileDetail({ profile, groupTitle }: { profile: DemoProfile; groupTitle: string }) {
  const t = useT();
  const toast = useToast();
  const [tab, setTab] = useState<TabId>("general");
  const [confirming, setConfirming] = useState(false);
  const noChange = () => toast(t("demo.no_change"));

  const entries: MenuEntry[] = [
    { kind: "item", id: "duplicate", label: t("detail.duplicate"), onSelect: noChange },
    { kind: "item", id: "export", label: t("detail.export"), onSelect: noChange },
    { kind: "separator" },
    { kind: "item", id: "delete", label: t("detail.delete"), danger: true, onSelect: () => setConfirming(true) },
  ];
  const cells: [string, string][] = [
    ["strip.state", t("strip.state")],
    ["strip.ip", t("strip.ip")],
    ["strip.traffic", t("strip.traffic")],
    profile.protocol === "wireguard" ? ["strip.handshake", t("strip.handshake")] : ["strip.duration", t("strip.duration")],
  ];
  return (
    <div className="detail">
      <div className="detail-head">
        <div className="detail-titlerow">
          <h1 className="detail-title">{profile.name}</h1>
          {profile.favourite ? <span className="detail-star" role="img" aria-label={t("list.favourite")}>{icons.star}</span> : null}
        </div>
        <div className="detail-actions">
          <Menu label={t("detail.more")} trigger={<>{icons.more}{t("detail.more")}</>} entries={entries} />
        </div>
        <p className="detail-meta">
          <ProtocolTag protocol={profile.protocol} />
          <span>{groupTitle}</span>
          <span className="detail-server">{profile.server}</span>
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
          panel: id === "general" ? <GeneralForm key={profile.id} profile={profile} /> : <p className="detail-later">{t("pane.arrives_later")}</p>,
        }))}
      />
      <ConfirmDialog
        open={confirming}
        danger
        title={t("detail.delete.title", { name: profile.name })}
        body={t("detail.delete.body")}
        confirmLabel={t("detail.delete")}
        cancelLabel={t("detail.delete.cancel")}
        onCancel={() => setConfirming(false)}
        onConfirm={() => {
          setConfirming(false);
          noChange();
        }}
      />
    </div>
  );
}
