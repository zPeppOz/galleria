import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, LogEntry } from "./types";

// Commands the first-run panels show for the user to copy. The app never runs them (D27).
export const START_SERVICE = "sudo systemctl enable --now galleriad";
export const joinGroup = (info: AppInfo): string => `sudo usermod -aG ${info.group} ${info.user}`;

/** The error a command answers with: a stable `code` (translated by the catalogues), the English `message`, and what the code carries. */
export interface CommandError { code: string; message: string; data?: Record<string, unknown> }

/**
 * Connects a profile of the rubrica. Only an id goes: the keys and the parameters are made in Rust (spec §3.2). The state of the
 * session is not in the answer, it comes with `galleria:state`; the answer is only for the error of this call, among them
 * `already_active` with `data.foreign` and `data.profile`.
 */
export const connectProfile = (id: string, replace: boolean): Promise<void> => invoke<void>("connect_profile", { id, replace });

/** Ends the session of the service. */
export const disconnectSession = (): Promise<void> => invoke<void>("disconnect");

/** What the user typed for a question of the service: only the fields of that question are set. */
export interface InputAnswer { username?: string; password?: string; otp?: string; key_passphrase?: string }

/**
 * Gives the service what was typed (plan B5, «Il viaggio di una credenziale»). It is the only place of the page that invokes
 * `provide_input`, and the only caller is the connection: the values go in this one call, nothing keeps them, and the answer is empty.
 * `remember` asks Rust to keep the user and the password once the session is up; for a code or a passphrase it does nothing.
 */
export const provideInput = (session: number, request: number, values: InputAnswer, remember: boolean): Promise<void> =>
  invoke<void>("provide_input", { session, request, values, remember });

/** Takes the remembered user and password of a profile out of the keyring (P6). */
export const forgetCredentials = (id: string): Promise<void> => invoke<void>("forget_credentials", { id });

/** A page of the log of the service, oldest first: the lines after `after`, at most `limit` (1 to 1000). */
export const logsGet = (after: number | null, limit: number): Promise<{ entries: LogEntry[]; last: number }> => invoke("logs_get", { after, limit });

/**
 * «Export log…»: Rust reads the whole log again, opens the window «Save» of the system and writes the file. The page gives no text and
 * no place and is not told where it went; `saved` is `false` when the person cancelled.
 */
export const logsExport = (): Promise<{ saved: boolean }> => invoke("logs_export");
