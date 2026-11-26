import { mockIPC } from "@tauri-apps/api/mocks";
import { describe, expect, it } from "vitest";
import { updateView } from "./commands";

describe("the command of the version check (plan B6, T4)", () => {
  it("update_view_is_one_call_with_no_argument_and_gives_the_view_as_it_is", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    const view = { current: "0.1.0", latest: "9.9.9", url: "https://github.com/zPeppOz/galleria/releases/tag/v9.9.9", enabled: true };
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return view;
    });
    expect(await updateView()).toEqual(view);
    // nothing travels to Rust: the page cannot ask for a request, only for what is known
    expect(calls).toEqual([{ cmd: "update_view", args: {} }]);
  });
});
