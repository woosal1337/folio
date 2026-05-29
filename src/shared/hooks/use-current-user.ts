/**
 * Current-user accessors — the single source of truth for "who is this"
 * across the app. Identity is owned by `auth-store` (mirrored from the
 * Keychain-cached `UserIdentity`, kept in sync with the backend on
 * profile save), so anything that needs the user's name — task owner
 * defaults, "Me" labels, agent attribution — reads it from here rather
 * than re-deriving or hardcoding it.
 */

import { useAuthStore } from "@/shared/stores/auth-store";

/** Reactive: the signed-in user's display name, or "" if unset. */
export function useDisplayName(): string {
  return useAuthStore((s) => s.identity?.display_name ?? "");
}

/**
 * Non-reactive read for use outside React render (event handlers,
 * stores, one-shot defaults). Returns "" when signed out or unnamed.
 */
export function currentDisplayName(): string {
  return useAuthStore.getState().identity?.display_name ?? "";
}
