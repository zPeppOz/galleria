import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { I18n } from "../i18n";
import type { Rubrica } from "../rubrica/store";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeRubricaView } from "../test/fixtures";
import type { SecretStoreKind } from "../secrets/store";
import { RubricaNotices } from "./RubricaNotices";

const rubrica = (over: Partial<Rubrica> = {}): Rubrica => ({
  loaded: true,
  view: makeRubricaView(),
  error: null,
  selectedId: null,
  select: () => undefined,
  detail: null,
  detailError: null,
  reload: async () => undefined,
  update: async () => {
    throw new Error("not used");
  },
  validateRaw: async () => ({ issues: [], can_save: true }),
  actions: { duplicate: async () => Promise.reject(new Error("not used")), export: async () => false, remove: async () => undefined },
  groups: { create: async () => [], rename: async () => [], remove: async () => [], reorder: async () => [] },
  ...over,
});

const show = (store: SecretStoreKind | null | undefined, lang: "en" | "it" = "en") =>
  render(
    <I18n lang={lang}>
      <main>
        <RubricaNotices rubrica={rubrica()} onImport={() => undefined} secretStore={store} />
      </main>
    </I18n>,
  );

describe("RubricaNotices, the archive of the secrets", () => {
  it("the_file_fallback_notice_stays_and_cannot_be_dismissed", async () => {
    const { rerender } = show("file");
    const notice = screen.getByText("No system keyring: the keys are saved in a file only you can read.");
    // a notice of the page, polite (not an alert), with a name for its icon, and nothing to close it
    const box = notice.closest("[role]");
    expect(box?.getAttribute("role")).toBe("status");
    expect(screen.getByRole("img", { name: "Warning" })).toBeTruthy();
    expect(screen.queryByRole("button")).toBeNull();
    // it is still there when the rubrica is read again (the shell draws it again with the same answer)
    rerender(
      <I18n lang="en">
        <main>
          <RubricaNotices rubrica={rubrica({ view: makeRubricaView({ groups: ["Lavoro"] }) })} onImport={() => undefined} secretStore="file" />
        </main>
      </I18n>,
    );
    expect(screen.getByText("No system keyring: the keys are saved in a file only you can read.")).toBeTruthy();
    await expectNoAccessibilityViolations(document.body);
  });

  it("no_notice_with_a_keyring", () => {
    show("keyring");
    expect(document.body.textContent).not.toMatch(/keyring|portachiavi/i);
  });

  it("no_notice_until_the_answer_is_known", () => {
    show(null);
    expect(document.body.textContent).not.toMatch(/keyring|portachiavi/i);
    document.body.innerHTML = "";
    show(undefined);
    expect(document.body.textContent).not.toMatch(/keyring|portachiavi/i);
  });

  it("the_notice_is_in_italian_too", () => {
    show("file", "it");
    expect(screen.getByText("Nessun portachiavi di sistema: le chiavi sono salvate in un file leggibile solo da te.")).toBeTruthy();
  });
});
