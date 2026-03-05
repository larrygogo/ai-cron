import { useState, useEffect } from "react";
import { Play, Pencil, Trash2, RefreshCw, Save, Loader } from "lucide-react";
import { useTaskStore } from "../../stores/tasks";
import { StatusBadge } from "./StatusBadge";
import { RunHistory } from "../runs/RunHistory";
import { NextRunsPreview } from "../scheduler/NextRunsPreview";
import { ConfirmDialog } from "../ui/ConfirmDialog";
import type { Task } from "../../lib/types";
import * as api from "../../lib/tauri";

interface Props {
  task: Task;
  onEdit: (task: Task) => void;
  liveRunId?: string;
}

const toolLabels: Record<string, string> = {
  claude: "Claude Code CLI",
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
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [activeTab, setActiveTab] = useState<"detail" | "history">("detail");
  const [editingPlan, setEditingPlan] = useState(false);
  const [planDraft, setPlanDraft] = useState(task.execution_plan || "");
  const [generatingPlan, setGeneratingPlan] = useState(false);
  const [savingPlan, setSavingPlan] = useState(false);

  // Listen for plan_generated events
  useEffect(() => {
    const unlisten = api.onPlanGenerated((taskId) => {
      if (taskId === task.id) {
        api.getTask(task.id).then((updated) => {
          updateTaskInStore(updated);
          setPlanDraft(updated.execution_plan || "");
        });
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [task.id]);

  // Sync plan draft when task changes
  useEffect(() => {
    setPlanDraft(task.execution_plan || "");
    setEditingPlan(false);
  }, [task.execution_plan]);

  const handleGeneratePlan = async () => {
    setGeneratingPlan(true);
    try {
      const plan = await api.generatePlan(task.id);
      setPlanDraft(plan);
      updateTaskInStore({ ...task, execution_plan: plan });
    } catch (e) {
      console.error("Plan generation failed:", e);
    } finally {
      setGeneratingPlan(false);
    }
  };

  const handleSavePlan = async () => {
    setSavingPlan(true);
    try {
      await api.updatePlan(task.id, planDraft);
      updateTaskInStore({ ...task, execution_plan: planDraft });
      setEditingPlan(false);
    } catch (e) {
      console.error("Plan save failed:", e);
    } finally {
      setSavingPlan(false);
    }
  };

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
            onClick={() => setShowDeleteConfirm(true)}
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

        {/* Tab bar */}
        <div style={{
          display: "flex",
          gap: 0,
          borderBottom: "1px solid var(--border)",
          marginBottom: 16,
        }}>
          {(["detail", "history"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              style={{
                padding: "8px 16px",
                fontSize: 12,
                fontWeight: activeTab === tab ? 500 : 400,
                color: activeTab === tab ? "var(--accent)" : "var(--text-muted)",
                background: "none",
                border: "none",
                borderBottom: activeTab === tab ? "2px solid var(--accent)" : "2px solid transparent",
                cursor: "pointer",
                marginBottom: -1,
              }}
            >
              {tab === "detail" ? "详情" : "运行历史"}
            </button>
          ))}
        </div>

        {activeTab === "detail" && (<>
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

        {/* Execution Plan */}
        <div>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
            <div className="label" style={{ marginBottom: 0 }}>执行计划</div>
            <div style={{ display: "flex", gap: 4 }}>
              {editingPlan ? (
                <>
                  <button
                    className="btn btn-ghost"
                    style={{ fontSize: 10, padding: "2px 8px" }}
                    onClick={() => { setEditingPlan(false); setPlanDraft(task.execution_plan || ""); }}
                  >
                    取消
                  </button>
                  <button
                    className="btn btn-primary"
                    style={{ fontSize: 10, padding: "2px 8px" }}
                    onClick={handleSavePlan}
                    disabled={savingPlan}
                  >
                    {savingPlan ? <Loader size={10} className="spin" /> : <Save size={10} />}
                    保存
                  </button>
                </>
              ) : (
                <>
                  <button
                    className="btn btn-ghost"
                    style={{ fontSize: 10, padding: "2px 8px" }}
                    onClick={() => setEditingPlan(true)}
                    disabled={!task.execution_plan}
                  >
                    <Pencil size={10} />
                    编辑
                  </button>
                  <button
                    className="btn btn-ghost"
                    style={{ fontSize: 10, padding: "2px 8px" }}
                    onClick={handleGeneratePlan}
                    disabled={generatingPlan}
                  >
                    {generatingPlan ? <Loader size={10} className="spin" /> : <RefreshCw size={10} />}
                    {task.execution_plan ? "重新生成" : "生成计划"}
                  </button>
                </>
              )}
            </div>
          </div>
          {editingPlan ? (
            <textarea
              className="input"
              style={{ minHeight: 120, fontSize: 11.5, lineHeight: 1.6, fontFamily: "monospace" }}
              value={planDraft}
              onChange={(e) => setPlanDraft(e.target.value)}
            />
          ) : task.execution_plan ? (
            <div
              style={{
                background: "var(--bg-input)",
                border: "1px solid var(--border)",
                borderRadius: 4,
                padding: "10px 12px",
                fontSize: 11.5,
                lineHeight: 1.6,
                color: "var(--text-secondary)",
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
                fontFamily: "monospace",
                maxHeight: 200,
                overflowY: "auto",
              }}
            >
              {task.execution_plan}
            </div>
          ) : (
            <div style={{ fontSize: 11, color: "var(--text-muted)", padding: "8px 0" }}>
              {generatingPlan ? "计划生成中..." : "暂无执行计划，点击「生成计划」自动生成"}
            </div>
          )}
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
        </>)}

        {activeTab === "history" && (
          <RunHistory taskId={task.id} liveRunId={liveRunId} />
        )}
      </div>

      {showDeleteConfirm && (
        <ConfirmDialog
          title="删除任务"
          message={`确定删除任务 "${task.name}"？此操作不可撤销。`}
          confirmLabel="删除"
          danger
          onConfirm={() => {
            setShowDeleteConfirm(false);
            handleDelete();
          }}
          onCancel={() => setShowDeleteConfirm(false)}
        />
      )}
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
