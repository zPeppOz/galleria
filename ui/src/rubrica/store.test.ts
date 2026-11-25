import { mockIPC } from "@tauri-apps/api/mocks";
import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { makeDetail, makeRubricaView, makeSummary, SAMPLE_PROFILES } from "../test/fixtures";
import type { RubricaView } from "./types";
import { useRubrica } from "./store";

/** A backend that answers from `state.view`, which a test may replace, and counts the calls. */
function backend(view: RubricaView) {
  const state = { view, calls: [] as string[], failList: null as unknown, failDetail: null as unknown };
  mockIPC((cmd, args) => {
    state.calls.push(cmd);
    if (cmd === "profiles_list") {
      if (state.failList) return Promise.reject(state.failList);
      return state.view;
    }
    if (cmd === "profile_get") {
      if (state.failDetail) return Promise.reject(state.failDetail);
      const found = state.view.profiles.find((p) => p.id === (args as { id: string }).id);
      return found ? makeDetail(found) : Promise.reject({ code: "gui_profile_not_found", message: "Profile not found" });
    }
    return undefined;
  });
  return state;
}

const count = (state: { calls: string[] }, cmd: string) => state.calls.filter((c) => c === cmd).length;

describe("useRubrica", () => {
  it("loads_on_start_and_on_window_focus", async () => {
    const state = backend(makeRubricaView());
    const { result } = renderHook(() => useRubrica());
    expect(result.current.loaded).toBe(false);
    await waitFor(() => expect(result.current.loaded).toBe(true));
    expect(result.current.view?.profiles).toHaveLength(5);
    expect(count(state, "profiles_list")).toBe(1);

    // another window or the CLI changed the profiles: coming back to this window reads them again
    state.view = makeRubricaView({ profiles: [...SAMPLE_PROFILES, makeSummary({ id: "x", name: "Nuovo" })] });
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    await waitFor(() => expect(result.current.view?.profiles.map((p) => p.name)).toContain("Nuovo"));
    expect(count(state, "profiles_list")).toBe(2);
  });

  it("starts_on_the_first_profile_and_loads_its_detail", async () => {
    backend(makeRubricaView());
    const { result } = renderHook(() => useRubrica());
    await waitFor(() => expect(result.current.detail?.name).toBe("Ufficio Napoli"));
    expect(result.current.selectedId).toBe(result.current.view?.profiles[0].id);
  });

  it("keeps_the_selection_when_the_list_reloads", async () => {
    const state = backend(makeRubricaView());
    const { result } = renderHook(() => useRubrica());
    await waitFor(() => expect(result.current.loaded).toBe(true));
    const homelab = state.view.profiles.find((p) => p.name === "Homelab")!;
    act(() => result.current.select(homelab.id));
    await waitFor(() => expect(result.current.detail?.id).toBe(homelab.id));
    await act(async () => {
      await result.current.reload();
    });
    expect(result.current.selectedId).toBe(homelab.id);
    expect(result.current.detail?.id).toBe(homelab.id);
  });

  it("selection_moves_when_the_selected_profile_disappears", async () => {
    const state = backend(makeRubricaView());
    const { result } = renderHook(() => useRubrica());
    await waitFor(() => expect(result.current.loaded).toBe(true));
    const [, second, third] = state.view.profiles;
    act(() => result.current.select(second.id));
    await waitFor(() => expect(result.current.detail?.id).toBe(second.id));
    // the second one is deleted elsewhere: the one that takes its place is chosen, not the first
    state.view = { ...state.view, profiles: state.view.profiles.filter((p) => p.id !== second.id) };
    await act(async () => {
      await result.current.reload();
    });
    expect(result.current.selectedId).toBe(third.id);
    await waitFor(() => expect(result.current.detail?.id).toBe(third.id));
    // and with nothing left there is no selection and no detail
    state.view = { ...state.view, profiles: [] };
    await act(async () => {
      await result.current.reload();
    });
    expect(result.current.selectedId).toBeNull();
    await waitFor(() => expect(result.current.detail).toBeNull());
  });

  it("a_store_error_keeps_the_last_good_list_and_reports_the_code", async () => {
    const state = backend(makeRubricaView());
    const { result } = renderHook(() => useRubrica());
    await waitFor(() => expect(result.current.loaded).toBe(true));
    state.failList = { code: "gui_store_insecure", message: "x", data: { path: "/home/m/.config/galleria", mode: "755", fix: "chmod 700 '/home/m/.config/galleria'" } };
    await act(async () => {
      await result.current.reload();
    });
    expect(result.current.error?.code).toBe("gui_store_insecure");
    expect(result.current.view?.profiles).toHaveLength(5);
    // fixed in a terminal: the next read clears the error
    state.failList = null;
    await act(async () => {
      await result.current.reload();
    });
    expect(result.current.error).toBeNull();
  });

  it("a_first_read_that_fails_is_loaded_with_an_error_and_no_list", async () => {
    const state = backend(makeRubricaView());
    state.failList = { code: "gui_store_unreadable", message: "x" };
    const { result } = renderHook(() => useRubrica());
    await waitFor(() => expect(result.current.loaded).toBe(true));
    expect(result.current.view).toBeNull();
    expect(result.current.error?.code).toBe("gui_store_unreadable");
  });

  it("the_last_answer_wins_when_two_reads_overlap", async () => {
    const first = makeRubricaView({ profiles: [makeSummary({ id: "a", name: "Old" })] });
    const second = makeRubricaView({ profiles: [makeSummary({ id: "b", name: "New" })] });
    const resolvers: ((v: RubricaView) => void)[] = [];
    mockIPC((cmd) => (cmd === "profiles_list" ? new Promise<RubricaView>((resolve) => resolvers.push(resolve)) : undefined));
    const { result } = renderHook(() => useRubrica());
    await waitFor(() => expect(resolvers).toHaveLength(1));
    act(() => {
      void result.current.reload();
    });
    await waitFor(() => expect(resolvers).toHaveLength(2));
    // the newer read answers first, the older one arrives late and must not overwrite it
    await act(async () => {
      resolvers[1](second);
    });
    await act(async () => {
      resolvers[0](first);
    });
    expect(result.current.view?.profiles.map((p) => p.name)).toEqual(["New"]);
  });
});
