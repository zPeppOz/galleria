import type { AppInfo, DaemonView, Status } from "../daemon/types";

export function makeStatus(over: Partial<Status> = {}): Status {
  return {
    state: "disconnected",
    session: null,
    profile: null,
    protocol: null,
    since: null,
    attempt: 0,
    detail: null,
    local_addresses: [],
    remote: null,
    net: null,
    stats: null,
    input_request: null,
    error: null,
    ...over,
  };
}

export function makeView(over: Partial<DaemonView> = {}): DaemonView {
  return {
    link: "connected",
    daemon_version: "0.1.0",
    incompatible: null,
    backends: [],
    status: makeStatus(),
    ...over,
  };
}

export function makeInfo(over: Partial<AppInfo> = {}): AppInfo {
  return {
    version: "0.1.0",
    socket_path: "/run/galleria/galleriad.sock",
    user: "mrossi",
    group: "galleria",
    ...over,
  };
}
