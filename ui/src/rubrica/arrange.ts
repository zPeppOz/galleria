import type { ProfileSummary } from "./types";

export interface ArrangedGroup { title: string | null; profiles: ProfileSummary[] }
export interface Arranged { groups: ArrangedGroup[]; total: number }
export type SortKey = "name" | "recent";

/** Lower case with the accents taken off, so that «citta» finds «Città». */
const fold = (text: string): string => text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();

const lastUse = (profile: ProfileSummary): number => (profile.last_used === null ? Number.NaN : Date.parse(profile.last_used));

/**
 * The profiles as the list shows them. Pure: it imports nothing of React or Tauri.
 *
 * - Groups: in the order of `galleria.toml`; then the groups that only the profiles name, alphabetically; at the end the
 *   profiles without a group (`title: null`). A group of the list with no profile shows only while the search is empty.
 * - Inside a group the favourites come first (P5), in both orders; then by name (`name`), or by the latest use and then by
 *   name (`recent`, P6: a profile never used, or with a date that cannot be read, comes after the used ones). Names are
 *   compared with the collator of `locale`; a tie goes by id, so the order never depends on the order the profiles came in.
 * - The search looks at the name, the server and the group, ignores case and accents, and every word must be found.
 */
export function arrange(input: { profiles: ProfileSummary[]; groups: string[]; query: string; sort: SortKey; locale?: string }): Arranged {
  const { profiles, groups, query, sort, locale } = input;
  const collator = new Intl.Collator(locale);
  const words = fold(query).split(/\s+/).filter(Boolean);
  const found = words.length === 0 ? profiles : profiles.filter((p) => {
    const text = fold(`${p.name} ${p.server ?? ""} ${p.group ?? ""}`);
    return words.every((word) => text.includes(word));
  });

  const byName = (a: ProfileSummary, b: ProfileSummary) => collator.compare(a.name, b.name) || collator.compare(a.id, b.id);
  const byUse = (a: ProfileSummary, b: ProfileSummary) => {
    const [ta, tb] = [lastUse(a), lastUse(b)];
    if (Number.isNaN(ta) || Number.isNaN(tb)) return Number.isNaN(ta) === Number.isNaN(tb) ? 0 : Number.isNaN(ta) ? 1 : -1;
    return tb - ta;
  };
  const order = (a: ProfileSummary, b: ProfileSummary) => Number(b.favorite) - Number(a.favorite) || (sort === "recent" ? byUse(a, b) : 0) || byName(a, b);

  const named = new Map<string, ProfileSummary[]>(groups.map((title) => [title, []]));
  const loose: ProfileSummary[] = [];
  const unknown = new Set<string>();
  for (const profile of found) {
    if (profile.group === null) {
      loose.push(profile);
      continue;
    }
    if (!named.has(profile.group)) {
      named.set(profile.group, []);
      unknown.add(profile.group);
    }
    named.get(profile.group)?.push(profile);
  }
  const titles = [...groups.filter((g, i) => groups.indexOf(g) === i), ...[...unknown].sort((a, b) => collator.compare(a, b))];
  const result: ArrangedGroup[] = titles
    .map((title) => ({ title, profiles: [...(named.get(title) ?? [])].sort(order) }))
    // an empty group is a place to put things in: it shows when nobody is looking for something
    .filter((group) => group.profiles.length > 0 || words.length === 0);
  if (loose.length > 0) result.push({ title: null, profiles: [...loose].sort(order) });
  return { groups: result, total: found.length };
}
