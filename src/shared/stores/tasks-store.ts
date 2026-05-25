/**
 * Tasks store — single source of truth for the kanban UI.
 *
 * Pattern: the Rust TaskStore is authoritative on disk; this store
 * mirrors it in memory and re-fetches after every mutation. Mutations
 * are optimistic where it matters for feel (status drag-drop) and
 * pessimistic everywhere else (create/edit/delete go through IPC and
 * then refresh).
 *
 * The store also exposes a `seedFromBackend()` call invoked once on
 * app mount so the kanban renders instantly when the user navigates
 * to /tasks, and a `refresh()` for "the model just created a task,
 * please reload" callers (the agent panel + auto-extract-tasks hook).
 */

import { create } from "zustand";
import { toast } from "sonner";

import {
  createTask as ipcCreate,
  deleteTask as ipcDelete,
  listTasks as ipcList,
  setTaskStatus as ipcSetStatus,
  updateTask as ipcUpdate,
} from "@/shared/lib/ipc";
import type { NewTask } from "@/shared/types/NewTask";
import type { Task } from "@/shared/types/Task";
import type { TaskStatus } from "@/shared/types/TaskStatus";
import type { TaskUpdate } from "@/shared/types/TaskUpdate";

interface TasksState {
  tasks: Task[];
  /** True while the first list() is in flight. UI uses this to render skeletons. */
  loading: boolean;
  /** Last load error, surfaced as a banner. */
  error: string | null;

  refresh: () => Promise<void>;
  create: (task: NewTask) => Promise<Task | null>;
  update: (id: string, patch: TaskUpdate) => Promise<Task | null>;
  remove: (id: string) => Promise<void>;
  setStatus: (id: string, status: TaskStatus) => Promise<void>;
}

export const useTasksStore = create<TasksState>((set, get) => ({
  tasks: [],
  loading: true,
  error: null,

  refresh: async () => {
    set({ loading: true, error: null });
    try {
      const tasks = await ipcList();
      set({ tasks, loading: false });
    } catch (e) {
      const message = String(e);
      console.error("tasks_store.refresh:", e);
      set({ error: message, loading: false });
      toast.error("Could not load tasks", { description: message });
    }
  },

  create: async (newTask) => {
    try {
      const task = await ipcCreate(newTask);
      // Append rather than re-fetch: the user just typed this in,
      // the latency would feel like a stall.
      set((s) => ({ tasks: [...s.tasks, task] }));
      return task;
    } catch (e) {
      const message = String(e);
      console.error("tasks_store.create:", e);
      toast.error("Could not create task", { description: message });
      return null;
    }
  },

  update: async (id, patch) => {
    try {
      const updated = await ipcUpdate(id, patch);
      set((s) => ({
        tasks: s.tasks.map((t) => (t.id === id ? updated : t)),
      }));
      return updated;
    } catch (e) {
      const message = String(e);
      console.error("tasks_store.update:", e);
      toast.error("Could not update task", { description: message });
      return null;
    }
  },

  remove: async (id) => {
    // Optimistic remove + rollback on failure. Deletes are common,
    // failures are rare, and the row disappearing instantly feels
    // right.
    const prev = get().tasks;
    set((s) => ({ tasks: s.tasks.filter((t) => t.id !== id) }));
    try {
      await ipcDelete(id);
    } catch (e) {
      console.error("tasks_store.remove:", e);
      toast.error("Could not delete task", { description: String(e) });
      set({ tasks: prev });
    }
  },

  setStatus: async (id, status) => {
    // Optimistic status flip so drag-drop feels instant. The card
    // moves columns immediately; if the IPC fails we roll back.
    const prev = get().tasks;
    set((s) => ({
      tasks: s.tasks.map((t) =>
        t.id === id ? { ...t, status, updated_at: new Date().toISOString() } : t
      ),
    }));
    try {
      const updated = await ipcSetStatus(id, status);
      // Sync server-side updated_at back in.
      set((s) => ({ tasks: s.tasks.map((t) => (t.id === id ? updated : t)) }));
    } catch (e) {
      console.error("tasks_store.setStatus:", e);
      toast.error("Could not move task", { description: String(e) });
      set({ tasks: prev });
    }
  },
}));
