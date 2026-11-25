import type { ProfileSummary } from "./types";

export interface ArrangedGroup { title: string | null; profiles: ProfileSummary[] }
export interface Arranged { groups: ArrangedGroup[]; total: number }

/**
 * The profiles in the groups of the list. Pure: it imports nothing of React or Tauri. The groups come in the order of
 * `galleria.toml`; then the groups that only the profiles name, alphabetically; at the end the profiles without a group
 * (`title: null`). Inside a group the profiles keep the order they came in.
 */
export function arrange(input: { profiles: ProfileSummary[]; groups: string[] }): Arranged {
  const { profiles, groups } = input;
  const named = new Map<string, ProfileSummary[]>(groups.map((title) => [title, []]));
  const loose: ProfileSummary[] = [];
  const unknown = new Set<string>();
  for (const profile of profiles) {
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
  const titles = [...groups.filter((g, i) => groups.indexOf(g) === i), ...[...unknown].sort((a, b) => a.localeCompare(b))];
  const result: ArrangedGroup[] = titles.map((title) => ({ title, profiles: named.get(title) ?? [] }));
  if (loose.length > 0) result.push({ title: null, profiles: loose });
  return { groups: result, total: profiles.length };
}
