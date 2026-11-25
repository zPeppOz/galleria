import { hasKey, type Translate } from "../../i18n";
import type { Draft } from "../../rubrica/draft";
import type { ProfileDetail, ProfileSettings } from "../../rubrica/types";

/** What a tab of the form is given: the saved profile, the draft the text fields change, and the texts of the fields that are wrong. */
export interface TabProps {
  /** The profile as it is on disk. The switches show this, and writing one goes through `setSwitch`. */
  detail: ProfileDetail;
  draft: Draft;
  /** Changes the draft and forgets the error of the fields it touches. */
  edit: (change: (draft: Draft) => Draft, touched: string) => void;
  /** field path → text of the error. */
  errors: Record<string, string>;
  groups: string[];
  /** The settings with a write in flight shown as they will be, so that a switch does not wait for the disk to move. */
  settings: ProfileSettings;
  /** A switch writes at once (P7). */
  setSwitch: (key: keyof ProfileSettings, value: boolean) => void;
}

/** The text of a problem with a field: the one for this field and this code, else the one for the code. */
export function issueText(t: Translate, field: string, code: string): string {
  const last = field.split(".").pop() ?? field;
  const specific = `issue.${last}.${code}`;
  if (hasKey(specific)) return t(specific);
  const generic = `issue.${code}`;
  return hasKey(generic) ? t(generic) : code;
}

/** «Endpoint» for the only peer, «Endpoint of peer 2» when there are several. */
export function peerLabel(t: Translate, peers: number, single: "field.endpoint" | "field.public_key" | "field.allowed_ips" | "field.keepalive", many: "field.peer_endpoint" | "field.peer_public_key" | "field.peer_allowed_ips" | "field.peer_keepalive", index: number): string {
  return peers > 1 ? t(many, { n: index + 1 }) : t(single);
}
