import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";
import { Button } from "../design/Button";
import { ConfirmDialog } from "../design/ConfirmDialog";
import { Dialog } from "../design/Dialog";
import { Field } from "../design/Field";
import { icons } from "../design/icons";
import { useToast } from "../design/Toast";
import { errorText, useT, type Translate } from "../i18n";
import type { CmdError, GroupActions, ProfileSummary } from "../rubrica/types";
import { checkLabel, GROUP_MAX } from "../rubrica/validate";
import "./GroupsDialog.css";
import { issueText } from "./tabs/tab";

/** Where the focus goes once the list has been redrawn: a field or a button of the row of a group, or the field of the new one. */
type Target = { group: string; part: "name" | "up" | "down" } | { group: null };

const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();
const count = (t: Translate, n: number) => (n === 1 ? t("groups.members.one") : t("groups.members.other", { count: n }));

/** The text for a refusal about a name: the reason of `gui_group_invalid` when the catalogue has one, else the text of the code. */
function nameProblem(t: Translate, error: CmdError): string {
  const reason = (error.data as { reason?: string } | undefined)?.reason;
  return error.code === "gui_group_invalid" && reason && reason !== "not_a_permutation" ? issueText(t, "name", reason) : errorText(t, error.code, error.message);
}

/**
 * «Manage groups» (P9): a modal window with a row for each group of the list (the name, which is renamed with Enter or the
 * button that shows when it changed; up, down, delete), the groups that only the profiles have (to add to the list) and
 * the field for a new group. A group is only a name: deleting one keeps its profiles, and says how many are left without.
 * An error about a name asks for an answer, so it goes on the field and takes the focus; what worked is a notice, except
 * a move, which the focus following the row says well enough.
 */
export function GroupsDialog(props: { open: boolean; onClose: () => void; groups: string[]; profiles: ProfileSummary[]; actions: GroupActions }) {
  const { open, onClose, groups, profiles, actions } = props;
  const t = useT();
  const toast = useToast();
  const body = useRef<HTMLDivElement>(null);
  const [edits, setEdits] = useState<Record<string, string>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [fresh, setFresh] = useState("");
  const [freshError, setFreshError] = useState("");
  const [asking, setAsking] = useState<string | null>(null);
  const [target, setTarget] = useState<{ at: Target; turn: number } | null>(null);
  const busy = useRef(false);

  const members = (name: string) => profiles.filter((p) => p.group === name).length;
  const unlisted = [...new Set(profiles.flatMap((p) => (p.group && !groups.includes(p.group) ? [p.group] : [])))].sort((a, b) => a.localeCompare(b));
  const focusOn = (at: Target) => setTarget((was) => ({ at, turn: (was?.turn ?? 0) + 1 }));

  // After the list is redrawn the focus goes where `target` says; a button that is off for the new place hands it to the other one.
  useEffect(() => {
    if (!target || !body.current) return;
    const root = body.current;
    const at = target.at;
    if (at.group === null) {
      root.querySelector<HTMLElement>(".groups-new input")?.focus();
      return;
    }
    const row = [...root.querySelectorAll<HTMLElement>("[data-group]")].find((el) => el.dataset.group === at.group);
    const part = (which: "name" | "up" | "down") => row?.querySelector<HTMLElement>(which === "name" ? "input" : `.groups-buttons button:nth-child(${which === "up" ? 1 : 2})`) ?? null;
    const usable = (el: HTMLElement | null) => (el && !(el as HTMLButtonElement).disabled ? el : null);
    const other = at.part === "up" ? "down" : "up";
    (at.part === "name" ? part("name") : (usable(part(at.part)) ?? usable(part(other))))?.focus();
  }, [target]);

  /** `field`: the row whose name was being written, `null` for the field of the new group, nothing for a write that has no name in it. */
  const fail = (error: unknown, field?: string | null) => {
    const problem = error as CmdError;
    const onName = problem.code === "gui_group_exists" || (problem.code === "gui_group_invalid" && (problem.data as { reason?: string } | undefined)?.reason !== "not_a_permutation");
    if (onName && field !== undefined) {
      const text = nameProblem(t, problem);
      if (field === null) setFreshError(text);
      else setErrors((now) => ({ ...now, [field]: text }));
      focusOn(field === null ? { group: null } : { group: field, part: "name" });
      return;
    }
    toast(errorText(t, problem.code, problem.message), { tone: "error" });
  };

  /** Runs one write: one at a time (the next would carry a list that the first has just made stale). */
  const write = async (job: () => Promise<void>) => {
    if (busy.current) return;
    busy.current = true;
    try {
      await job();
    } finally {
      busy.current = false;
    }
  };

  // What the window can know without asking: the same rules as Rust, which decides on its own all the same.
  const problemWith = (name: string, among: string[]): string | null => {
    const trimmed = name.trim();
    const reason = checkLabel(trimmed, GROUP_MAX);
    if (reason) return issueText(t, "name", reason);
    return among.some((g) => same(g, trimmed)) ? t("error.gui_group_exists") : null;
  };

  const create = (name: string, how: "new" | "added") =>
    write(async () => {
      const problem = problemWith(name, groups);
      if (problem) {
        setFreshError(problem);
        focusOn({ group: null });
        return;
      }
      try {
        const next = await actions.create(name);
        const made = next[next.length - 1] ?? name.trim();
        if (how === "new") {
          setFresh("");
          setFreshError("");
        }
        // the new row takes the focus; for a group that was only in use by profiles the button that had it is gone
        focusOn({ group: made, part: "name" });
        toast(t(how === "new" ? "groups.created" : "groups.added", { name: made }), { tone: "ok" });
      } catch (error) {
        fail(error, null);
      }
    });

  const rename = (from: string) =>
    write(async () => {
      const to = (edits[from] ?? from).trim();
      if (to === from) {
        setEdits(({ [from]: _unchanged, ...rest }) => rest);
        return;
      }
      const problem = problemWith(to, groups.filter((g) => g !== from));
      if (problem) {
        setErrors((now) => ({ ...now, [from]: problem }));
        focusOn({ group: from, part: "name" });
        return;
      }
      try {
        await actions.rename(from, to);
        setEdits(({ [from]: _done, ...rest }) => rest);
        setErrors(({ [from]: _done, ...rest }) => rest);
        focusOn({ group: to, part: "name" });
        toast(t("groups.renamed", { name: to }), { tone: "ok" });
      } catch (error) {
        fail(error, from);
      }
    });

  const move = (name: string, by: -1 | 1) =>
    write(async () => {
      const at = groups.indexOf(name);
      const next = [...groups];
      next.splice(at, 1);
      next.splice(at + by, 0, name);
      try {
        await actions.reorder(next);
        focusOn({ group: name, part: by < 0 ? "up" : "down" });
      } catch (error) {
        fail(error);
      }
    });

  const remove = (name: string) =>
    write(async () => {
      const at = groups.indexOf(name);
      const neighbour = groups[at + 1] ?? groups[at - 1] ?? null;
      try {
        await actions.remove(name);
        focusOn(neighbour === null ? { group: null } : { group: neighbour, part: "name" });
        toast(t("groups.deleted", { name }), { tone: "ok" });
      } catch (error) {
        fail(error);
      }
    });

  const askOrDelete = (name: string) => {
    if (members(name) > 0) setAsking(name);
    else void remove(name);
  };

  const submit = (event: FormEvent, job: () => void) => {
    event.preventDefault();
    job();
  };

  const clearError = useCallback((key: string) => setErrors(({ [key]: _gone, ...rest }) => rest), []);
  const askedCount = asking === null ? 0 : members(asking);

  return (
    <>
      <Dialog open={open} onClose={onClose} title={t("groups.title")} description={t("groups.description")} closeLabel={t("dialog.close")}>
        <div className="groups" ref={body}>
          {groups.length === 0 ? <p className="groups-empty">{t("groups.none")}</p> : null}
          {groups.length > 0 ? (
            <ul className="groups-list" aria-label={t("groups.listed")}>
              {groups.map((name, index) => {
                const edited = edits[name] !== undefined && edits[name].trim() !== name;
                return (
                  <li key={name} className="groups-row" data-group={name}>
                    <form className="groups-name" onSubmit={(event) => submit(event, () => void rename(name))}>
                      <Field
                        name={`group:${name}`}
                        label={t("groups.row", { n: index + 1 })}
                        value={edits[name] ?? name}
                        error={errors[name]}
                        onChange={(value) => {
                          setEdits((now) => ({ ...now, [name]: value }));
                          clearError(name);
                        }}
                      />
                      {edited ? <Button onClick={() => void rename(name)} aria-label={t("groups.rename", { name })}>{t("groups.rename.short")}</Button> : null}
                    </form>
                    <span className="groups-count">{count(t, members(name))}</span>
                    <div className="groups-buttons">
                      <Button variant="ghost" icon={icons.arrowUp} aria-label={t("groups.up", { name })} disabled={index === 0} onClick={() => void move(name, -1)} />
                      <Button variant="ghost" icon={icons.arrowDown} aria-label={t("groups.down", { name })} disabled={index === groups.length - 1} onClick={() => void move(name, 1)} />
                      <Button variant="ghost" icon={icons.trash} aria-label={t("groups.delete", { name })} onClick={() => askOrDelete(name)} />
                    </div>
                  </li>
                );
              })}
            </ul>
          ) : null}
          {unlisted.length > 0 ? (
            <section className="groups-unlisted" aria-labelledby="groups-unlisted-title">
              <h3 id="groups-unlisted-title" className="groups-subtitle">{t("groups.unlisted")}</h3>
              <ul className="groups-list">
                {unlisted.map((name) => (
                  <li key={name} className="groups-row groups-row-unlisted">
                    <span className="groups-unlisted-name">{name}</span>
                    <span className="groups-count">{count(t, members(name))}</span>
                    <Button onClick={() => void create(name, "added")} aria-label={t("groups.unlisted.add", { name })}>{t("groups.add.short")}</Button>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
          <form className="groups-new" onSubmit={(event) => submit(event, () => void create(fresh, "new"))}>
            <Field
              name="new-group"
              label={t("groups.new")}
              value={fresh}
              error={freshError || undefined}
              onChange={(value) => {
                setFresh(value);
                setFreshError("");
              }}
            />
            <Button variant="primary" onClick={() => void create(fresh, "new")}>{t("groups.add")}</Button>
          </form>
        </div>
      </Dialog>
      <ConfirmDialog
        open={asking !== null}
        title={t("groups.delete.title", { name: asking ?? "" })}
        body={askedCount === 1 ? t("groups.delete.body.one") : t("groups.delete.body.other", { count: askedCount })}
        confirmLabel={t("groups.delete.confirm")}
        cancelLabel={t("groups.delete.cancel")}
        danger
        onConfirm={() => {
          const name = asking;
          setAsking(null);
          if (name !== null) void remove(name);
        }}
        onCancel={() => setAsking(null)}
      />
    </>
  );
}
