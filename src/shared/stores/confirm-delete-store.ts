/**
 * Promise-based confirmation for destructive actions (delete a note,
 * space/folder, conversation…). A caller does
 * `if (await confirm({ ... })) { …delete }` without threading props.
 *
 * For irreversible deletes, set `doubleConfirm: true` — the dialog then
 * requires an explicit "I understand" acknowledgement before the delete
 * button enables, so nothing is removed on a single stray click.
 *
 * Mirrors `useCloudCostConfirmStore`; the host dialog
 * (`ConfirmDeleteDialog`) is mounted once at App root.
 */

import { create } from "zustand";

export interface ConfirmDeletePayload {
  /** Dialog title, e.g. "Delete note?". */
  title: string;
  /** Body text explaining what's removed and that it can't be undone. */
  description: string;
  /** Destructive button label. Defaults to "Delete". */
  confirmLabel?: string;
  /** When true, require an "I understand" checkbox before enabling the
   *  destructive button — the second confirmation for irreversible work. */
  doubleConfirm?: boolean;
  /** Optional acknowledgement label shown next to the checkbox. */
  acknowledgeLabel?: string;
}

interface ConfirmDeleteState {
  open: boolean;
  payload: ConfirmDeletePayload | null;
  _resolve: ((confirmed: boolean) => void) | null;
  /** Open the dialog; resolves true only on an explicit confirm. */
  confirm: (payload: ConfirmDeletePayload) => Promise<boolean>;
  /** Internal — wired to the dialog's Cancel/Delete buttons + close. */
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

/** Convenience wrapper for callers. */
export function confirmDelete(payload: ConfirmDeletePayload): Promise<boolean> {
  return useConfirmDeleteStore.getState().confirm(payload);
}
