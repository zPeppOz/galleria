import { render } from "@testing-library/react";
import { useState } from "react";
import type { SortOrder } from "../app/uiState";
import type { DaemonView } from "../daemon/types";
import type { Announce } from "../design/Toast";
import { Preferences } from "../prefs";
import { arrange } from "../rubrica/arrange";
import type { Rubrica } from "../rubrica/store";
import type { CmdError, RubricaView } from "../rubrica/types";
import { Shell } from "../shell/Shell";
import { makeDetail, makeInfo, makeRubricaView, makeView } from "./fixtures";

interface Options {
  lang?: "en" | "it";
  onRetry?: () => void;
  announce?: Announce;
  /** The rubrica the shell shows; null is a read that has not answered yet. */
  rubrica?: RubricaView | null;
  error?: CmdError | null;
  /** The saved sort order the window starts with. */
  sort?: SortOrder;
}

/** The shell with a rubrica that answers at once and keeps its own selection and sort order: no app behind it, nothing to wait for. */
function Harness({ view, options }: { view: DaemonView | null; options: Options }) {
  const list = options.rubrica === undefined ? makeRubricaView() : options.rubrica;
  const [sort, setSort] = useState<SortOrder>(options.sort ?? "name");
  const first = list ? arrange({ profiles: list.profiles, groups: list.groups, query: "", sort }).groups.flatMap((g) => g.profiles)[0]?.id ?? null : null;
  const [selectedId, select] = useState<string | null>(first);
  const selected = list?.profiles.find((p) => p.id === selectedId) ?? null;
  const rubrica: Rubrica = {
    loaded: list !== null || Boolean(options.error),
    view: list,
    error: options.error ?? null,
    selectedId,
    select,
    detail: selected ? makeDetail(selected) : null,
    detailError: null,
    reload: async () => undefined,
  };
  return (
    <Shell view={view} info={view ? makeInfo() : null} onRetry={options.onRetry ?? (() => undefined)} announce={options.announce} rubrica={rubrica} sort={sort} onSortChange={setSort} />
  );
}

export const renderShell = (view: DaemonView | null = makeView(), options: Options = {}) =>
  render(
    <Preferences lang={options.lang ?? "en"}>
      <Harness view={view} options={options} />
    </Preferences>,
  );
