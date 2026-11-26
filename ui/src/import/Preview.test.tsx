import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { I18n } from "../i18n";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makePreview } from "../test/fixtures";
import { Preview } from "./Preview";
import type { ImportNotice } from "./types";

const notice = (over: Partial<ImportNotice> = {}): ImportNotice => ({ line: 7, directive: "up", kind: "dangerous", dns_script: false, ...over });

function show(over: Parameters<typeof makePreview>[0] = {}, lang: "en" | "it" = "en") {
  return render(
    <I18n lang={lang}>
      <main>
        <Preview preview={makePreview(over)} />
      </main>
    </I18n>,
  );
}

const items = () => within(screen.getByRole("list")).getAllByRole("listitem").map((li) => li.textContent);

describe("Preview", () => {
  it("script_directive_shows_the_d19_text", () => {
    show({ notices: [notice()], removed: 1 });
    // the two sentences of D19, in one line with the line and the name
    expect(items()).toEqual(["Line 7 · Directive up removed: Galleria does not run the scripts of profiles. If the profile depends on it, it may not work fully."]);
    // and in Italian
    document.body.innerHTML = "";
    show({ notices: [notice()], removed: 1 }, "it");
    expect(items()).toEqual(["Riga 7 · Direttiva up rimossa: Galleria non esegue gli script dei profili. Se il profilo ne dipende, potrebbe non funzionare del tutto."]);
  });

  it("resolver_script_says_galleria_applies_the_dns", () => {
    show({ notices: [notice({ directive: "down", line: 9, dns_script: true })], removed: 1 });
    const [text] = items();
    expect(text).toBe("Line 9 · Directive down removed: Galleria does not run the scripts of profiles. Galleria applies the DNS of the server itself.");
    expect(text).not.toContain("may not work fully");
  });

  it("other_dangerous_and_unsupported_directives_do_not_say_script", () => {
    show({ notices: [notice({ directive: "management", line: 3 }), notice({ directive: "invented-directive", line: 4, kind: "unsupported" })], removed: 2 });
    expect(items()).toEqual(["Line 3 · Directive management removed: Galleria does not allow it in profiles.", "Line 4 · Directive invented-directive removed: Galleria does not support it."]);
    expect(document.body.textContent).not.toContain("scripts");
  });

  it("a_notice_shows_line_and_name_only", () => {
    show({ notices: [notice({ directive: "plugin", line: 12 })], removed: 1 });
    const li = screen.getByRole("listitem");
    expect(within(li).getByText("plugin").tagName).toBe("CODE");
    expect(li.textContent).toMatch(/^Line 12 · Directive plugin removed:/);
    // nothing of an argument: the view has none to show
    expect(document.body.textContent).not.toMatch(/\/tmp|\/usr/);
  });

  it("the_preview_says_where_the_keys_will_go", () => {
    const found = ["openvpn.block.key", "openvpn.block.tls-crypt"];
    const say = (store: "keyring" | "file" | null, lang: "en" | "it" = "en") => {
      document.body.innerHTML = "";
      render(
        <I18n lang={lang}>
          <main>
            <Preview preview={makePreview({ found })} store={store} />
          </main>
        </I18n>,
      );
      return document.body.textContent ?? "";
    };
    expect(say("keyring")).toContain("Found: private key, TLS encryption key. They will be saved in the system keyring.");
    expect(say("file")).toContain("Found: private key, TLS encryption key. They will be saved in a file only you can read.");
    expect(say("keyring", "it")).toContain("Saranno salvati nel portachiavi di sistema.");
    expect(say("file", "it")).toContain("Saranno salvati in un file leggibile solo da te.");
    // not known yet: the general sentence, which promises nothing about where
    const unknown = say(null);
    expect(unknown).toContain("They will be saved in the secrets archive.");
    expect(unknown).toContain("The credentials will be asked at the first connection.");
  });

  it("a_profile_without_secrets_says_so", () => {
    show({ found: [], auth: { certificate: false, user_pass: false, otp: false } });
    expect(document.body.textContent).toContain("The profile has no keys or secrets in it.");
    expect(document.body.textContent).not.toContain("credentials will be asked");
  });

  it("missing_client_is_a_warning_and_is_not_counted", () => {
    show({ notices: [notice({ directive: "tls-client", line: 0, kind: "no_tls_client" })], removed: 0 });
    expect(items()).toEqual(["The “client” line is missing: the profile may not be a client profile."]);
    expect(document.body.textContent).not.toContain("removed");
  });

  it("wireguard_hook_shows_the_d19_text", () => {
    show({ protocol: "wireguard", found: ["wireguard.private_key", "wireguard.peer.1.preshared_key"], notices: [notice({ directive: "PostUp", line: 5 })], removed: 1, file_name: "office.conf", server: "office.example:51820" });
    expect(items()).toEqual(["Line 5 · Directive PostUp removed: Galleria does not run the scripts of profiles. If the profile depends on it, it may not work fully."]);
    const text = document.body.textContent ?? "";
    expect(text).toContain("File: office.conf");
    expect(text).toContain("private key, shared key of peer 2");
    expect(screen.getByText("WireGuard")).toBeTruthy();
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = show({ notices: [notice(), notice({ directive: "status", line: 8 })], removed: 2, file_name: "inline.ovpn" });
    await expectNoAccessibilityViolations(container, { landmarks: false });
  });
});
