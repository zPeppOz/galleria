import { invoke } from "@tauri-apps/api/core";
import { createContext, useContext, useEffect, useState } from "react";

/** Where a secret of this window is written: the keyring of the system, or a file only the user can read (spec §9). */
export type SecretStoreKind = "keyring" | "file";

/**
 * Asks Rust where the secrets go: when the window opens, and again when it comes back to the front (a keyring that was
 * started or stopped meanwhile changes the answer). The answer is a word and nothing else: Rust only asks the session bus
 * whether the keyring is there, and the command opens no collection and never unlocks (plan B5, T3). `null` until it
 * answers, and after a failure: the window then says nothing about it.
 */
export function useSecretStore(): SecretStoreKind | null {
  const [store, setStore] = useState<SecretStoreKind | null>(null);
  useEffect(() => {
    let closed = false;
    const read = () => {
      invoke<{ store: SecretStoreKind }>("secret_store_status").then(
        (answer) => {
          // Anything but one of the two words is «not known»: the window then says nothing about the keyring.
          if (!closed && (answer?.store === "keyring" || answer?.store === "file")) setStore(answer.store);
        },
        () => undefined,
      );
    };
    read();
    const visible = () => {
      if (document.visibilityState === "visible") read();
    };
    window.addEventListener("focus", read);
    document.addEventListener("visibilitychange", visible);
    return () => {
      closed = true;
      window.removeEventListener("focus", read);
      document.removeEventListener("visibilitychange", visible);
    };
  }, []);
  return store;
}

/** What the window knows of where the secrets go, for the texts that name the place (the shell provides it; `null`: not known, and the texts are the general ones). */
export const SecretStoreContext = createContext<SecretStoreKind | null>(null);

export const useSecretStoreKind = (): SecretStoreKind | null => useContext(SecretStoreContext);
