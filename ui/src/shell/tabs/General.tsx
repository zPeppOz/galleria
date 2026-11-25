import { Field } from "../../design/Field";
import { Segmented } from "../../design/Segmented";
import { Switch } from "../../design/Switch";
import { useT } from "../../i18n";
import { peerLabel, type TabProps } from "./tab";

/** «General» (D26): name, group, notes, the endpoint of each WireGuard peer, the reconnect switch. An OpenVPN profile shows its server and transport, which come from the file. */
export function General({ detail, draft, edit, errors, groups, settings, setSwitch }: TabProps) {
  const t = useT();
  const transport = detail.openvpn?.remotes[0]?.transport ?? null;
  // The groups of the list, and the one the profile has when the list does not (a file written by hand).
  const own = detail.group && !groups.includes(detail.group) ? [detail.group] : [];
  const options = [{ value: "", label: t("group.none") }, ...[...groups, ...own].map((g) => ({ value: g, label: g }))];
  const peers = draft.wireguard?.peers ?? [];
  return (
    <div className="detail-form">
      <h2 className="detail-section">{t("form.connection")}</h2>
      <Field name="name" label={t("field.name")} value={draft.name} error={errors.name} onChange={(name) => edit((d) => ({ ...d, name }), "name")} />
      <Field name="group" label={t("field.group")} as="select" value={draft.group} options={options} onChange={(group) => edit((d) => ({ ...d, group }), "group")} />
      {detail.protocol === "openvpn" ? (
        <>
          <Field label={t("field.server")} mono value={detail.server ?? ""} readOnly />
          {transport ? (
            <div className="detail-transport">
              <span className="detail-transport-label" aria-hidden="true">{t("field.transport")}</span>
              <Segmented label={t("field.transport")} value={transport} readOnly options={[{ value: "udp", label: "UDP" }, { value: "tcp", label: "TCP" }]} onChange={() => undefined} />
              <p className="detail-hint">{t("field.transport.hint")}</p>
            </div>
          ) : null}
        </>
      ) : (
        peers.map((peer, i) => (
          <Field
            // eslint-disable-next-line react/no-array-index-key
            key={i}
            name={`peers.${i}.endpoint`}
            label={peerLabel(t, peers.length, "field.endpoint", "field.peer_endpoint", i)}
            mono
            value={peer.endpoint}
            error={errors[`peers.${i}.endpoint`]}
            onChange={(endpoint) => edit((d) => ({ ...d, wireguard: d.wireguard && { ...d.wireguard, peers: d.wireguard.peers.map((p, j) => (j === i ? { ...p, endpoint } : p)) } }), `peers.${i}.endpoint`)}
          />
        ))
      )}
      <Field name="notes" label={t("field.notes")} as="textarea" value={draft.notes} error={errors.notes} onChange={(notes) => edit((d) => ({ ...d, notes }), "notes")} />
      <h2 className="detail-section">{t("form.behaviour")}</h2>
      <Switch label={t("field.auto_reconnect")} hint={t("field.auto_reconnect.hint")} checked={settings.auto_reconnect} onChange={(on) => setSwitch("auto_reconnect", on)} />
    </div>
  );
}
