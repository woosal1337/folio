/**
 * Promise-based confirm dialog for "are you sure you want to upload
 * this WAV to OpenAI?" — the recording-store calls `confirm(...)` and
 * awaits the user's choice without having to thread props through the
 * component tree. v2 roadmap finding 055.
 *
 * The dialog component (`CloudCostConfirmDialog`) is mounted once at
 * App root and subscribes to this store; whichever code path needs to
 * confirm just imports `useCloudCostConfirmStore.getState().confirm()`.
 *
 * Pattern mirrors the existing `useSettingsUiStore` deep-link helper.
 */

import { create } from "zustand";

import type { CostEstimate } from "@/shared/lib/cost-estimate";

export interface CloudCostConfirmPayload {
  recordingLabel: string;
  estimate: CostEstimate;
}

interface CloudCostConfirmState {
  open: boolean;
  payload: CloudCostConfirmPayload | null;
  /** Internal resolver — fulfilled by the dialog's Confirm/Cancel buttons. */
  _resolve: ((proceed: boolean) => void) | null;

  /**
   * Open the confirm dialog. Returns a promise resolving to `true` if
   * the user picked "Upload" and `false` if they cancelled or closed.
   */
  confirm: (payload: CloudCostConfirmPayload) => Promise<boolean>;
  /** Internal — wired to the dialog's buttons. */
  resolve: (proceed: boolean) => void;
}

export const useCloudCostConfirmStore = create<CloudCostConfirmState>((set, get) => ({
  open: false,
  payload: null,
  _resolve: null,

  confirm: (payload) =>
    new Promise<boolean>((resolve) => {
      // If a prior confirm is somehow still pending (re-entry), resolve
      // it as cancelled so we don't leak a hanging promise.
      const prev = get()._resolve;
      if (prev) prev(false);
      set({ open: true, payload, _resolve: resolve });
    }),

  resolve: (proceed) => {
    const r = get()._resolve;
    if (r) r(proceed);
    set({ open: false, payload: null, _resolve: null });
  },
}));
