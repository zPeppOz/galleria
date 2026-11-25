import { PROTOCOL_NAME, PROTOCOL_TAG } from "../daemon/protocol";
import type { Protocol } from "../daemon/types";
import "./ProtocolTag.css";

/** The short tag of a protocol («WG», «OVPN»), hidden from assistive technology; next to it the full name, said by the screen reader. */
export function ProtocolTag({ protocol, id }: { protocol: Protocol; id?: string }) {
  return (
    <span id={id} className={`protocol-tag protocol-tag-${protocol}`}>
      <span aria-hidden="true">{PROTOCOL_TAG[protocol]}</span>
      <span className="visually-hidden">{PROTOCOL_NAME[protocol]}</span>
    </span>
  );
}
