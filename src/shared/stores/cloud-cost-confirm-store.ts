import { create } from "zustand";

import type { CostEstimate } from "@/shared/lib/cost-estimate";

export interface CloudCostConfirmPayload {
  recordingLabel: string;
  estimate: CostEstimate;
}

interface CloudCostConfirmState {
  open: boolean;
  payload: CloudCostConfirmPayload | null;

  _resolve: ((proceed: boolean) => void) | null;

  confirm: (payload: CloudCostConfirmPayload) => Promise<boolean>;

  resolve: (proceed: boolean) => void;
}

export const useCloudCostConfirmStore = create<CloudCostConfirmState>((set, get) => ({
  open: false,
  payload: null,
  _resolve: null,

  confirm: (payload) =>
    new Promise<boolean>((resolve) => {
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
