import { mockIPC } from "@tauri-apps/api/mocks";
import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { makeSummary } from "../test/fixtures";
import { useImportActions } from "./store";
import type { ImportPreview } from "./types";

const preview: ImportPreview = {
  id: "i1", origin: "text", file_name: null, suggested_name: "", protocol: "openvpn", server: "vpn.example:1194",
  auth: { certificate: false, user_pass: false, otp: false }, found: ["openvpn.block.key"], notices: [], removed: 0, groups: [], expires_in_secs: 600,
};

describe("useImportActions", () => {
  it("sends_the_text_and_answers_with_the_preview", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return preview;
    });
    const { result } = renderHook(() => useImportActions(async () => undefined));
    expect(await result.current.fromText("client\n")).toEqual(preview);
    expect(calls).toEqual([{ cmd: "import_from_text", args: { text: "client\n" } }]);
  });

  it("pick_file_sends_nothing_and_answers_with_the_preview_or_null", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    let answer: ImportPreview | null = preview;
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return answer;
    });
    const { result } = renderHook(() => useImportActions(async () => undefined));
    expect(await result.current.pickFile()).toEqual(preview);
    answer = null;
    expect(await result.current.pickFile()).toBeNull();
    // no path goes out: the window is Rust's
    expect(calls.map((c) => c.cmd)).toEqual(["import_pick_file", "import_pick_file"]);
    expect(calls.every((c) => c.args === undefined || Object.keys(c.args as object).length === 0)).toBe(true);
  });

  it("commit_reads_the_rubrica_again_before_it_answers", async () => {
    const order: string[] = [];
    mockIPC((cmd) => {
      order.push(cmd);
      return makeSummary({ id: "new", name: "Nuovo" });
    });
    const { result } = renderHook(() =>
      useImportActions(async () => {
        order.push("reload");
      }),
    );
    expect((await result.current.commit("i1", "Nuovo", "Lavoro")).name).toBe("Nuovo");
    expect(order).toEqual(["import_commit", "reload"]);
  });

  it("a_refusal_is_thrown_as_the_error_of_the_command_and_the_list_is_not_read", async () => {
    let reloaded = false;
    mockIPC(() => Promise.reject({ code: "gui_invalid_field", message: "Some fields are not valid", data: { fields: [{ field: "name", code: "required" }] } }));
    const { result } = renderHook(() =>
      useImportActions(async () => {
        reloaded = true;
      }),
    );
    await expect(result.current.commit("i1", "", null)).rejects.toMatchObject({ code: "gui_invalid_field" });
    await expect(result.current.fromText("x")).rejects.toMatchObject({ code: "gui_invalid_field" });
    await expect(result.current.pickFile()).rejects.toMatchObject({ code: "gui_invalid_field" });
    expect(reloaded).toBe(false);
  });

  it("discard_never_fails", async () => {
    mockIPC(() => Promise.reject({ code: "gui_import_expired", message: "gone" }));
    const { result } = renderHook(() => useImportActions(async () => undefined));
    await expect(result.current.discard("i1")).resolves.toBeUndefined();
  });
});
