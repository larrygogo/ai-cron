import { useState, useEffect } from "react";
import { TaskSidebar } from "../components/tasks/TaskSidebar";
import { TaskDetail } from "../components/tasks/TaskDetail";
import { AddTaskModal } from "../components/nl/AddTaskModal";
import { TaskFormModal } from "../components/tasks/TaskFormModal";
import { useTaskStore } from "../stores/tasks";
import { useRunStore } from "../stores/runs";
import { Bot } from "lucide-react";
import { ConfirmDialog } from "../components/ui/ConfirmDialog";
import * as api from "../lib/tauri";
import type { Task } from "../lib/types";

export function Dashboard() {
  const { tasks, selectedId } = useTaskStore();
  const { appendOutput, updateRunStatus } = useRunStore();
  const [showAddModal, setShowAddModal] = useState(false);
  const [editingTask, setEditingTask] = useState<Task | null>(null);
  const [liveRunId, setLiveRunId] = useState<string | undefined>();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const selectedTask = tasks.find((t) => t.id === selectedId) ?? null;

  // Subscribe to run events (global, not per-task — callbacks read selectedId dynamically)
  useEffect(() => {
    let cancelled = false;
    const unsubs: (() => void)[] = [];

    const setup = async () => {
      const unsubStarted = await api.onRunStarted(({ runId, taskId }) => {
        if (cancelled) return;
        const currentSelectedId = useTaskStore.getState().selectedId;
        if (taskId === currentSelectedId) setLiveRunId(runId);
      });
      if (cancelled) { unsubStarted(); return; }
      unsubs.push(unsubStarted);

      const unsubOutput = await api.onRunOutput(({ runId, chunk, stream }) => {
        if (cancelled) return;
        const state = useTaskStore.getState();
        const task = state.tasks.find((t) => t.id === state.selectedId);
        if (task) appendOutput(runId, task.id, chunk, stream);
      });
      if (cancelled) { unsubOutput(); return; }
      unsubs.push(unsubOutput);

      const unsubCompleted = await api.onRunCompleted(({ runId, taskId, status, exitCode, durationMs }) => {
        if (cancelled) return;
        setLiveRunId((prev) => (prev === runId ? undefined : prev));
        updateRunStatus(runId, taskId, status, durationMs, exitCode);
        useTaskStore.getState().fetchTasks();
      });
      if (cancelled) { unsubCompleted(); return; }
      unsubs.push(unsubCompleted);
    };

    setup();

    return () => {
      cancelled = true;
      unsubs.forEach((unsub) => unsub());
    };
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      switch (e.key) {
        case "n": case "N": setShowAddModal(true); break;
        case "e": case "E": if (selectedTask) setEditingTask(selectedTask); break;
        case "r": case "R":
          if (selectedTask) api.triggerTaskNow(selectedTask.id).catch(console.error);
          break;
        case "Delete":
          if (selectedTask) setShowDeleteConfirm(true);
          break;
        case " ":
          if (selectedTask) {
            e.preventDefault();
            api.setTaskEnabled(selectedTask.id, !selectedTask.enabled).then(() => {
              useTaskStore.getState().updateTaskInStore({ ...selectedTask, enabled: !selectedTask.enabled });
            });
          }
          break;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [selectedTask]);

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
      {editingTask && (
        <TaskFormModal task={editingTask} onClose={() => setEditingTask(null)} />
      )}
      {showDeleteConfirm && selectedTask && (
        <ConfirmDialog
          title="删除任务"
          message={`确定删除任务 "${selectedTask.name}"？此操作不可撤销。`}
          confirmLabel="删除"
          danger
          onConfirm={() => {
            setShowDeleteConfirm(false);
            api.deleteTask(selectedTask.id).then(() => {
              useTaskStore.getState().removeTaskFromStore(selectedTask.id);
            });
          }}
          onCancel={() => setShowDeleteConfirm(false)}
        />
      )}
    </div>
  );
}

function EmptyDetail({ onAdd }: { onAdd: () => void }) {
  return (
    <div className="empty-state" style={{ flex: 1 }}>
      <Bot size={32} style={{ color: "var(--text-muted)" }} />
      <div>
        <div style={{ fontSize: 13, marginBottom: 4 }}>未选择任务</div>
        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
          从侧边栏选择一个任务，或创建新任务
        </div>
      </div>
      <button className="btn btn-ghost" onClick={onAdd}>
        添加第一个任务 <kbd style={{ fontSize: 10, opacity: 0.6 }}>N</kbd>
      </button>
    </div>
  );
}
