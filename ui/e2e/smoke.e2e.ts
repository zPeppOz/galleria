// WebDriver smoke test (level 3 of the spec §14): the real app in a real WebKitGTK window against a daemon that serves the real IPC (the `fake-daemon`
// example: the real `galleriad` server with a fake backend, S5). Run by lab/tests/gui-smoke.sh, which starts tauri-driver, Xvfb and the daemon and exports:
//   GALLERIA_SMOKE_APP     the built galleria-gui binary
//   GALLERIA_SMOKE_BIN     a directory with `daemon-start` and `daemon-stop` (the tests run them from Node, not
//                          from the webview)
//   GALLERIA_SMOKE_DRIVER_PORT  the port tauri-driver listens on
// Nothing here asks the daemon to connect: no route, DNS or interface is touched.
import { execFileSync } from "node:child_process";
import { afterEach, describe, expect, it } from "vitest";
import { closeApp, daemon, openApp, TEN_SECONDS, waitForBody } from "./support";

const CONNECTED = /Connected to the service|Collegato al servizio/;
const DISCONNECTED = /Disconnected|Disconnesso/;
const START_THE_SERVICE = /Start the service|Avvia il servizio/;
const SYSTEMCTL = "sudo systemctl enable --now galleriad";

afterEach(closeApp);

describe("galleria-gui against the real daemon", () => {
  it("shows_the_real_daemon_state", async () => {
    daemon("start");
    const b = await openApp();
    await waitForBody(b, DISCONNECTED, "the window never showed the daemon state «Disconnected»");
    await waitForBody(b, CONNECTED, "the bar never showed the active link");
  });

  it("relinks_after_a_daemon_restart", async () => {
    daemon("start");
    const b = await openApp();
    await waitForBody(b, CONNECTED, "no link at the start");
    daemon("stop");
    await waitForBody(b, "systemctl", "the window never showed the command to start the service");
    daemon("start");
    await waitForBody(b, DISCONNECTED, "the window did not relink by itself after the daemon came back");
    await waitForBody(b, CONNECTED, "the bar did not show the link after the restart");
  });

  it("first_run_without_daemon", async () => {
    daemon("stop");
    const b = await openApp();
    await waitForBody(b, START_THE_SERVICE, "the «Start the service» panel never appeared");
    await waitForBody(b, SYSTEMCTL, "the panel does not show the command");
  });

  it("window_has_no_remote_content", async () => {
    daemon("stop");
    const b = await openApp();
    await waitForBody(b, START_THE_SERVICE, "the window never rendered");
    const remoteRefs = await b.execute(
      () => document.querySelectorAll('script[src^="http"],link[href^="http"],img[src^="http"]').length,
    );
    expect(remoteRefs).toBe(0);
  });

  // S7: «Copy» must fill the system clipboard from the real WebKitGTK webview. Read back with xclip under Xvfb.
  it("copy_fills_the_system_clipboard", async () => {
    daemon("stop");
    const b = await openApp();
    await waitForBody(b, START_THE_SERVICE, "the window never rendered");
    await b.$("button=Copy").click().catch(async () => b.$("button=Copia").click());
    await b.waitUntil(
      async () => {
        try {
          return execFileSync("xclip", ["-o", "-selection", "clipboard"], { encoding: "utf8" }) === SYSTEMCTL;
        } catch {
          return false;
        }
      },
      { timeout: TEN_SECONDS, interval: 250, timeoutMsg: "the clipboard did not receive the command" },
    );
  });
});
