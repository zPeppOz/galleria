import type { Protocol } from "../daemon/types";
import type { AuthFlags, ProfileSummary } from "../rubrica/types";

// Hand-written mirror of the views of `crates/galleria-gui/src/import.rs` (snake_case, as serialised). None of them has a
// secret, a path or the text of the profile: `found` holds the names of the secrets that were found, never their values.

/** Where the file or the text came from. */
export type ImportOrigin = "picker" | "drop" | "open" | "text";

export type ImportNoticeKind = "dangerous" | "unsupported" | "no_tls_client";

/**
 * One thing the import dropped or noticed: the line and the name of the directive, never its arguments. `dns_script` is
 * true when the directive ran one of the two scripts that set the DNS of the server, which Galleria does by itself.
 */
export interface ImportNotice { line: number; directive: string; kind: ImportNoticeKind; dns_script: boolean }

export interface ImportPreview {
  id: string;
  origin: ImportOrigin;
  /** The name of the file, never the folder; null for pasted text. */
  file_name: string | null;
  /** The name proposed for the profile; empty for pasted text (the page proposes its own). */
  suggested_name: string;
  protocol: Protocol;
  server: string | null;
  auth: AuthFlags;
  /** The names of the secrets found (`openvpn.block.key`…). */
  found: string[];
  notices: ImportNotice[];
  /** How many directives were dropped: the dangerous and the unsupported ones; a missing `client` is not one. */
  removed: number;
  groups: string[];
  expires_in_secs: number;
}

/**
 * What the window can do with an import (`import_from_text`, `import_pick_file`, `import_commit`, `import_discard`). Each one throws the
 * error of the command. Nothing is written before `commit`; a `commit` that worked reads the rubrica again before it
 * answers, so whoever asked finds the new profile in the list.
 */
export interface ImportActions {
  /** The preview of the text the user pasted. */
  fromText: (text: string) => Promise<ImportPreview>;
  /** The preview of the file the user chooses in the window «Open» that Rust opens (it takes no path: the page never has one); `null` when the window is cancelled. */
  pickFile: () => Promise<ImportPreview | null>;
  /** Saves the preview as a profile with this name, in this group (`null`: none); answers with the row of the new profile. */
  commit: (id: string, name: string, group: string | null) => Promise<ProfileSummary>;
  /** Lets the preview go; a preview that is not there any more is not an error, and neither is any other failure. */
  discard: (id: string) => Promise<void>;
}
