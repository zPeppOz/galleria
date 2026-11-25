import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { I18n } from "../i18n";
import { makeInfo, makeView } from "../test/fixtures";
import { FirstRun } from "./FirstRun";

const show = (view = makeView({ link: "daemon_not_running", status: null }), info = makeInfo(), onRetry = vi.fn()) =>
  render(
    <I18n lang="it">
      <FirstRun view={view} info={info} onRetry={onRetry} />
    </I18n>,
  );

describe("FirstRun", () => {
  it("daemon_not_running_shows_the_systemctl_command", () => {
    show();
    expect(screen.getByText("sudo systemctl enable --now galleriad")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Avvia il servizio" })).toBeTruthy();
  });

  it("permission_denied_shows_usermod_with_the_user_name", () => {
    show(makeView({ link: "permission_denied", status: null }), makeInfo({ user: "mrossi" }));
    expect(screen.getByText("sudo usermod -aG galleria mrossi")).toBeTruthy();
    expect(screen.getByText(/esci e rientra nella sessione/)).toBeTruthy();
  });

  it("incompatible_protocol_shows_both_versions", () => {
    show(
      makeView({ link: "incompatible_protocol", status: null, incompatible: { daemon_min: 2, daemon_max: 3, daemon_version: "0.2.0" } }),
      makeInfo({ version: "0.1.0" }),
    );
    expect(screen.getByText(/versione 0\.1\.0/)).toBeTruthy();
    expect(screen.getByText(/versione 0\.2\.0/)).toBeTruthy();
  });

  it("copy_puts_the_command_on_the_clipboard", async () => {
    const user = userEvent.setup();
    const write = vi.spyOn(navigator.clipboard, "writeText");
    show();
    await user.click(screen.getByRole("button", { name: "Copia" }));
    expect(write).toHaveBeenCalledWith("sudo systemctl enable --now galleriad");
  });

  it("retry_calls_the_handler", async () => {
    const user = userEvent.setup();
    const onRetry = vi.fn();
    show(undefined, undefined, onRetry);
    await user.click(screen.getByRole("button", { name: "Riprova" }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});
