import { mockIPC } from "@tauri-apps/api/mocks";
import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useSecretStore } from "./store";

function backend(first: unknown) {
  const state = { answer: first, calls: 0 };
  mockIPC((cmd) => {
    if (cmd !== "secret_store_status") return undefined;
    state.calls += 1;
    return state.answer instanceof Error ? Promise.reject(state.answer) : state.answer;
  });
  return state;
}

describe("useSecretStore", () => {
  it("asks_at_start_and_again_when_the_window_comes_back", async () => {
    const state = backend({ store: "file" });
    const { result } = renderHook(() => useSecretStore());
    expect(result.current).toBeNull();
    await waitFor(() => expect(result.current).toBe("file"));
    expect(state.calls).toBe(1);
    // a keyring that was started meanwhile
    state.answer = { store: "keyring" };
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    await waitFor(() => expect(result.current).toBe("keyring"));
    expect(state.calls).toBe(2);
  });

  it("an_answer_that_is_not_one_of_the_two_words_is_not_known", async () => {
    const state = backend({ store: "something else" });
    const { result } = renderHook(() => useSecretStore());
    await waitFor(() => expect(state.calls).toBe(1));
    expect(result.current).toBeNull();
  });

  it("a_failure_leaves_it_not_known", async () => {
    const state = backend(new Error("no command"));
    const { result } = renderHook(() => useSecretStore());
    await waitFor(() => expect(state.calls).toBe(1));
    expect(result.current).toBeNull();
  });
});
