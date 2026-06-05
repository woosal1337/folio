import { create } from "zustand";

export interface ConfirmDeletePayload {
  title: string;

  description: string;

  confirmLabel?: string;
}

interface ConfirmDeleteState {
  open: boolean;
  payload: ConfirmDeletePayload | null;
  _resolve: ((confirmed: boolean) => void) | null;

  confirm: (payload: ConfirmDeletePayload) => Promise<boolean>;

  resolve: (confirmed: boolean) => void;
}

export const useConfirmDeleteStore = create<ConfirmDeleteState>((set, get) => ({
  open: false,
  payload: null,
  _resolve: null,

  confirm: (payload) =>
    new Promise<boolean>((resolve) => {
      const prev = get()._resolve;
      if (prev) prev(false);
      set({ open: true, payload, _resolve: resolve });
    }),

  resolve: (confirmed) => {
    const r = get()._resolve;
    if (r) r(confirmed);
    set({ open: false, payload: null, _resolve: null });
  },
}));

export function confirmDelete(payload: ConfirmDeletePayload): Promise<boolean> {
  return useConfirmDeleteStore.getState().confirm(payload);
}
