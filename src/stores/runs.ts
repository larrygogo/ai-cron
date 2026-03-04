import { create } from "zustand";
import type { Run } from "../lib/types";
import * as api from "../lib/tauri";

interface RunStore {
  runsByTask: Record<string, Run[]>;
  liveOutput: Record<string, { stdout: string; stderr: string }>;
  loading: Record<string, boolean>;

  fetchRuns: (taskId: string) => Promise<void>;
  updateRunStatus: (
    runId: string,
    taskId: string,
    status: string,
    durationMs?: number,
    exitCode?: number
  ) => void;
  appendOutput: (
    runId: string,
    taskId: string,
    chunk: string,
    stream: "stdout" | "stderr"
  ) => void;
  clearLiveOutput: (runId: string) => void;
}

export const useRunStore = create<RunStore>((set) => ({
  runsByTask: {},
  liveOutput: {},
  loading: {},

  fetchRuns: async (taskId) => {
    set((s) => ({ loading: { ...s.loading, [taskId]: true } }));
    try {
      const runs = await api.getRuns(taskId, 50);
      set((s) => ({
        runsByTask: { ...s.runsByTask, [taskId]: runs },
        loading: { ...s.loading, [taskId]: false },
      }));
    } catch {
      set((s) => ({ loading: { ...s.loading, [taskId]: false } }));
    }
  },

  updateRunStatus: (runId, taskId, status, durationMs, exitCode) => {
    set((s) => ({
      runsByTask: {
        ...s.runsByTask,
        [taskId]: (s.runsByTask[taskId] || []).map((r) =>
          r.id === runId
            ? {
                ...r,
                status: status as Run["status"],
                duration_ms: durationMs,
                exit_code: exitCode,
                ended_at: new Date().toISOString(),
              }
            : r
        ),
      },
    }));
  },

  appendOutput: (runId, _taskId, chunk, stream) => {
    set((s) => {
      const current = s.liveOutput[runId] || { stdout: "", stderr: "" };
      return {
        liveOutput: {
          ...s.liveOutput,
          [runId]: {
            stdout:
              stream === "stdout"
                ? current.stdout + chunk + "\n"
                : current.stdout,
            stderr:
              stream === "stderr"
                ? current.stderr + chunk + "\n"
                : current.stderr,
          },
        },
      };
    });
  },

  clearLiveOutput: (runId) => {
    set((s) => {
      const { [runId]: _, ...rest } = s.liveOutput;
      return { liveOutput: rest };
    });
  },
}));
