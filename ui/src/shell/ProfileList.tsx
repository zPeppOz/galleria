import { useId, useImperativeHandle, useRef, type KeyboardEvent, type Ref } from "react";
import type { Protocol, VpnState } from "../daemon/types";
import { icons } from "../design/icons";
import { useT } from "../i18n";
import "./ProfileList.css";
import { ProtocolTag } from "./ProtocolTag";

/** `state` is the state of the session that uses this profile, when it has one that is not «disconnected» (T4). */
export interface ListProfile { id: string; name: string; server: string; protocol: Protocol; favourite?: boolean; state?: VpnState }
export interface ListGroup { id: string; title: string; profiles: ListProfile[] }

function Option(props: {
  profile: ListProfile;
  selected: boolean;
  tabStop: boolean;
  setRef: (id: string, el: HTMLDivElement | null) => void;
  onSelect: (id: string) => void;
  onKeyDown: (event: KeyboardEvent<HTMLDivElement>, id: string) => void;
}) {
  const { profile, selected, tabStop, setRef, onSelect, onKeyDown } = props;
  const t = useT();
  const id = useId();
  const parts = { name: `${id}-name`, fav: `${id}-fav`, server: `${id}-server`, protocol: `${id}-protocol`, state: `${id}-state` };
  return (
    <div
      ref={(el) => setRef(profile.id, el)}
      role="option"
      className="profile-option"
      aria-selected={selected}
      tabIndex={tabStop ? 0 : -1}
      // The state is part of the name: «Ufficio Napoli Connected», in words and not only in the colour of the dot
      aria-labelledby={profile.state ? `${parts.name} ${parts.state}` : parts.name}
      aria-describedby={`${profile.favourite ? `${parts.fav} ` : ""}${parts.server} ${parts.protocol}`}
      onClick={() => onSelect(profile.id)}
      onKeyDown={(event) => onKeyDown(event, profile.id)}
    >
      <span className={`profile-dot${profile.state ? ` profile-dot-${profile.state}` : ""}`} aria-hidden="true" />
      <span className="profile-text">
        <span className="profile-name">
          <span id={parts.name}>{profile.name}</span>
          {profile.favourite ? <span className="profile-star" aria-hidden="true">{icons.star}</span> : null}
        </span>
        <span id={parts.server} className="profile-server">{profile.server}</span>
        {profile.state ? <span id={parts.state} className="profile-state">{t(`state.${profile.state}`)}</span> : null}
      </span>
      {profile.favourite ? <span id={parts.fav} className="visually-hidden">{t("list.favourite")}</span> : null}
      <ProtocolTag id={parts.protocol} protocol={profile.protocol} />
    </div>
  );
}

/**
 * The profiles in groups, as a listbox with one stop for the Tab key: the arrows, Home and End move the focus and the
 * choice together (the choice follows the focus), a letter jumps to the next name that starts with it. It gets its
 * data from its props. When nothing matches the search (`groups` is empty and there is an `empty` text) the listbox stays
 * in the page and the text says so next to it; `focusRef` lets the search field hand the focus over to the list.
 */
export function ProfileList(props: {
  label: string;
  groups: ListGroup[];
  selectedId: string;
  onSelect: (id: string) => void;
  empty?: string;
  focusRef?: Ref<{ focus: () => void }>;
}) {
  const { label, groups, selectedId, onSelect, empty, focusRef } = props;
  const elements = useRef(new Map<string, HTMLDivElement>());
  const flat = groups.flatMap((group) => group.profiles);
  const tabStop = flat.some((p) => p.id === selectedId) ? selectedId : flat[0]?.id;
  useImperativeHandle(focusRef, () => ({ focus: () => (tabStop ? elements.current.get(tabStop)?.focus() : undefined) }), [tabStop]);

  const go = (profile: ListProfile | undefined) => {
    if (!profile) return;
    onSelect(profile.id);
    const el = elements.current.get(profile.id);
    el?.focus();
    el?.scrollIntoView?.({ block: "nearest" });
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>, from: string) => {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    const current = flat.findIndex((p) => p.id === from);
    let target: ListProfile | undefined;
    if (event.key === "ArrowDown") target = flat[Math.min(current + 1, flat.length - 1)];
    else if (event.key === "ArrowUp") target = flat[Math.max(current - 1, 0)];
    else if (event.key === "Home") target = flat[0];
    else if (event.key === "End") target = flat[flat.length - 1];
    else if (event.key.length === 1 && event.key !== " ") {
      const letter = event.key.toLowerCase();
      const order = [...flat.slice(current + 1), ...flat.slice(0, current + 1)];
      target = order.find((p) => p.name.toLowerCase().startsWith(letter));
      if (!target) return;
    } else return;
    event.preventDefault();
    go(target);
  };

  return (
    <>
      <div className="profile-list" role="listbox" aria-label={label}>
        {groups.map((group) => (
          <div key={group.id} className="profile-group" role="group" aria-label={group.title}>
            <div className="profile-group-title" aria-hidden="true">
              <span>{group.title}</span>
              <span>{group.profiles.length}</span>
            </div>
            {group.profiles.map((profile) => (
              <Option
                key={profile.id}
                profile={profile}
                selected={profile.id === selectedId}
                tabStop={profile.id === tabStop}
                setRef={(id, el) => (el ? elements.current.set(id, el) : elements.current.delete(id))}
                onSelect={onSelect}
                onKeyDown={onKeyDown}
              />
            ))}
          </div>
        ))}
      </div>
      {flat.length === 0 && empty ? <p className="profile-list-empty">{empty}</p> : null}
    </>
  );
}
