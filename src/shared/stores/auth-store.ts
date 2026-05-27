/**
 * Auth state — mirror of the Keychain-backed session on the Rust side.
 *
 * On app boot we call `authStatus()` exactly once and cache the result.
 * Subsequent sign-in / sign-out events update the store synchronously;
 * the recording route + onboarding conductor read from here to decide
 * whether to render the app or the login screen.
 *
 * `hydrated` distinguishes "we haven't checked yet" (show a splash) from
 * "we checked and the user is signed out" (show the login screen). The
 * recording route uses this to avoid a flash of the login screen on
 * cold boot.
 */

import { create } from "zustand";

import { authStatus } from "@/shared/lib/ipc";
import type { UserIdentity } from "@/shared/types/UserIdentity";

interface AuthState {
  hydrated: boolean;
  signedIn: boolean;
  identity: UserIdentity | null;

  hydrate: () => Promise<void>;
  setSignedIn: (identity: UserIdentity) => void;
  clear: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  hydrated: false,
  signedIn: false,
  identity: null,

  hydrate: async () => {
    try {
      const status = await authStatus();
      set({
        hydrated: true,
        signedIn: status.signed_in,
        identity: status.identity,
      });
    } catch (e) {
      console.error("auth hydrate:", e);
      set({ hydrated: true, signedIn: false, identity: null });
    }
  },

  setSignedIn: (identity) => set({ signedIn: true, identity, hydrated: true }),
  clear: () => set({ signedIn: false, identity: null }),
}));
