import { create } from "zustand";

import {
  createFolder as ipcCreate,
  deleteFolder as ipcDelete,
  listFolders as ipcList,
  renameFolder as ipcRename,
  setNoteFolder as ipcSetNoteFolder,
} from "@/shared/lib/ipc";

interface FoldersState {
  folders: string[];
  loaded: boolean;
  load: () => Promise<void>;
  create: (name: string) => Promise<void>;
  rename: (from: string, to: string) => Promise<void>;
  remove: (name: string) => Promise<void>;

  assign: (sessionDir: string, folder: string | null) => Promise<void>;
}

export const useFolders = create<FoldersState>((set) => ({
  folders: [],
  loaded: false,
  load: async () => {
    try {
      set({ folders: await ipcList(), loaded: true });
    } catch (e) {
      console.error("list_folders:", e);
    }
  },
  create: async (name) => {
    set({ folders: await ipcCreate(name) });
  },
  rename: async (from, to) => {
    set({ folders: await ipcRename(from, to) });
  },
  remove: async (name) => {
    set({ folders: await ipcDelete(name) });
  },
  assign: async (sessionDir, folder) => {
    await ipcSetNoteFolder(sessionDir, folder);
    set({ folders: await ipcList() });
  },
}));
