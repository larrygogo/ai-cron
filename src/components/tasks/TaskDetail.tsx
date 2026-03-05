import { useState } from "react";
import { Play, Pencil, Trash2 } from "lucide-react";
import { useTaskStore } from "../../stores/tasks";
import { StatusBadge } from "./StatusBadge";
import { RunHistory } from "../runs/RunHistory";
import { NextRunsPreview } from "../scheduler/NextRunsPreview";
import type { Task } from "../../lib/types";
import * as api from "../../lib/tauri";

interface Props {
  task: Task;
  onEdit: (task: Task) => void;
  liveRunId?: string;
}

const toolLabels: Record<string, string> = {
  claude: "claude -p",
  opencode: "opencode",
  codex: "codex --approval-mode full-auto",
  custom: "custom",
};

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="toggle">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span className="toggle-track" />
    </label>
  );
}

export function TaskDetail({ task, onEdit, liveRunId }: Props) {
  const { updateTaskInStore, removeTaskFromStore } = useTaskStore();
  const [running, setRunning] = useState(false);

  const handleToggleEnabled = async () => {
    try {
      await api.setTaskEnabled(task.id, !task.enabled);
      updateTaskInStore({ ...task, enabled: !task.enabled });
    } catch (e) {
      console.error(e);
    }
  };

  const handleRunNow = async () => {
    setRunning(true);
    try {
      await api.triggerTaskNow(task.id);
    } catch (e) {
      console.error(e);
    } finally {
      setRunning(false);
    }
  };

  const handleDelete = async () => {
    if (!confirm(`确定删除任务 "${task.name}"？`)) return;
    try {
      await api.deleteTask(task.id);
      removeTaskFromStore(task.id);
    } catch (e) {
      console.error("Delete failed:", e);
    }
  };

  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        height: "100%",
        overflowY: "auto",
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "14px 20px 12px",
          borderBottom: "1px solid var(--border)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          flexShrink: 0,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <span style={{ fontSize: 14, fontWeight: 500 }}>{task.name}</span>
          <StatusBadge
            status={
              !task.enabled
                ? "disabled"
                : liveRunId
                ? "running"
                : task.last_run_status ?? "unknown"
            }
          />
        </div>
        <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <button
            className="btn btn-ghost"
            onClick={handleRunNow}
            disabled={running}
            style={{ fontSize: 11 }}
          >
            <Play size={11} />
            {running ? "运行中..." : "立即运行"}
          </button>
          <button
            className="btn btn-ghost"
            onClick={() => onEdit(task)}
            style={{ fontSize: 11 }}
          >
            <Pencil size={11} />
            编辑
          </button>
          <div
            style={{ display: "flex", alignItems: "center", gap: 6 }}
            title={task.enabled ? "Disable" : "Enable"}
          >
            <Toggle checked={task.enabled} onChange={handleToggleEnabled} />
          </div>
          <button
            className="btn btn-danger"
            onClick={handleDelete}
            style={{ fontSize: 11 }}
          >
            <Trash2 size={11} />
          </button>
        </div>
      </div>

      {/* Content */}
      <div style={{ padding: "18px 20px", display: "flex", flexDirection: "column", gap: 20 }}>

        {/* Info grid */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "12px 20px" }}>
          <InfoRow label="工具">
            <code style={{ fontSize: 11.5, color: "var(--accent-blue)" }}>
              {toolLabels[task.ai_tool] ?? task.ai_tool}
            </code>
          </InfoRow>
          <InfoRow label="调度">
            <span style={{ fontSize: 12 }}>
              {task.cron_human || task.cron_expression}
            </span>
            <code
              style={{
                fontSize: 10,
                color: "var(--text-muted)",
                marginLeft: 6,
              }}
            >
              ({task.cron_expression})
            </code>
          </InfoRow>
          <InfoRow label="目录">
            <code style={{ fontSize: 11 }}>{task.working_directory || "—"}</code>
          </InfoRow>
          <InfoRow label="安全">
            <span style={{ fontSize: 11.5 }}>
              {task.restrict_network && "禁止网络 "}
              {task.restrict_filesystem && "禁止文件系统 "}
              {!task.restrict_network && !task.restrict_filesystem && "不限制"}
            </span>
          </InfoRow>
          {task.inject_context && (
            <InfoRow label="上下文">
              <span style={{ fontSize: 11, color: "var(--accent)" }}>
                注入时间 + 上次运行信息
              </span>
            </InfoRow>
          )}
        </div>

        {/* Prompt */}
        <div>
          <div className="label">提示词</div>
          <div
            style={{
              background: "var(--bg-input)",
              border: "1px solid var(--border)",
              borderRadius: 4,
              padding: "10px 12px",
              fontSize: 12,
              lineHeight: 1.6,
              color: "var(--text-secondary)",
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
            }}
          >
            {task.prompt || <span style={{ color: "var(--text-muted)" }}>—</span>}
          </div>
        </div>

        {/* Webhook */}
        {task.webhook_config && (
          <InfoRow label="Webhook">
            <span style={{ fontSize: 11 }}>
              {task.webhook_config.platform} ·{" "}
              {task.webhook_config.url.length > 40
                ? `${task.webhook_config.url.slice(0, 40)}…`
                : task.webhook_config.url}
            </span>
          </InfoRow>
        )}

        {/* Next runs */}
        <div>
          <div className="label" style={{ marginBottom: 8 }}>下次运行</div>
          {task.enabled ? (
            <NextRunsPreview cronExpr={task.cron_expression} />
          ) : (
            <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
              任务已禁用
            </span>
          )}
        </div>

        {/* Run history */}
        <div>
          <div className="label" style={{ marginBottom: 8 }}>运行历史</div>
          <RunHistory taskId={task.id} liveRunId={liveRunId} />
        </div>
      </div>
    </div>
  );
}

function InfoRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <div className="label">{label}</div>
      <div style={{ display: "flex", alignItems: "center", flexWrap: "wrap" }}>
        {children}
      </div>
    </div>
  );
}
