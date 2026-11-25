/**
 * The directives that run a script or a plugin (D19): the ones that get the sentence about scripts, in the editor of the
 * raw text and in the preview of an import. The other directives that the check calls dangerous (`log`, `user`,
 * `auth-user-pass file`…) get a text of their own, because they do not run one. `SCRIPT_DIRECTIVES` of
 * `crates/galleria-gui/src/import.rs` is the same list, and a test of that crate keeps the two equal.
 */
export const SCRIPT_DIRECTIVES: ReadonlySet<string> = new Set([
  "up",
  "down",
  "route-up",
  "route-pre-down",
  "ipchange",
  "tls-verify",
  "client-connect",
  "client-disconnect",
  "learn-address",
  "auth-user-pass-verify",
  "plugin",
  "up-restart",
  "dns-updown",
]);

/** The keys of the `[Interface]` of a WireGuard profile that run a command. */
export const WIREGUARD_HOOKS: ReadonlySet<string> = new Set(["PreUp", "PostUp", "PreDown", "PostDown"]);

/** Whether a directive (or a key of a WireGuard profile) runs a script or a command of the profile. */
export const runsAScript = (directive: string): boolean => SCRIPT_DIRECTIVES.has(directive) || WIREGUARD_HOOKS.has(directive);
