import { useState } from "react";
import { X, Plus, Minus } from "lucide-react";
import { NextRunsPreview } from "../scheduler/NextRunsPreview";
import { useTaskStore } from "../../stores/tasks";
import * as api from "../../lib/tauri";
import type {
  Task,
  AiTool,
  CreateTaskRequest,
  UpdateTaskRequest,
  WebhookConfig,
} from "../../lib/types";

interface Props {
  task?: Task;
  onClose: () => void;
}

const defaultWebhook: WebhookConfig = {
  url: "",
  platform: "generic",
  on_start: false,
  on_success: true,
  on_failure: true,
  on_killed: false,
};

export function TaskFormModal({ task, onClose }: Props) {
  const isEdit = !!task;
  const { updateTaskInStore, addTaskToStore } = useTaskStore();

  const [name, setName] = useState(task?.name ?? "");
  const [cronExpr, setCronExpr] = useState(task?.cron_expression ?? "0 9 * * *");
  const [cronHuman, setCronHuman] = useState(task?.cron_human ?? "");
  const [aiTool, setAiTool] = useState<AiTool>(task?.ai_tool ?? "claude");
  const [customCommand, setCustomCommand] = useState(task?.custom_command ?? "");
  const [prompt, setPrompt] = useState(task?.prompt ?? "");
  const [workDir, setWorkDir] = useState(task?.working_directory ?? "");
  const [enabled, setEnabled] = useState(task?.enabled ?? true);
  const [injectContext, setInjectContext] = useState(task?.inject_context ?? false);
  const [restrictNetwork, setRestrictNetwork] = useState(task?.restrict_network ?? false);
  const [restrictFs, setRestrictFs] = useState(task?.restrict_filesystem ?? false);
  const [allowedTools, setAllowedTools] = useState<string[]>(task?.allowed_tools ?? []);
  const [skipPermissions, setSkipPermissions] = useState(task?.skip_permissions ?? false);

  // Env vars
  const [envPairs, setEnvPairs] = useState<{ key: string; value: string }[]>(
    Object.entries(task?.env_vars ?? {}).map(([key, value]) => ({ key, value }))
  );

  // Webhook
  const [webhookEnabled, setWebhookEnabled] = useState(!!task?.webhook_config);
  const [webhook, setWebhook] = useState<WebhookConfig>(
    task?.webhook_config ?? { ...defaultWebhook }
  );

  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  const handleSave = async () => {
    if (!name.trim() || !cronExpr.trim() || !prompt.trim()) {
      setError("名称、Cron 表达式和提示词为必填项。");
      return;
    }
    setSaving(true);
    setError("");

    const envVars: Record<string, string> = {};
    for (const p of envPairs) {
      if (p.key.trim()) envVars[p.key.trim()] = p.value;
    }

    const webhookConfig = webhookEnabled && webhook.url.trim() ? webhook : null;

    try {
      if (isEdit) {
        const req: UpdateTaskRequest = {
          name,
          cron_expression: cronExpr,
          cron_human: cronHuman,
          ai_tool: aiTool,
          custom_command: aiTool === "custom" ? customCommand : undefined,
          prompt,
          working_directory: workDir,
          enabled,
          inject_context: injectContext,
          restrict_network: restrictNetwork,
          restrict_filesystem: restrictFs,
          env_vars: envVars,
          webhook_config: webhookConfig,
          allowed_tools: aiTool === "claude" ? allowedTools : [],
          skip_permissions: aiTool === "claude" ? skipPermissions : false,
        };
        const updated = await api.updateTask(task!.id, req);
        updateTaskInStore(updated);
      } else {
        const req: CreateTaskRequest = {
          name,
          cron_expression: cronExpr,
          cron_human: cronHuman,
          ai_tool: aiTool,
          custom_command: aiTool === "custom" ? customCommand : undefined,
          prompt,
          working_directory: workDir,
          enabled,
          inject_context: injectContext,
          restrict_network: restrictNetwork,
          restrict_filesystem: restrictFs,
          env_vars: envVars,
          webhook_config: webhookConfig ?? undefined,
          allowed_tools: aiTool === "claude" ? allowedTools : [],
          skip_permissions: aiTool === "claude" ? skipPermissions : false,
        };
        const created = await api.createTask(req);
        addTaskToStore(created);
      }
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="modal-overlay">
      <div
        className="modal"
        style={{ width: 620 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <span style={{ fontSize: 13, fontWeight: 600 }}>
            {isEdit ? "编辑任务" : "创建任务"}
          </span>
          <button
            className="btn btn-ghost"
            onClick={onClose}
            style={{ padding: 4 }}
          >
            <X size={14} />
          </button>
        </div>

        <div className="modal-body">
          {/* Name */}
          <div>
            <label className="label">名称</label>
            <input
              className="input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="我的日常任务"
            />
          </div>

          {/* Cron */}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div>
              <label className="label">Cron 表达式</label>
              <input
                className="input"
                value={cronExpr}
                onChange={(e) => setCronExpr(e.target.value)}
                placeholder="0 9 * * *"
              />
            </div>
            <div>
              <label className="label">调度描述</label>
              <input
                className="input"
                value={cronHuman}
                onChange={(e) => setCronHuman(e.target.value)}
                placeholder="每天 09:00"
              />
            </div>
          </div>

          {/* Cron preview */}
          {cronExpr.trim() && (
            <div>
              <label className="label">下次运行预览</label>
              <NextRunsPreview cronExpr={cronExpr} />
            </div>
          )}

          {/* AI Tool */}
          <div>
            <label className="label">AI 工具</label>
            <select
              className="input"
              value={aiTool}
              onChange={(e) => setAiTool(e.target.value as AiTool)}
            >
              <option value="claude">Claude Code CLI (claude -p)</option>
              <option value="custom">自定义命令</option>
            </select>
          </div>

          {/* Custom command template */}
          {aiTool === "custom" && (
            <div>
              <label className="label">
                命令模板{" "}
                <span style={{ textTransform: "none", color: "var(--text-muted)" }}>
                  (use &#123;prompt&#125;, &#123;cwd&#125;, &#123;timestamp&#125;)
                </span>
              </label>
              <input
                className="input"
                value={customCommand}
                onChange={(e) => setCustomCommand(e.target.value)}
                placeholder="echo {prompt}"
              />
            </div>
          )}

          {/* Claude CLI options */}
          {aiTool === "claude" && (
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 8,
                padding: "10px 12px",
                background: "var(--bg-input)",
                border: "1px solid var(--border)",
                borderRadius: 4,
              }}
            >
              <div>
                <label className="label">
                  授权工具{" "}
                  <span style={{ textTransform: "none", color: "var(--text-muted)" }}>
                    (--allowedTools)
                  </span>
                </label>
                <textarea
                  className="input"
                  value={allowedTools.join("\n")}
                  onChange={(e) =>
                    setAllowedTools(
                      e.target.value.split("\n").filter((l) => l.trim() !== "")
                    )
                  }
                  rows={3}
                  placeholder={"Bash(gh *)\nBash(npm *)\nRead"}
                  style={{ fontFamily: "var(--font-mono, monospace)", fontSize: 11 }}
                />
                <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 2 }}>
                  每行一个工具模式，Claude CLI 将自动获得这些工具的使用权限
                </div>
              </div>
              <ToggleRow
                label="跳过权限确认 (--dangerously-skip-permissions)"
                checked={skipPermissions}
                onChange={setSkipPermissions}
              />
            </div>
          )}

          {/* Prompt */}
          <div>
            <label className="label">提示词</label>
            <textarea
              className="input"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              rows={4}
              placeholder="描述 AI 代理应执行的任务..."
            />
          </div>

          {/* Working directory */}
          <div>
            <label className="label">工作目录</label>
            <input
              className="input"
              value={workDir}
              onChange={(e) => setWorkDir(e.target.value)}
              placeholder="/path/to/project"
            />
          </div>

          {/* Toggles */}
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "1fr 1fr",
              gap: "10px 20px",
            }}
          >
            <ToggleRow
              label="启用"
              checked={enabled}
              onChange={setEnabled}
            />
            <ToggleRow
              label="注入上下文"
              checked={injectContext}
              onChange={setInjectContext}
            />
            <ToggleRow
              label="限制网络"
              checked={restrictNetwork}
              onChange={setRestrictNetwork}
            />
            <ToggleRow
              label="限制文件系统"
              checked={restrictFs}
              onChange={setRestrictFs}
            />
          </div>

          {/* Env vars */}
          <div>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                marginBottom: 6,
              }}
            >
              <label className="label" style={{ marginBottom: 0 }}>
                环境变量
              </label>
              <button
                className="btn btn-ghost"
                style={{ fontSize: 10, padding: "2px 8px" }}
                onClick={() =>
                  setEnvPairs([...envPairs, { key: "", value: "" }])
                }
              >
                <Plus size={10} /> 添加
              </button>
            </div>
            {envPairs.map((pair, i) => (
              <div
                key={i}
                style={{
                  display: "flex",
                  gap: 6,
                  marginBottom: 4,
                  alignItems: "center",
                }}
              >
                <input
                  className="input"
                  style={{ width: "35%" }}
                  placeholder="KEY"
                  value={pair.key}
                  onChange={(e) => {
                    const next = [...envPairs];
                    next[i] = { ...next[i], key: e.target.value };
                    setEnvPairs(next);
                  }}
                />
                <input
                  className="input"
                  style={{ flex: 1 }}
                  placeholder="value"
                  value={pair.value}
                  onChange={(e) => {
                    const next = [...envPairs];
                    next[i] = { ...next[i], value: e.target.value };
                    setEnvPairs(next);
                  }}
                />
                <button
                  className="btn btn-ghost"
                  style={{ padding: "2px 4px" }}
                  onClick={() => setEnvPairs(envPairs.filter((_, j) => j !== i))}
                >
                  <Minus size={10} />
                </button>
              </div>
            ))}
          </div>

          {/* Webhook */}
          <div>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                marginBottom: 6,
              }}
            >
              <label className="label" style={{ marginBottom: 0 }}>
                Webhook 通知
              </label>
              <Toggle checked={webhookEnabled} onChange={setWebhookEnabled} />
            </div>
            {webhookEnabled && (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 8,
                  padding: "10px 12px",
                  background: "var(--bg-input)",
                  border: "1px solid var(--border)",
                  borderRadius: 4,
                }}
              >
                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "120px 1fr",
                    gap: 8,
                  }}
                >
                  <div>
                    <label className="label">平台</label>
                    <select
                      className="input"
                      value={webhook.platform}
                      onChange={(e) =>
                        setWebhook({
                          ...webhook,
                          platform: e.target.value as "feishu" | "generic",
                        })
                      }
                    >
                      <option value="generic">通用</option>
                      <option value="feishu">飞书</option>
                    </select>
                  </div>
                  <div>
                    <label className="label">URL</label>
                    <input
                      className="input"
                      value={webhook.url}
                      onChange={(e) =>
                        setWebhook({ ...webhook, url: e.target.value })
                      }
                      placeholder="https://..."
                    />
                  </div>
                </div>
                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr",
                    gap: "6px 16px",
                  }}
                >
                  <ToggleRow
                    label="开始时"
                    checked={webhook.on_start}
                    onChange={(v) =>
                      setWebhook({ ...webhook, on_start: v })
                    }
                  />
                  <ToggleRow
                    label="成功时"
                    checked={webhook.on_success}
                    onChange={(v) =>
                      setWebhook({ ...webhook, on_success: v })
                    }
                  />
                  <ToggleRow
                    label="失败时"
                    checked={webhook.on_failure}
                    onChange={(v) =>
                      setWebhook({ ...webhook, on_failure: v })
                    }
                  />
                  <ToggleRow
                    label="终止时"
                    checked={webhook.on_killed}
                    onChange={(v) =>
                      setWebhook({ ...webhook, on_killed: v })
                    }
                  />
                </div>
              </div>
            )}
          </div>

          {/* Error */}
          {error && (
            <div
              style={{
                fontSize: 11,
                color: "var(--accent-red)",
                padding: "6px 10px",
                background: "#ff444410",
                borderRadius: 4,
              }}
            >
              {error}
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn btn-ghost" onClick={onClose}>
            取消
          </button>
          <button
            className="btn btn-primary"
            onClick={handleSave}
            disabled={saving}
          >
            {saving ? "保存中..." : isEdit ? "保存修改" : "创建任务"}
          </button>
        </div>
      </div>
    </div>
  );
}

function Toggle({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="toggle">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span className="toggle-track" />
    </label>
  );
}

function ToggleRow({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        fontSize: 12,
        color: "var(--text-secondary)",
      }}
    >
      <span>{label}</span>
      <Toggle checked={checked} onChange={onChange} />
    </div>
  );
}
