import { useReportUnsaved } from "../app/closing";
import { useCallback, useEffect, useRef, useState, type MutableRefObject, type ReactNode } from "react";
import { Button } from "../design/Button";
import { CommandNotice } from "../connection/CommandNotice";
import { ConnectButton } from "../connection/ConnectButton";
import { Stats } from "../connection/Stats";
import { ConfirmDialog } from "../design/ConfirmDialog";
import { icons } from "../design/icons";
import { InlineAlert } from "../design/InlineAlert";
import { LogTab } from "../log/LogTab";
import { Menu, type MenuEntry } from "../design/Menu";
import { Tabs } from "../design/Tabs";
import { useToast } from "../design/Toast";
import { errorText, useT, type Translate } from "../i18n";
import { build, fieldOrder, fieldTab, isDirty, toDraft, type Draft } from "../rubrica/draft";
import type { CmdError, FieldIssue, ProfileActions, ProfileDetail as Detail, ProfilePatch, ProfileSettings, ProfileSummary, RawCheck } from "../rubrica/types";
import { NAME_MAX } from "../rubrica/validate";
import "./ProfileDetail.css";
import { ProtocolTag } from "./ProtocolTag";
import { SaveBar } from "./SaveBar";
import { Authentication } from "./tabs/Authentication";
import { Configuration } from "./tabs/Configuration";
import { General } from "./tabs/General";
import { Network } from "./tabs/Network";
import { issueText } from "./tabs/tab";

const TABS = ["general", "auth", "network", "config", "log"] as const;
type TabId = (typeof TABS)[number];

/** What the shell asks of the form before it lets go of it (to choose another profile): is there something unsaved, save it, drop it. */
export interface Guard {
  dirty: boolean;
  save: () => Promise<boolean>;
  discard: () => void;
  /** Puts the focus back on the first field with an error, after a dialog that was over the form has closed. */
  focusError: () => void;
}

export interface DetailProps {
  profile: ProfileSummary;
  detail: Detail | null;
  detailError: CmdError | null;
  /** The groups of the list, for the group selector. */
  groups: string[];
  /** The names of every profile of the rubrica: a copy takes the first «X (copy N)» that none of them has. */
  names: string[];
  /** The profile is the one of the session the service has open: it cannot be deleted, and the menu says why. */
  inUse: boolean;
  update: (id: string, patch: ProfilePatch, expectedModified: string) => Promise<Detail>;
  /** Checks the text of the editor of an OpenVPN profile (T7): the form asks while the text is typed and before it saves. */
  validateRaw: (id: string, text: string) => Promise<RawCheck>;
  /** Duplicate, export and delete (T8). */
  actions: ProfileActions;
  /** The window chooses this profile (the copy, once it is in the list): the shell asks before it lets go of unsaved edits. */
  onSelect: (id: string) => void;
  /** A profile was deleted and the rubrica has been read again: the shell puts the focus where the next one is. */
  onDeleted: (id: string) => void;
  reload: () => Promise<void>;
  guard?: MutableRefObject<Guard | null>;
}

/**
 * Header, «More actions», status strip and the five tabs. The strip is the live state of the session of this profile (the
 * numbers are not a live region). The panels, the star and the entries of the menu come from the caller: the form, or «reading».
 */
function Frame(props: { profile: ProfileSummary; panels: Record<TabId, ReactNode>; tab: TabId; onTab: (tab: TabId) => void; menu: MenuEntry[]; star?: { pressed: boolean; onToggle: () => void }; above?: ReactNode; below?: ReactNode }) {
  const { profile, panels, tab, onTab, menu, star, above, below } = props;
  const t = useT();
  const entries = menu;
  return (
    <>
      <div className="detail-head">
        <div className="detail-titlerow">
          <h1 className="detail-title">{profile.name}</h1>
          {star ? (
            <button type="button" className="detail-star detail-star-button" aria-pressed={star.pressed} aria-label={t("list.favourite")} onClick={star.onToggle}>
              {icons.star}
            </button>
          ) : profile.favorite ? (
            <span className="detail-star" role="img" aria-label={t("list.favourite")}>{icons.star}</span>
          ) : null}
        </div>
        <div className="detail-actions">
          <ConnectButton profile={profile} />
          <Menu label={t("detail.more")} trigger={<>{icons.more}{t("detail.more")}</>} entries={entries} />
        </div>
        <p className="detail-meta">
          <ProtocolTag protocol={profile.protocol} />
          <span>{profile.group ?? t("group.none")}</span>
          {profile.server ? <span className="detail-server">{profile.server}</span> : null}
        </p>
      </div>
      <Stats profile={profile} />
      {above}
      <Tabs
        label={t("tabs.label")}
        value={tab}
        onChange={(next) => onTab(next as TabId)}
        tabs={TABS.map((id) => ({ id, label: t(`tab.${id}`), panel: panels[id] }))}
      />
      {below}
    </>
  );
}

/** The form: the saved profile, a draft of its text fields, the writes (P7: the switches and the star at once, the text with «Save»). */
function Form({ detail, groups, update, validateRaw, reload, guard, menu }: Pick<DetailProps, "groups" | "update" | "validateRaw" | "reload" | "guard"> & { detail: Detail; menu: MenuEntry[] }) {
  const t = useT();
  const toast = useToast();
  const root = useRef<HTMLDivElement>(null);
  const [base, setBase] = useState(detail);
  const [draft, setDraft] = useState<Draft>(() => toDraft(detail));
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [pending, setPending] = useState<Partial<ProfileSettings & { favorite: boolean }>>({});
  const [busy, setBusy] = useState(false);
  const [changed, setChanged] = useState(false);
  const [adopt, setAdopt] = useState(false);
  const [tab, setTab] = useState<TabId>("general");
  const [focus, setFocus] = useState<{ field: string; turn: number } | null>(null);
  const [barGone, setBarGone] = useState(0);
  // What the check says about the text in the editor (T7): null until the text is changed and the check has answered.
  const [raw, setRaw] = useState<RawCheck | null>(null);
  // One write at a time: a second one would carry a time of modification that the first has just made stale.
  const writing = useRef(false);
  const dirty = isDirty(draft, base);
  // Rust decides a close and a quit on this bit (Q35): the page keeps it up to date.
  useReportUnsaved(dirty);
  const latest = useRef({ base, dirty });
  latest.current = { base, dirty };

  const rebase = useCallback((next: Detail) => {
    setBase(next);
    setDraft(toDraft(next));
    setErrors({});
    setPending({});
    setChanged(false);
    setAdopt(false);
    setRaw(null);
  }, []);

  // The text of an OpenVPN profile is checked 400 ms after the last key, with nothing written. A text that is as saved has
  // nothing to say; an answer for a text that is no longer the one in the editor is dropped.
  const text = draft.config;
  const savedText = base.openvpn?.config ?? null;
  useEffect(() => {
    if (text === null || text === savedText) {
      setRaw(null);
      return undefined;
    }
    let current = true;
    const timer = setTimeout(() => {
      validateRaw(base.id, text).then(
        (check) => {
          if (current) setRaw(check);
        },
        () => undefined,
      );
    }, 400);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [text, savedText, base.id, validateRaw]);

  // The profile changed on disk (the CLI, another window): without edits of ours it is taken in silently; with edits
  // they are kept and the form says so. Only a change of the live detail counts, not our own write coming back.
  const seen = useRef(detail.modified);
  useEffect(() => {
    if (detail.modified !== seen.current) {
      seen.current = detail.modified;
      if (detail.modified !== latest.current.base.modified) {
        if (latest.current.dirty) setChanged(true);
        else rebase(detail);
      }
    }
  }, [detail, rebase]);
  useEffect(() => {
    if (adopt && detail.modified !== base.modified) rebase(detail);
  }, [adopt, detail, base.modified, rebase]);

  const showIssues = useCallback(
    (issues: FieldIssue[]) => {
      const order = fieldOrder(base.wireguard?.peers.length ?? 0);
      const rank = (field: string) => (order.includes(field) ? order.indexOf(field) : order.length);
      const sorted = [...issues].sort((a, b) => rank(a.field) - rank(b.field));
      setErrors(Object.fromEntries(sorted.map((i) => [i.field, issueText(t, i.field, i.code)])));
      // An error that asks for an answer takes the focus (Q27): no notice, the field is read with its error.
      const first = sorted[0];
      if (first) {
        setTab(fieldTab(first.field));
        setFocus((was) => ({ field: first.field, turn: (was?.turn ?? 0) + 1 }));
      }
    },
    [base.wireguard?.peers.length, t],
  );
  useEffect(() => {
    if (!focus) return;
    root.current?.querySelector<HTMLElement>(`[name="${focus.field}"]`)?.focus();
  }, [focus]);

  const showConfigRefused = useCallback(() => {
    setErrors((now) => ({ ...now, openvpn_config: issueText(t, "openvpn_config", "config_issues") }));
    setTab("config");
    setFocus((was) => ({ field: "config_issue", turn: (was?.turn ?? 0) + 1 }));
  }, [t]);

  const fail = useCallback(
    (failure: unknown) => {
      const error = failure as CmdError;
      // The save was refused for the text: the list Rust found is the list to show, and the first one takes the focus.
      if (error.code === "gui_invalid_field" && error.data?.issues?.length) {
        setRaw({ issues: error.data.issues, can_save: false });
        showConfigRefused();
        return;
      }
      if (error.code === "gui_invalid_field" && error.data?.fields?.length) {
        showIssues(error.data.fields);
        return;
      }
      if (error.code === "gui_profile_changed") {
        setChanged(true);
        void reload();
      }
      toast(errorText(t, error.code, error.message), { tone: "error" });
    },
    [reload, showConfigRefused, showIssues, t, toast],
  );

  const save = useCallback(async (): Promise<boolean> => {
    if (writing.current) return false;
    const { patch, issues } = build(draft, base);
    if (issues.length > 0) {
      showIssues(issues);
      return false;
    }
    if (Object.keys(patch).length === 0) {
      setDraft(toDraft(base));
      return true;
    }
    writing.current = true;
    setBusy(true);
    try {
      // The text is checked again right before it is sent, whatever the last check said (Rust checks it once more): a
      // refusal puts the list on the screen and the focus on its first line, and nothing is written.
      if (patch.openvpn_config !== undefined) {
        const check = await validateRaw(base.id, patch.openvpn_config);
        setRaw(check);
        if (!check.can_save) {
          showConfigRefused();
          return false;
        }
      }
      const next = await update(base.id, patch, base.modified);
      rebase(next);
      toast(t("profile.saved"), { tone: "ok" });
      void reload();
      return true;
    } catch (failure) {
      fail(failure);
      return false;
    } finally {
      writing.current = false;
      setBusy(false);
    }
  }, [base, draft, fail, rebase, reload, showConfigRefused, showIssues, t, toast, update, validateRaw]);

  // A switch and the star write at once. The control shows the new value while the write goes; if it fails it goes back.
  const writeNow = useCallback(
    (patch: ProfilePatch, shown: Partial<ProfileSettings & { favorite: boolean }>) => {
      if (writing.current) return;
      writing.current = true;
      setPending(shown);
      update(base.id, patch, base.modified).then(
        (next) => {
          setBase(next);
          setPending({});
          void reload();
        },
        (failure) => {
          setPending({});
          fail(failure);
        },
      ).finally(() => {
        writing.current = false;
      });
    },
    [base.id, base.modified, fail, reload, update],
  );
  const setSwitch = (key: keyof ProfileSettings, value: boolean) => writeNow({ settings: { ...base.settings, [key]: value } }, { [key]: value });
  const settings: ProfileSettings = { ...base.settings, ...pending };
  const favourite = pending.favorite ?? base.favorite;

  const discard = useCallback(() => {
    setDraft(toDraft(latest.current.base));
    setErrors({});
    setRaw(null);
  }, []);
  const focusError = useCallback(() => setFocus((was) => was && { ...was, turn: was.turn + 1 }), []);
  useEffect(() => {
    if (!guard) return undefined;
    guard.current = { dirty, save, discard, focusError };
    return () => {
      guard.current = null;
    };
  });

  const edit = (change: (draft: Draft) => Draft, touched: string) => {
    setDraft(change);
    setErrors((now) => {
      if (!(touched in now)) return now;
      const { [touched]: _gone, ...rest } = now;
      return rest;
    });
  };
  // the names of the secrets are read live: what Rust remembers changes without the profile being edited
  const props = { detail: { ...base, secret_refs: detail.secret_refs }, draft, edit, errors, groups, settings, setSwitch };
  const reloadNow = () => {
    setAdopt(true);
    void reload();
  };
  // Cancel and Save leave the focus where it is; when the button that had it goes away with the bar, the tab takes it.
  const afterBar = (action: () => void | boolean | Promise<unknown>) => async () => {
    const hadFocus = root.current?.querySelector(".savebar")?.contains(document.activeElement) ?? false;
    const result = await action();
    if (hadFocus && result !== false) setBarGone((turn) => turn + 1);
  };
  useEffect(() => {
    if (barGone > 0) root.current?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]')?.focus();
  }, [barGone]);

  return (
    <div className="detail" ref={root}>
      <Frame
        profile={base}
        tab={tab}
        onTab={setTab}
        menu={menu}
        star={{ pressed: favourite, onToggle: () => writeNow({ favorite: !favourite }, { favorite: !favourite }) }}
        above={
          changed ? (
            <InlineAlert tone="warn" toneLabel={t("alert.warn")} actions={<Button onClick={reloadNow}>{t("changed.reload")}</Button>}>
              {t("changed.body")}
            </InlineAlert>
          ) : null
        }
        panels={{
          general: <General {...props} />,
          auth: <Authentication {...props} />,
          network: <Network {...props} />,
          config: <Configuration {...props} issues={raw?.issues ?? []} />,
          log: <LogTab profileId={base.id} />,
        }}
        below={<SaveBar dirty={dirty} busy={busy} onSave={afterBar(save)} onCancel={afterBar(discard)} />}
      />
    </div>
  );
}

/**
 * The profile that is chosen. The header is drawn from the row of the list, which is already there; the form waits for
 * the detail and is remade for every profile (a draft belongs to one profile). The three actions of «More actions» are
 * here, because they work on the profile as it is on the disk and do not wait for the form: the copy takes the name the
 * catalogue proposes, «Export» and «Delete» ask first.
 */
export function ProfileDetail({ profile, detail, detailError, groups, names, inUse, update, validateRaw, actions, onSelect, onDeleted, reload, guard }: DetailProps) {
  const t = useT();
  const toast = useToast();
  const [asking, setAsking] = useState<"export" | "delete" | null>(null);
  // The copy whose «More actions» button takes the focus when the copy is shown: the menu of the old profile goes with it.
  const refocus = useRef<string | null>(null);
  const shownId = detail?.id ?? null;
  useEffect(() => {
    if (refocus.current !== null && shownId === refocus.current) {
      refocus.current = null;
      document.querySelector<HTMLElement>(".detail-actions .gx-menu-trigger")?.focus();
    }
  }, [shownId]);

  // A keyring that is locked (or does not answer) stops a duplicate, an export or a delete: the notice is inline, it takes the
  // focus and it says what to do, because the person has to do something before trying again (Q27: it is not announced).
  const [keyring, setKeyring] = useState<CmdError | null>(null);
  const keyringNotice = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (keyring) keyringNotice.current?.focus();
  }, [keyring]);
  useEffect(() => setKeyring(null), [shownId]);
  const failed = (failure: unknown) => {
    const error = failure as CmdError;
    if (error.code === "gui_keyring_locked" || error.code === "gui_keyring_failed") {
      setKeyring({ ...error });
      return;
    }
    toast(errorText(t, error.code, error.message), { tone: "error" });
  };
  const duplicate = async () => {
    setKeyring(null);
    try {
      const copy = await actions.duplicate(profile.id, copyName(t, profile.name, names));
      toast(t("detail.duplicated", { name: copy.name }), { tone: "ok" });
      refocus.current = copy.id;
      onSelect(copy.id);
    } catch (failure) {
      failed(failure);
    }
  };
  const exportFile = async () => {
    setAsking(null);
    setKeyring(null);
    try {
      // a cancel of the window «Save» is not an outcome: nothing is said
      if (await actions.export(profile.id)) toast(t("detail.exported"), { tone: "ok" });
    } catch (failure) {
      failed(failure);
    }
  };
  const remove = async () => {
    setAsking(null);
    setKeyring(null);
    try {
      await actions.remove(profile.id);
      toast(t("detail.deleted", { name: profile.name }), { tone: "ok" });
      onDeleted(profile.id);
    } catch (failure) {
      failed(failure);
    }
  };
  const menu: MenuEntry[] = [
    { kind: "item", id: "duplicate", label: t("detail.duplicate"), onSelect: () => void duplicate() },
    { kind: "item", id: "export", label: t("detail.export"), onSelect: () => setAsking("export") },
    { kind: "separator" },
    { kind: "item", id: "delete", label: t("detail.delete"), danger: true, disabled: inUse, hint: inUse ? t("detail.delete.in_use") : undefined, onSelect: () => setAsking("delete") },
  ];
  const dialogs = (
    <>
      <ConfirmDialog
        open={asking === "export"}
        title={t("export.title", { name: profile.name })}
        body={t("export.body")}
        confirmLabel={t("export.confirm")}
        cancelLabel={t("export.cancel")}
        onConfirm={() => void exportFile()}
        onCancel={() => setAsking(null)}
      />
      <ConfirmDialog
        open={asking === "delete"}
        danger
        title={t("delete.title", { name: profile.name })}
        body={t("delete.body")}
        confirmLabel={t("delete.confirm")}
        cancelLabel={t("delete.cancel")}
        onConfirm={() => void remove()}
        onCancel={() => setAsking(null)}
      />
    </>
  );

  const keyringNoticeElement = keyring ? (
    <div className="detail-keyring" ref={keyringNotice} tabIndex={-1} data-notice="keyring">
      <InlineAlert tone="error" toneLabel={t("alert.error")}>{errorText(t, keyring.code, keyring.message)}</InlineAlert>
    </div>
  ) : null;

  if (detail) {
    return (
      <>
        {keyringNoticeElement}
        <CommandNotice profileId={profile.id} />
        <Form key={detail.id} detail={detail} groups={groups} update={update} validateRaw={validateRaw} reload={reload} guard={guard} menu={menu} />
        {dialogs}
      </>
    );
  }
  const waiting = detailError ? (
    <InlineAlert tone="error" toneLabel={t("alert.error")}>{errorText(t, detailError.code, detailError.message)}</InlineAlert>
  ) : (
    <p className="detail-later" aria-busy="true">{t("detail.loading")}</p>
  );
  return (
    <div className="detail">
      <CommandNotice profileId={profile.id} />
      <WaitingFrame profile={profile} waiting={waiting} menu={menu} />
      {dialogs}
    </div>
  );
}

function WaitingFrame({ profile, waiting, menu }: { profile: ProfileSummary; waiting: ReactNode; menu: MenuEntry[] }) {
  const [tab, setTab] = useState<TabId>("general");
  return (
    <Frame
      profile={profile}
      tab={tab}
      onTab={setTab}
      menu={menu}
      panels={{ general: waiting, auth: waiting, network: waiting, config: waiting, log: <LogTab profileId={profile.id} /> }}
    />
  );
}

/**
 * The name of a copy: «X (copy)», then «X (copy 2)» and so on, the first that no profile has. It is cut short when it
 * would not fit in a name (the longest is `NAME_MAX`), from the name of the original, so that Rust never refuses it.
 */
export function copyName(t: Translate, name: string, taken: string[]): string {
  const make = (n: number) => {
    const key = n === 1 ? "detail.copy_name" : "detail.copy_name_n";
    const room = Math.max(1, NAME_MAX - [...t(key, { name: "", n })].length);
    return t(key, { name: [...name].slice(0, room).join("").trimEnd(), n });
  };
  const used = new Set(taken);
  let n = 1;
  while (used.has(make(n))) n += 1;
  return make(n);
}

