import type { AppInfo } from "./types";

// Commands the first-run panels show for the user to copy. The app never runs them (D27).
export const START_SERVICE = "sudo systemctl enable --now galleriad";
export const joinGroup = (info: AppInfo): string => `sudo usermod -aG ${info.group} ${info.user}`;

// What the empty rubrica shows for the user to copy, until the import of the window arrives (B4).
export const IMPORT_PROFILE = "galleria import <file>";
