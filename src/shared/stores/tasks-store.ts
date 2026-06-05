import { create } from "zustand";
import { toast } from "sonner";

import { humanizeError } from "@/shared/lib/errors";
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

  loading: boolean;

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
      const message = humanizeError(e);
      console.error("tasks_store.refresh:", e);
      set({ error: message, loading: false });
      toast.error("Could not load tasks", { description: message });
    }
  },

  create: async (newTask) => {
    try {
      const task = await ipcCreate(newTask);

      set((s) => ({ tasks: [...s.tasks, task] }));
      return task;
    } catch (e) {
      const message = humanizeError(e);
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
      const message = humanizeError(e);
      console.error("tasks_store.update:", e);
      toast.error("Could not update task", { description: message });
      return null;
    }
  },

  remove: async (id) => {
    const prev = get().tasks;
    set((s) => ({ tasks: s.tasks.filter((t) => t.id !== id) }));
    try {
      await ipcDelete(id);
    } catch (e) {
      console.error("tasks_store.remove:", e);
      toast.error("Could not delete task", { description: humanizeError(e) });
      set({ tasks: prev });
    }
  },

  setStatus: async (id, status) => {
    const prev = get().tasks;
    set((s) => ({
      tasks: s.tasks.map((t) =>
        t.id === id ? { ...t, status, updated_at: new Date().toISOString() } : t
      ),
    }));
    try {
      const updated = await ipcSetStatus(id, status);

      set((s) => ({ tasks: s.tasks.map((t) => (t.id === id ? updated : t)) }));
    } catch (e) {
      console.error("tasks_store.setStatus:", e);
      toast.error("Could not move task", { description: humanizeError(e) });
      set({ tasks: prev });
    }
  },
}));
