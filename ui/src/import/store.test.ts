import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { makeSummary } from "../test/fixtures";
import { useImportActions, useIncoming } from "./store";
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

describe("useIncoming", () => {
  const failed = { failed: { error: { code: "gui_import_many", message: "x" }, file_name: null } };

  it("asks_for_a_pending_offer_at_start", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC(
      (cmd, args) => {
        calls.push({ cmd, args });
        return cmd === "import_pending" ? { preview: { ...preview, origin: "open", file_name: "inline.ovpn" } } : undefined;
      },
      { shouldMockEvents: true },
    );
    const { result } = renderHook(() => useIncoming());
    await waitFor(() => expect(result.current.incoming).not.toBeNull());
    expect(result.current.incoming?.offer).toEqual({ preview: { ...preview, origin: "open", file_name: "inline.ovpn" } });
    // asked once, with nothing: no path and no text go out
    expect(calls.map((c) => c.cmd)).toEqual(["import_pending"]);
    expect(Object.keys((calls[0].args ?? {}) as object)).toEqual([]);
  });

  it("nothing_waiting_is_no_offer", async () => {
    mockIPC(() => null, { shouldMockEvents: true });
    const { result } = renderHook(() => useIncoming());
    await waitFor(() => expect(result.current.hovering).toBe(false));
    expect(result.current.incoming).toBeNull();
  });

  it("an_open_file_event_opens_the_preview", async () => {
    const calls: string[] = [];
    mockIPC(
      (cmd) => {
        calls.push(cmd);
        return null;
      },
      { shouldMockEvents: true },
    );
    const { result } = renderHook(() => useIncoming());
    await waitFor(() => expect(calls).toEqual(["import_pending"]));
    await act(async () => {
      await emit("galleria:open-file", { preview: { ...preview, origin: "drop" } });
    });
    expect(result.current.incoming?.offer).toEqual({ preview: { ...preview, origin: "drop" } });
    // an offer that was heard does not wait any more: the page tells Rust, which lets go of it
    await waitFor(() => expect(calls).toEqual(["import_pending", "import_pending"]));
    // the same failure twice is two offers: the number changes
    const turns: number[] = [];
    for (let i = 0; i < 2; i++) {
      await act(async () => {
        await emit("galleria:open-file", failed);
      });
      turns.push(result.current.incoming?.turn ?? 0);
    }
    expect(turns[1]).toBeGreaterThan(turns[0]);
    expect(result.current.incoming?.offer).toEqual(failed);
  });

  it("an_offer_that_is_both_heard_and_pending_is_shown_once", async () => {
    // the event arrives while the answer of `import_pending` is on its way: the answer holds the same offer
    let release!: () => void;
    const gate = new Promise<void>((resolve) => (release = resolve));
    let asked!: () => void;
    const askedAt = new Promise<void>((resolve) => (asked = resolve));
    const offer = { preview: { ...preview, origin: "open" } };
    mockIPC(
      async (cmd) => {
        if (cmd !== "import_pending") return undefined;
        asked();
        await gate;
        return offer;
      },
      { shouldMockEvents: true },
    );
    const { result } = renderHook(() => useIncoming());
    await askedAt;
    await act(async () => {
      await emit("galleria:open-file", offer);
    });
    const turn = result.current.incoming?.turn;
    expect(turn).toBeGreaterThan(0);
    await act(async () => {
      release();
      await gate;
    });
    expect(result.current.incoming?.turn).toBe(turn);
  });

  it("the_hover_event_tells_a_file_is_over_the_window", async () => {
    mockIPC(() => null, { shouldMockEvents: true });
    const { result } = renderHook(() => useIncoming());
    await waitFor(() => expect(result.current.hovering).toBe(false));
    await act(async () => {
      await emit("galleria:drop-hover", { active: true });
    });
    expect(result.current.hovering).toBe(true);
    await act(async () => {
      await emit("galleria:drop-hover", { active: false });
    });
    expect(result.current.hovering).toBe(false);
  });
});
