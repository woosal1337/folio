/**
 * Memories store — mirrors useTasksStore. Source of truth lives in
 * the Rust MemoryStore (markdown files + SQLite); this Zustand store
 * caches the list for the /memory page and exposes optimistic
 * mutations.
 *
 * Refresh-after-write is the default rather than splice-into-array
 * because `create` may supersede a prior memory — the server-side
 * conflict resolution can change more than one row in a single call.
 */

import { create } from "zustand";
import { toast } from "sonner";

import {
  createMemory as ipcCreate,
  deleteMemory as ipcDelete,
  listMemories as ipcList,
  pinMemory as ipcPin,
  purgeMemory as ipcPurge,
  rebuildMemoryIndex as ipcRebuild,
  updateMemory as ipcUpdate,
} from "@/shared/lib/ipc";
import type { Memory } from "@/shared/types/Memory";
import type { MemoryKind } from "@/shared/types/MemoryKind";
import type { MemoryQuery } from "@/shared/types/MemoryQuery";
import type { MemoryUpdate } from "@/shared/types/MemoryUpdate";
import type { NewMemory } from "@/shared/types/NewMemory";

interface MemoriesState {
  memories: Memory[];
  loading: boolean;
  error: string | null;
  /** Whether the current view includes superseded / soft-deleted rows. */
  includeArchived: boolean;
  /** Optional kind filter; empty array means "all". */
  kindsFilter: MemoryKind[];

  setIncludeArchived: (include: boolean) => void;
  setKindsFilter: (kinds: MemoryKind[]) => void;

  refresh: () => Promise<void>;
  create: (memory: NewMemory) => Promise<Memory | null>;
  update: (id: string, patch: MemoryUpdate) => Promise<Memory | null>;
  pin: (id: string, pinned: boolean) => Promise<Memory | null>;
  remove: (id: string) => Promise<void>;
  purge: (id: string) => Promise<void>;
  rebuildIndex: () => Promise<number | null>;
}

export const useMemoriesStore = create<MemoriesState>((set, get) => ({
  memories: [],
  loading: true,
  error: null,
  includeArchived: false,
  kindsFilter: [],

  setIncludeArchived: (includeArchived) => {
    set({ includeArchived });
    void get().refresh();
  },
  setKindsFilter: (kindsFilter) => {
    set({ kindsFilter });
    void get().refresh();
  },

  refresh: async () => {
    set({ loading: true, error: null });
    try {
      const { includeArchived, kindsFilter } = get();
      const query: MemoryQuery = {
        query: null,
        kinds: kindsFilter,
        include_archived: includeArchived,
        limit: null,
      };
      const memories = await ipcList(query);
      set({ memories, loading: false });
    } catch (e) {
      const message = String(e);
      console.error("memories_store.refresh:", e);
      set({ error: message, loading: false });
      toast.error("Could not load memories", { description: message });
    }
  },

  create: async (newMemory) => {
    try {
      const memory = await ipcCreate(newMemory);
      // Refresh in case create superseded a prior memory.
      await get().refresh();
      return memory;
    } catch (e) {
      console.error("memories_store.create:", e);
      toast.error("Could not create memory", { description: String(e) });
      return null;
    }
  },

  update: async (id, patch) => {
    try {
      const updated = await ipcUpdate(id, patch);
      set((s) => ({
        memories: s.memories.map((m) => (m.id === id ? updated : m)),
      }));
      return updated;
    } catch (e) {
      console.error("memories_store.update:", e);
      toast.error("Could not update memory", { description: String(e) });
      return null;
    }
  },

  pin: async (id, pinned) => {
    try {
      const updated = await ipcPin(id, pinned);
      set((s) => ({
        memories: s.memories.map((m) => (m.id === id ? updated : m)),
      }));
      return updated;
    } catch (e) {
      console.error("memories_store.pin:", e);
      toast.error("Could not pin memory", { description: String(e) });
      return null;
    }
  },

  remove: async (id) => {
    // Soft delete — the row stays in the on-disk store but gets
    // valid_until set. We optimistically hide it from the live view.
    const prev = get().memories;
    set((s) => ({ memories: s.memories.filter((m) => m.id !== id) }));
    try {
      await ipcDelete(id);
    } catch (e) {
      console.error("memories_store.remove:", e);
      toast.error("Could not archive memory", { description: String(e) });
      set({ memories: prev });
    }
  },

  purge: async (id) => {
    const prev = get().memories;
    set((s) => ({ memories: s.memories.filter((m) => m.id !== id) }));
    try {
      await ipcPurge(id);
    } catch (e) {
      console.error("memories_store.purge:", e);
      toast.error("Could not purge memory", { description: String(e) });
      set({ memories: prev });
    }
  },

  rebuildIndex: async () => {
    try {
      const n = await ipcRebuild();
      toast.success(`Memory index rebuilt`, {
        description: `${n} memory${n === 1 ? "y" : "ies"} reindexed`,
      });
      await get().refresh();
      return n;
    } catch (e) {
      console.error("memories_store.rebuildIndex:", e);
      toast.error("Could not rebuild memory index", { description: String(e) });
      return null;
    }
  },
}));
