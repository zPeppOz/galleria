import { Field } from "../../design/Field";
import { Switch } from "../../design/Switch";
import { useT } from "../../i18n";
import { peerLabel, type TabProps } from "./tab";

const FULL_TUNNEL = new Set(["0.0.0.0/0", "::/0"]);

/**
 * «Network» (D26). WireGuard: addresses, DNS, search domains, MTU, and for each peer its routed networks and keepalive.
 * «All traffic goes through the tunnel» is an indication read from the routed networks, not a control: the networks are
 * what is edited. OpenVPN: only the two switches the profile has; what the server pushes is shown while connected (B5).
 */
export function Network({ draft, edit, errors, settings, setSwitch }: TabProps) {
  const t = useT();
  const wg = draft.wireguard;
  const setWg = (change: (w: NonNullable<typeof wg>) => NonNullable<typeof wg>, touched: string) => edit((d) => ({ ...d, wireguard: d.wireguard && change(d.wireguard) }), touched);
  const fullTunnel = wg?.peers.some((p) => p.allowed_ips.split(/[\s,]+/).some((net) => FULL_TUNNEL.has(net))) ?? false;
  return (
    <div className="detail-form">
      {wg ? (
        <>
          <h2 className="detail-section">{t("form.addressing")}</h2>
          <Field name="addresses" label={t("field.addresses")} hint={t("field.addresses.hint")} mono value={wg.addresses} error={errors.addresses} onChange={(addresses) => setWg((w) => ({ ...w, addresses }), "addresses")} />
          <Field name="dns" label={t("field.dns")} hint={t("field.dns.hint")} mono value={wg.dns} error={errors.dns} onChange={(dns) => setWg((w) => ({ ...w, dns }), "dns")} />
          <Field name="search_domains" label={t("field.search_domains")} hint={t("field.search_domains.hint")} mono value={wg.search_domains} error={errors.search_domains} onChange={(search_domains) => setWg((w) => ({ ...w, search_domains }), "search_domains")} />
          <Field name="mtu" label={t("field.mtu")} hint={t("field.mtu.hint")} mono inputMode="numeric" value={wg.mtu} error={errors.mtu} onChange={(mtu) => setWg((w) => ({ ...w, mtu }), "mtu")} />
          <h2 className="detail-section">{t("form.routing")}</h2>
          <p className="detail-fact">{fullTunnel ? t("net.full_tunnel") : t("net.split_tunnel")}</p>
          {wg.peers.map((peer, i) => (
            // eslint-disable-next-line react/no-array-index-key
            <div key={i} className="detail-peer">
              {wg.peers.length > 1 ? <h3 className="detail-peer-title">{t("form.peer", { n: i + 1 })}</h3> : null}
              <Field
                name={`peers.${i}.allowed_ips`}
                label={peerLabel(t, wg.peers.length, "field.allowed_ips", "field.peer_allowed_ips", i)}
                hint={t("field.allowed_ips.hint")}
                mono
                value={peer.allowed_ips}
                error={errors[`peers.${i}.allowed_ips`]}
                onChange={(allowed_ips) => setWg((w) => ({ ...w, peers: w.peers.map((p, j) => (j === i ? { ...p, allowed_ips } : p)) }), `peers.${i}.allowed_ips`)}
              />
              <Field
                name={`peers.${i}.persistent_keepalive`}
                label={peerLabel(t, wg.peers.length, "field.keepalive", "field.peer_keepalive", i)}
                hint={t("field.keepalive.hint")}
                mono
                inputMode="numeric"
                value={peer.persistent_keepalive}
                error={errors[`peers.${i}.persistent_keepalive`]}
                onChange={(persistent_keepalive) => setWg((w) => ({ ...w, peers: w.peers.map((p, j) => (j === i ? { ...p, persistent_keepalive } : p)) }), `peers.${i}.persistent_keepalive`)}
              />
            </div>
          ))}
        </>
      ) : (
        <>
          <h2 className="detail-section">{t("form.routing")}</h2>
          <Switch label={t("field.ignore_pushed_routes")} hint={t("field.ignore_pushed_routes.hint")} checked={settings.ignore_pushed_routes} onChange={(on) => setSwitch("ignore_pushed_routes", on)} />
          <p className="detail-hint">{t("net.during_connection")}</p>
        </>
      )}
      <h2 className="detail-section">{t("form.behaviour")}</h2>
      <Switch label={t("field.ipv6_block")} hint={t("field.ipv6_block.hint")} checked={settings.ipv6_block} onChange={(on) => setSwitch("ipv6_block", on)} />
    </div>
  );
}
