import { create } from "zustand";
import type { Task } from "../lib/types";
import * as api from "../lib/tauri";

interface TaskStore {
  tasks: Task[];
  selectedId: string | null;
  loading: boolean;
  error: string | null;

  fetchTasks: () => Promise<void>;
  setSelected: (id: string | null) => void;
  updateTaskInStore: (task: Task) => void;
  removeTaskFromStore: (id: string) => void;
  addTaskToStore: (task: Task) => void;
}

export const useTaskStore = create<TaskStore>((set) => ({
  tasks: [],
  selectedId: null,
  loading: false,
  error: null,

  fetchTasks: async () => {
    set({ loading: true, error: null });
    try {
      const tasks = await api.getTasks();
      set({ tasks, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  setSelected: (id) => set({ selectedId: id }),

  updateTaskInStore: (task) =>
    set((s) => ({
      tasks: s.tasks.map((t) => (t.id === task.id ? task : t)),
    })),

  removeTaskFromStore: (id) =>
    set((s) => ({
      tasks: s.tasks.filter((t) => t.id !== id),
      selectedId: s.selectedId === id ? null : s.selectedId,
    })),

  addTaskToStore: (task) =>
    set((s) => ({ tasks: [task, ...s.tasks], selectedId: task.id })),
}));
