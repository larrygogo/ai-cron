import { useEffect } from "react";
import { Plus, RefreshCw } from "lucide-react";
import { useTaskStore } from "../../stores/tasks";
import { TaskItem } from "./TaskItem";

interface Props {
  onAddTask: () => void;
}

export function TaskSidebar({ onAddTask }: Props) {
  const { tasks, selectedId, fetchTasks, setSelected, loading } =
    useTaskStore();

  useEffect(() => {
    fetchTasks();
  }, []);

  return (
    <div
      style={{
        width: 240,
        flexShrink: 0,
        background: "var(--bg-sidebar)",
        borderRight: "1px solid var(--border)",
        display: "flex",
        flexDirection: "column",
        height: "100%",
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "12px 14px 10px",
          borderBottom: "1px solid var(--border-subtle)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <span
          style={{
            fontSize: 11,
            color: "var(--text-muted)",
            textTransform: "uppercase",
            letterSpacing: "0.08em",
          }}
        >
          任务{" "}
          {tasks.length > 0 && (
            <span style={{ color: "var(--text-secondary)" }}>
              ({tasks.length})
            </span>
          )}
        </span>
        <div style={{ display: "flex", gap: 4 }}>
          <button
            className="btn btn-ghost"
            style={{ padding: "3px 6px", border: "none" }}
            onClick={() => fetchTasks()}
            title="刷新"
          >
            <RefreshCw size={12} />
          </button>
          <button
            className="btn btn-ghost"
            style={{ padding: "3px 6px", border: "none" }}
            onClick={onAddTask}
            title="新建任务 (N)"
          >
            <Plus size={12} />
          </button>
        </div>
      </div>

      {/* Task list */}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {loading && tasks.length === 0 && (
          <div className="empty-state" style={{ padding: "30px 14px" }}>
            <span style={{ fontSize: 11 }}>加载中...</span>
          </div>
        )}
        {!loading && tasks.length === 0 && (
          <div className="empty-state" style={{ padding: "30px 14px" }}>
            <span style={{ fontSize: 11 }}>暂无任务</span>
            <button className="btn btn-ghost" onClick={onAddTask}>
              <Plus size={11} /> 添加任务
            </button>
          </div>
        )}
        {tasks.map((task) => (
          <TaskItem
            key={task.id}
            task={task}
            selected={selectedId === task.id}
            onClick={() => setSelected(task.id)}
          />
        ))}
      </div>

      {/* Footer: add */}
      <div
        style={{
          padding: "10px 14px",
          borderTop: "1px solid var(--border-subtle)",
        }}
      >
        <button
          className="btn btn-ghost"
          style={{ width: "100%", justifyContent: "center" }}
          onClick={onAddTask}
        >
          <Plus size={12} /> 添加任务
        </button>
      </div>
    </div>
  );
}
