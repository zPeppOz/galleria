import { mockIPC } from "@tauri-apps/api/mocks";
import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import { ConnectionProvider } from "../connection/ConnectionProvider";
import type { BackendInfo, DaemonView } from "../daemon/types";
import { ToastProvider } from "../design/Toast";
import { I18n, type Lang } from "../i18n";
import { makeView } from "./fixtures";

export interface Call { cmd: string; args: Record<string, unknown> }

/** The commands the page called, and what the service answers to each: by default nothing, which is a success. */
export function backend(answer: (call: Call) => unknown = () => undefined): Call[] {
  const calls: Call[] = [];
  mockIPC(
    (cmd, args) => {
      const call = { cmd, args: (args ?? {}) as Record<string, unknown> };
      calls.push(call);
      return answer(call);
    },
    { shouldMockEvents: true },
  );
  return calls;
}

export const WIREGUARD_WITH_HANDSHAKE: BackendInfo = { protocol: "wireguard", available: true, version: null, unavailable_reason: null, capabilities: { handshake: true, interactive_auth: false, server_pushed_config: false } };
export const OPENVPN: BackendInfo = { protocol: "openvpn", available: true, version: "2.7.0", unavailable_reason: null, capabilities: { handshake: false, interactive_auth: true, server_pushed_config: true } };

/** A part of the window under the notices and the real connection, fed by the view the test gives (and changes with `rerender`). */
export function renderConnected(ui: ReactNode, view: DaemonView = makeView(), lang: Lang = "en") {
  const tree = (current: DaemonView) => (
    <I18n lang={lang}>
      <ToastProvider closeLabel="Dismiss the notice">
        <ConnectionProvider view={current}>
          <main>{ui}</main>
        </ConnectionProvider>
      </ToastProvider>
    </I18n>
  );
  const result = render(tree(view));
  return { ...result, show: (next: DaemonView) => result.rerender(tree(next)) };
}
