import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { makeInfo, makeStatus, makeView } from "../test/fixtures";
import type { DaemonView } from "./types";
import { useDaemon } from "./store";

function mockBackend(view = makeView(), info = makeInfo()) {
  const calls: string[] = [];
  mockIPC(
    (cmd) => {
      calls.push(cmd);
      if (cmd === "daemon_view") return view;
      if (cmd === "app_info") return info;
      return undefined;
    },
    { shouldMockEvents: true },
  );
  return calls;
}

/** A backend whose `daemon_view` answers only after `release()`: the snapshot is on its way while events arrive. */
function slowSnapshot(view: DaemonView) {
  let release!: () => void;
  const gate = new Promise<void>((resolve) => (release = resolve));
  let askedNow!: () => void;
  const asked = new Promise<void>((resolve) => (askedNow = resolve));
  mockIPC(
    async (cmd) => {
      if (cmd === "daemon_view") {
        askedNow();
        await gate;
        return view;
      }
      if (cmd === "app_info") return makeInfo();
      return undefined;
    },
    { shouldMockEvents: true },
  );
  return { release, asked };
}

describe("useDaemon", () => {
  it("loads_the_view_and_app_info_on_start", async () => {
    mockBackend(makeView({ link: "permission_denied", status: null }), makeInfo({ user: "mrossi" }));
    const { result } = renderHook(() => useDaemon());
    await waitFor(() => expect(result.current.view?.link).toBe("permission_denied"));
    await waitFor(() => expect(result.current.info?.user).toBe("mrossi"));
  });

  it("follows_link_and_state_events", async () => {
    mockBackend();
    const { result } = renderHook(() => useDaemon());
    await waitFor(() => expect(result.current.view?.link).toBe("connected"));

    await act(async () => {
      await emit("galleria:state", makeStatus({ state: "connecting" }));
    });
    expect(result.current.view?.status?.state).toBe("connecting");

    await act(async () => {
      await emit("galleria:link", makeView({ link: "daemon_not_running", daemon_version: null, status: null }));
    });
    expect(result.current.view?.link).toBe("daemon_not_running");
  });

  it("retry_invokes_daemon_retry", async () => {
    const calls = mockBackend();
    const { result } = renderHook(() => useDaemon());
    await waitFor(() => expect(result.current.view).not.toBeNull());
    act(() => result.current.retry());
    await waitFor(() => expect(calls.filter((c) => c === "daemon_retry")).toHaveLength(1));
  });

  it("keeps_a_link_event_that_arrives_while_the_snapshot_is_on_its_way", async () => {
    const { release, asked } = slowSnapshot(makeView());
    const { result } = renderHook(() => useDaemon());
    await asked;
    // The daemon went away after the snapshot was taken but before its answer got here.
    await act(async () => {
      await emit("galleria:link", makeView({ link: "daemon_not_running", daemon_version: null, status: null }));
    });
    await act(async () => release());
    await waitFor(() => expect(result.current.info).not.toBeNull());
    expect(result.current.view?.link).toBe("daemon_not_running");
  });

  it("keeps_a_state_event_that_arrives_while_the_snapshot_is_on_its_way", async () => {
    const { release, asked } = slowSnapshot(makeView());
    const { result } = renderHook(() => useDaemon());
    await asked;
    await act(async () => {
      await emit("galleria:state", makeStatus({ state: "connecting" }));
    });
    await act(async () => release());
    await waitFor(() => expect(result.current.info).not.toBeNull());
    expect(result.current.view?.link).toBe("connected");
    expect(result.current.view?.status?.state).toBe("connecting");
  });

  it("stops_listening_when_unmounted", async () => {
    mockBackend();
    const { result, unmount } = renderHook(() => useDaemon());
    await waitFor(() => expect(result.current.view?.link).toBe("connected"));
    unmount();
    // Emitting after unmount must reach nobody; a leftover listener would update an unmounted hook and throw.
    await emit("galleria:link", makeView({ link: "daemon_not_running" }));
    expect(result.current.view?.link).toBe("connected");
  });
});
