import { create } from "zustand";

import { remoteMe, type RemoteAccount } from "@/shared/lib/ipc";

interface RemoteAccountState {
  account: RemoteAccount | null;
  loading: boolean;

  refresh: () => Promise<void>;
}

export const useRemoteAccountStore = create<RemoteAccountState>((set) => ({
  account: null,
  loading: false,

  refresh: async () => {
    set({ loading: true });
    try {
      set({ account: await remoteMe(), loading: false });
    } catch (e) {
      console.error("remote_me:", e);
      set({ loading: false });
    }
  },
}));
