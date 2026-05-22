/**
 * Settings cache backed by the Rust `SettingsStore`. The frontend writes
 * via `saveSettings`, which atomically persists to disk on the Rust
 * side; this store keeps an in-memory mirror so consumers don't re-fetch
 * on every render.
 */

import { create } from "zustand";

import {
  getSettings as fetchSettings,
  saveSettings as persistSettings,
} from "@/shared/lib/ipc";
import type { Settings } from "@/shared/types/Settings";

interface SettingsState {
  settings: Settings | null;
  loading: boolean;
  error: string | null;

  /** Load settings from the backend into the cache. */
  load: () => Promise<void>;
  /** Persist settings to disk and update the cache on success. */
  save: (next: Settings) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: null,
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const settings = await fetchSettings();
      set({ settings, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  save: async (next: Settings) => {
    set({ loading: true, error: null });
    try {
      await persistSettings(next);
      set({ settings: next, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },
}));
