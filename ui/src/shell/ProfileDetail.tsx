import { useCallback, useEffect, useRef, useState, type MutableRefObject, type ReactNode } from "react";
import { Button } from "../design/Button";
import { icons } from "../design/icons";
import { InlineAlert } from "../design/InlineAlert";
import { Menu, type MenuEntry } from "../design/Menu";
import { Tabs } from "../design/Tabs";
import { useToast } from "../design/Toast";
import { errorText, useT } from "../i18n";
import { build, fieldOrder, fieldTab, isDirty, toDraft, type Draft } from "../rubrica/draft";
import type { CmdError, FieldIssue, ProfileDetail as Detail, ProfilePatch, ProfileSettings, ProfileSummary } from "../rubrica/types";
import "./ProfileDetail.css";
import { ProtocolTag } from "./ProtocolTag";
import { SaveBar } from "./SaveBar";
import { Authentication } from "./tabs/Authentication";
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
  update: (id: string, patch: ProfilePatch, expectedModified: string) => Promise<Detail>;
  reload: () => Promise<void>;
  guard?: MutableRefObject<Guard | null>;
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
 * Header, «More actions», status strip and the five tabs. The strip is empty (the live state is B5) and the three
 * actions say that they arrive later (P10). The panels and the star come from the caller: the form, or «reading».
 */
function Frame(props: { profile: ProfileSummary; panels: Record<TabId, ReactNode>; tab: TabId; onTab: (tab: TabId) => void; star?: { pressed: boolean; onToggle: () => void }; above?: ReactNode; below?: ReactNode }) {
  const { profile, panels, tab, onTab, star, above, below } = props;
  const t = useT();
  const toast = useToast();
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
function Form({ detail, groups, update, reload, guard }: Omit<DetailProps, "profile" | "detail" | "detailError"> & { detail: Detail }) {
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
  // One write at a time: a second one would carry a time of modification that the first has just made stale.
  const writing = useRef(false);
  const dirty = isDirty(draft, base);
  const latest = useRef({ base, dirty });
  latest.current = { base, dirty };

  const rebase = useCallback((next: Detail) => {
    setBase(next);
    setDraft(toDraft(next));
    setErrors({});
    setPending({});
    setChanged(false);
    setAdopt(false);
  }, []);

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

  const fail = useCallback(
    (failure: unknown) => {
      const error = failure as CmdError;
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
    [reload, showIssues, t, toast],
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
  }, [base, draft, fail, rebase, reload, showIssues, t, toast, update]);

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
  const props = { detail: base, draft, edit, errors, groups, settings, setSwitch };
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
          config: <p className="detail-later">{t("pane.arrives_later")}</p>,
          log: <p className="detail-later">{t("pane.arrives_later")}</p>,
        }}
        below={<SaveBar dirty={dirty} busy={busy} onSave={afterBar(save)} onCancel={afterBar(discard)} />}
      />
    </div>
  );
}

/**
 * The profile that is chosen. The header is drawn from the row of the list, which is already there; the form waits for
 * the detail and is remade for every profile (a draft belongs to one profile).
 */
export function ProfileDetail({ profile, detail, detailError, groups, update, reload, guard }: DetailProps) {
  const t = useT();
  if (detail) return <Form key={detail.id} detail={detail} groups={groups} update={update} reload={reload} guard={guard} />;
  const waiting = detailError ? (
    <InlineAlert tone="error" toneLabel={t("alert.error")}>{errorText(t, detailError.code, detailError.message)}</InlineAlert>
  ) : (
    <p className="detail-later" aria-busy="true">{t("detail.loading")}</p>
  );
  return (
    <div className="detail">
      <WaitingFrame profile={profile} waiting={waiting} />
    </div>
  );
}

function WaitingFrame({ profile, waiting }: { profile: ProfileSummary; waiting: ReactNode }) {
  const t = useT();
  const [tab, setTab] = useState<TabId>("general");
  return (
    <Frame
      profile={profile}
      tab={tab}
      onTab={setTab}
      panels={{ general: waiting, auth: waiting, network: waiting, config: <p className="detail-later">{t("pane.arrives_later")}</p>, log: <p className="detail-later">{t("pane.arrives_later")}</p> }}
    />
  );
}

