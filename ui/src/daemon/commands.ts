import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "./types";

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
