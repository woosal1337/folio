/**
 * Settings-modal UI state. Separate from `settings-store` (which owns
 * the persisted Settings struct) — this one only tracks whether the
 * modal is open and which section is active, so any component can
 * deep-link into Settings without prop-drilling a callback through
 * five layers of React tree.
 *
 * Usage:
 *   const openAt = useSettingsUiStore((s) => s.openAt);
 *   <button onClick={() => openAt("ai")}>Configure</button>
 */

import { create } from "zustand";

export type SettingsSection =
  | "general"
  | "audio"
  | "transcription"
  | "ai"
  | "storage"
  | "appearance";

interface SettingsUiState {
  open: boolean;
  section: SettingsSection;

  /** Open the modal, optionally jumping to a specific section. */
  openAt: (section?: SettingsSection) => void;
  /** Close without resetting the section (so re-opening lands where the user left off). */
  close: () => void;
  /** Switch sections while the modal is already open. */
  setSection: (section: SettingsSection) => void;
  /** Bind <Dialog open={...} onOpenChange={...}> to one prop. */
  setOpen: (open: boolean) => void;
}

export const useSettingsUiStore = create<SettingsUiState>((set) => ({
  open: false,
  section: "general",

  openAt: (section) => set((s) => ({ open: true, section: section ?? s.section })),
  close: () => set({ open: false }),
  setSection: (section) => set({ section }),
  setOpen: (open) => set({ open }),
}));
