import { useState, useEffect } from "react";
import { TaskSidebar } from "../components/tasks/TaskSidebar";
import { TaskDetail } from "../components/tasks/TaskDetail";
import { AddTaskModal } from "../components/nl/AddTaskModal";
import { useTaskStore } from "../stores/tasks";
import { useRunStore } from "../stores/runs";
import { Bot } from "lucide-react";
import * as api from "../lib/tauri";
import type { Task } from "../lib/types";

export function Dashboard() {
  const { tasks, selectedId } = useTaskStore();
  const { appendOutput, updateRunStatus } = useRunStore();
  const [showAddModal, setShowAddModal] = useState(false);
  const [_editingTask, setEditingTask] = useState<Task | null>(null);
  const [liveRunId, setLiveRunId] = useState<string | undefined>();

  const selectedTask = tasks.find((t) => t.id === selectedId) ?? null;

  // Subscribe to run events
  useEffect(() => {
    const unsubs: Array<Promise<() => void>> = [
      api.onRunStarted(({ runId, taskId }) => {
        if (taskId === selectedId) setLiveRunId(runId);
      }),
      api.onRunOutput(({ runId, chunk, stream }) => {
        const task = tasks.find((t) => t.id === selectedId);
        if (task) appendOutput(runId, task.id, chunk, stream);
      }),
      api.onRunCompleted(({ runId, taskId, status, exitCode, durationMs }) => {
        if (runId === liveRunId) setLiveRunId(undefined);
        updateRunStatus(runId, taskId, status, durationMs, exitCode);
        // Refresh task list to get updated last_run_status
        useTaskStore.getState().fetchTasks();
      }),
    ];

    return () => {
      unsubs.forEach((p) => p.then((unsub) => unsub()));
    };
  }, [selectedId, liveRunId, tasks]);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (e.key === "n" || e.key === "N") setShowAddModal(true);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return (
    <div style={{ display: "flex", height: "100%", width: "100%" }}>
      {/* Sidebar */}
      <TaskSidebar onAddTask={() => setShowAddModal(true)} />

      {/* Main panel */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        {selectedTask ? (
          <TaskDetail
            task={selectedTask}
            onEdit={setEditingTask}
            liveRunId={liveRunId}
          />
        ) : (
          <EmptyDetail onAdd={() => setShowAddModal(true)} />
        )}
      </div>

      {/* Modals */}
      {showAddModal && (
        <AddTaskModal onClose={() => setShowAddModal(false)} />
      )}
    </div>
  );
}

function EmptyDetail({ onAdd }: { onAdd: () => void }) {
  return (
    <div className="empty-state" style={{ flex: 1 }}>
      <Bot size={32} style={{ color: "var(--text-muted)" }} />
      <div>
        <div style={{ fontSize: 13, marginBottom: 4 }}>No task selected</div>
        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
          Select a task from the sidebar or create a new one
        </div>
      </div>
      <button className="btn btn-ghost" onClick={onAdd}>
        Add first task <kbd style={{ fontSize: 10, opacity: 0.6 }}>N</kbd>
      </button>
    </div>
  );
}
