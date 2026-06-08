import { create } from "zustand";

export type SettingsSection =
  | "preferences"
  | "notifications"
  | "general"
  | "audio"
  | "transcription"
  | "ai"
  | "storage"
  | "analytics"
  | "connectors"
  | "webhooks"
  | "usage"
  | "privacy"
  | "appearance";

interface SettingsUiState {
  open: boolean;
  section: SettingsSection;

  openAt: (section?: SettingsSection) => void;

  close: () => void;

  setSection: (section: SettingsSection) => void;

  setOpen: (open: boolean) => void;
}

export const useSettingsUiStore = create<SettingsUiState>((set) => ({
  open: false,
  section: "preferences",

  openAt: (section) => set((s) => ({ open: true, section: section ?? s.section })),
  close: () => set({ open: false }),
  setSection: (section) => set({ section }),
  setOpen: (open) => set({ open }),
}));
