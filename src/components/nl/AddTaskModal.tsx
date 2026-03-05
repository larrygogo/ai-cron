import { useState } from "react";
import { Sparkles, X, ChevronRight, Loader } from "lucide-react";
import type { AiTool, CreateTaskRequest, TaskDraft } from "../../lib/types";
import { useTaskStore } from "../../stores/tasks";
import * as api from "../../lib/tauri";

interface Props {
  onClose: () => void;
}

const defaultDraft: TaskDraft = {
  name: "",
  cron_expression: "0 9 * * *",
  cron_human: "每天 09:00",
  prompt: "",
  ai_tool: "claude",
  suggested_directory: "~/",
};

const AI_TOOLS: AiTool[] = ["claude", "opencode", "codex", "custom"];

export function AddTaskModal({ onClose }: Props) {
  const [step, setStep] = useState<"nl" | "confirm">("nl");
  const [nlInput, setNlInput] = useState("");
  const [parsing, setParsing] = useState(false);
  const [parseError, setParseError] = useState<string | null>(null);
  const [draft, setDraft] = useState<TaskDraft>(defaultDraft);
  const [saving, setSaving] = useState(false);
  const { addTaskToStore } = useTaskStore();

  const handleParse = async () => {
    if (!nlInput.trim()) return;
    setParsing(true);
    setParseError(null);
    try {
      const result = await api.parseNlToTask(nlInput);
      setDraft(result);
      setStep("confirm");
    } catch (e) {
      setParseError(String(e));
    } finally {
      setParsing(false);
    }
  };

  const handleSkipToManual = () => {
    setStep("confirm");
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const req: CreateTaskRequest = {
        name: draft.name || "未命名任务",
        cron_expression: draft.cron_expression,
        cron_human: draft.cron_human,
        ai_tool: draft.ai_tool,
        prompt: draft.prompt,
        working_directory: draft.suggested_directory,
        enabled: true,
        inject_context: false,
        restrict_network: false,
        restrict_filesystem: false,
        env_vars: {},
      };
      const task = await api.createTask(req);
      addTaskToStore(task);
      onClose();
    } catch (e) {
      console.error(e);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Sparkles size={14} style={{ color: "var(--accent)" }} />
            <span style={{ fontSize: 13 }}>
              {step === "nl" ? "添加任务" : "确认任务"}
            </span>
          </div>
          <button className="btn btn-ghost" style={{ padding: "3px 8px" }} onClick={onClose}>
            <X size={12} />
          </button>
        </div>

        {/* Step: Natural language */}
        {step === "nl" && (
          <>
            <div className="modal-body">
              <p style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.6 }}>
                用自然语言描述你的任务，AI 将自动生成调度计划和提示词。
              </p>
              <div>
                <label className="label">需要自动化什么？</label>
                <textarea
                  className="input"
                  style={{ minHeight: 100 }}
                  placeholder='例如："每个工作日早上 9 点，检查主分支的失败测试并修复"'
                  value={nlInput}
                  onChange={(e) => setNlInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) handleParse();
                  }}
                  autoFocus
                />
                {parseError && (
                  <div style={{ fontSize: 11, color: "var(--accent-red)", marginTop: 6 }}>
                    {parseError}
                  </div>
                )}
              </div>
            </div>
            <div className="modal-footer">
              <button className="btn btn-ghost" onClick={handleSkipToManual}>
                手动配置
              </button>
              <button
                className="btn btn-primary"
                onClick={handleParse}
                disabled={!nlInput.trim() || parsing}
              >
                {parsing ? <Loader size={11} className="spin" /> : <Sparkles size={11} />}
                {parsing ? "解析中..." : "生成任务"}
              </button>
            </div>
          </>
        )}

        {/* Step: Confirm / edit draft */}
        {step === "confirm" && (
          <>
            <div className="modal-body">
              {/* Name */}
              <div>
                <label className="label">任务名称</label>
                <input
                  className="input"
                  value={draft.name}
                  onChange={(e) => setDraft({ ...draft, name: e.target.value })}
                  placeholder="Task name"
                  autoFocus
                />
              </div>

              {/* Cron */}
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
                <div>
                  <label className="label">Cron 表达式</label>
                  <input
                    className="input"
                    value={draft.cron_expression}
                    onChange={(e) => setDraft({ ...draft, cron_expression: e.target.value })}
                    placeholder="0 9 * * *"
                  />
                </div>
                <div>
                  <label className="label">调度描述</label>
                  <input
                    className="input"
                    value={draft.cron_human}
                    onChange={(e) => setDraft({ ...draft, cron_human: e.target.value })}
                    placeholder="每天 09:00"
                  />
                </div>
              </div>

              {/* AI Tool */}
              <div>
                <label className="label">AI 工具</label>
                <select
                  className="input"
                  value={draft.ai_tool}
                  onChange={(e) => setDraft({ ...draft, ai_tool: e.target.value as AiTool })}
                >
                  {AI_TOOLS.map((t) => (
                    <option key={t} value={t}>{t}</option>
                  ))}
                </select>
              </div>

              {/* Prompt */}
              <div>
                <label className="label">提示词</label>
                <textarea
                  className="input"
                  style={{ minHeight: 100 }}
                  value={draft.prompt}
                  onChange={(e) => setDraft({ ...draft, prompt: e.target.value })}
                  placeholder="AI 代理的任务描述..."
                />
              </div>

              {/* Working directory */}
              <div>
                <label className="label">工作目录</label>
                <input
                  className="input"
                  value={draft.suggested_directory}
                  onChange={(e) => setDraft({ ...draft, suggested_directory: e.target.value })}
                  placeholder="~/projects/my-app"
                />
              </div>
            </div>

            <div className="modal-footer">
              <button className="btn btn-ghost" onClick={() => setStep("nl")}>
                返回
              </button>
              <button
                className="btn btn-primary"
                onClick={handleSave}
                disabled={saving || !draft.name.trim()}
              >
                {saving ? "保存中..." : "创建任务"}
                <ChevronRight size={11} />
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
