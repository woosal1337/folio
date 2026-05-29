/**
 * Note folders ("Spaces"), GET-162. A tiny shared cache of the folder
 * list so the sidebar Spaces section, My Notes filter, and the note
 * header "Add to folder" chip all read/write one source of truth. The
 * backend (`folder.txt` per note + `folders.json` registry) is
 * authoritative; mutators return the fresh list and we store it.
 */

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
  /** Assign/clear a note's folder, then refresh the list. */
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
