import { useConnection } from "../../connection/store";
import { Button } from "../../design/Button";
import { Field } from "../../design/Field";
import { useT } from "../../i18n";
import { useSecretStoreKind } from "../../secrets/store";
import { peerLabel, type TabProps } from "./tab";

/**
 * «Authentication» (D26): the public key of each WireGuard peer is a field; the private key is said to be in the store and
 * is never shown. An OpenVPN profile shows its method as a label (it comes from the file, there is nothing to choose)
 * and the names of the secrets it keeps.
 */
export function Authentication({ detail, draft, edit, errors }: TabProps) {
  const t = useT();
  const store = useSecretStoreKind();
  const { forget } = useConnection();
  const saved = detail.protocol === "openvpn" && ["openvpn.username", "openvpn.password"].every((name) => detail.secret_refs.includes(name));
  const peers = draft.wireguard?.peers ?? [];
  const flags = detail.openvpn?.auth;
  const methods = flags
    ? [flags.certificate ? t("auth.method.certificate") : null, flags.user_pass ? t(flags.otp ? "auth.method.user_pass_otp" : "auth.method.user_pass") : null].filter((m): m is string => m !== null)
    : [];
  return (
    <div className="detail-form">
      {detail.protocol === "openvpn" ? (
        <>
          <h2 className="detail-section">{t("auth.method")}</h2>
          <p className="detail-fact">{methods.length > 0 ? methods.join(", ") : t("auth.method.none")}</p>
          <p className="detail-hint">{t("auth.method.hint")}</p>
          {saved ? (
            <>
              <h2 className="detail-section">{t(store === "keyring" ? "auth.saved.keyring" : store === "file" ? "auth.saved.file" : "auth.saved")}</h2>
              <p className="detail-hint">{t("auth.saved.hint")}</p>
              <div>
                <Button onClick={() => void forget(detail.id)}>{t("auth.forget")}</Button>
              </div>
            </>
          ) : null}
          <h2 className="detail-section">{t("auth.secrets")}</h2>
          {detail.secret_refs.length > 0 ? (
            <ul className="detail-secrets">
              {detail.secret_refs.map((name) => (
                <li key={name}><code>{name}</code></li>
              ))}
            </ul>
          ) : (
            <p className="detail-fact">{t("auth.secrets.none")}</p>
          )}
        </>
      ) : (
        <>
          <h2 className="detail-section">{t("form.secrets")}</h2>
          <dl className="detail-facts">
            <dt>{t("auth.private_key")}</dt>
            <dd>{t("auth.private_key.stored")}</dd>
          </dl>
          <h2 className="detail-section">{t("field.public_key")}</h2>
          {peers.map((peer, i) => (
            <Field
              // eslint-disable-next-line react/no-array-index-key
              key={i}
              name={`peers.${i}.public_key`}
              label={peerLabel(t, peers.length, "field.public_key", "field.peer_public_key", i)}
              mono
              value={peer.public_key}
              error={errors[`peers.${i}.public_key`]}
              onChange={(public_key) => edit((d) => ({ ...d, wireguard: d.wireguard && { ...d.wireguard, peers: d.wireguard.peers.map((p, j) => (j === i ? { ...p, public_key } : p)) } }), `peers.${i}.public_key`)}
            />
          ))}
        </>
      )}
    </div>
  );
}
