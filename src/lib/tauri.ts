import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppSettings,
  CreateTaskRequest,
  Run,
  RunWithTaskName,
  Task,
  TaskDraft,
  ToolInfo,
  UpdateTaskRequest,
} from "./types";

// ── Tasks ──────────────────────────────────────────────────────────────────
export const getTasks = () => invoke<Task[]>("get_tasks");
export const getTask = (id: string) => invoke<Task>("get_task", { id });
export const createTask = (req: CreateTaskRequest) =>
  invoke<Task>("create_task", { req });
export const updateTask = (id: string, req: UpdateTaskRequest) =>
  invoke<Task>("update_task", { id, req });
export const deleteTask = (id: string) => invoke<void>("delete_task", { id });
export const setTaskEnabled = (id: string, enabled: boolean) =>
  invoke<void>("set_task_enabled", { id, enabled });

// ── Runs ───────────────────────────────────────────────────────────────────
export const getRuns = (taskId: string, limit?: number) =>
  invoke<Run[]>("get_runs", { taskId, limit });
export const getAllRuns = (params?: {
  limit?: number;
  offset?: number;
  statusFilter?: string;
  searchQuery?: string;
}) =>
  invoke<RunWithTaskName[]>("get_all_runs", {
    limit: params?.limit,
    offset: params?.offset,
    statusFilter: params?.statusFilter,
    searchQuery: params?.searchQuery,
  });
export const getRun = (id: string) => invoke<Run>("get_run", { id });
export const deleteRunsForTask = (taskId: string) =>
  invoke<void>("delete_runs_for_task", { taskId });
export const cleanupOldRuns = () => invoke<number>("cleanup_old_runs");

// ── Runner ─────────────────────────────────────────────────────────────────
export const triggerTaskNow = (taskId: string) =>
  invoke<string>("trigger_task_now", { taskId });
export const killRun = (runId: string) => invoke<void>("kill_run", { runId });

// ── Scheduler ──────────────────────────────────────────────────────────────
export const previewNextRuns = (cronExpr: string, count?: number) =>
  invoke<string[]>("preview_next_runs", { cronExpr, count });

// ── Tools & Settings ───────────────────────────────────────────────────────
export const detectTools = () => invoke<ToolInfo[]>("detect_tools");
export const getSettings = () => invoke<AppSettings>("get_settings");
export const updateSettings = (settings: AppSettings) =>
  invoke<void>("update_settings", { settings });

// ── AI Parse ───────────────────────────────────────────────────────────────
export const parseNlToTask = (input: string) =>
  invoke<TaskDraft>("parse_nl_to_task", { input });

// ── Events ─────────────────────────────────────────────────────────────────
export const onRunStarted = (
  cb: (e: { runId: string; taskId: string }) => void
) => listen<{ runId: string; taskId: string }>("run:started", (e) => cb(e.payload));

export const onRunOutput = (
  cb: (e: { runId: string; chunk: string; stream: "stdout" | "stderr" }) => void
) =>
  listen<{ runId: string; chunk: string; stream: "stdout" | "stderr" }>(
    "run:output",
    (e) => cb(e.payload)
  );

export const onRunCompleted = (
  cb: (e: {
    runId: string;
    taskId: string;
    status: string;
    exitCode?: number;
    durationMs: number;
  }) => void
) =>
  listen<{
    runId: string;
    taskId: string;
    status: string;
    exitCode?: number;
    durationMs: number;
  }>("run:completed", (e) => cb(e.payload));
