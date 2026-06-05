import { create } from "zustand";

import { humanizeError } from "@/shared/lib/errors";
import {
  getSettings as fetchSettings,
  saveSettings as persistSettings,
} from "@/shared/lib/ipc";
import type { Settings } from "@/shared/types/Settings";

interface SettingsState {
  settings: Settings | null;
  loading: boolean;
  error: string | null;

  load: () => Promise<void>;

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
      set({ error: humanizeError(e), loading: false });
    }
  },

  save: async (next: Settings) => {
    set({ loading: true, error: null });
    try {
      await persistSettings(next);
      set({ settings: next, loading: false });
    } catch (e) {
      set({ error: humanizeError(e), loading: false });
      throw e;
    }
  },
}));
