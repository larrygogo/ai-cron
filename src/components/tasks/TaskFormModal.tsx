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
      setError("Name, Cron expression, and Prompt are required.");
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
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal"
        style={{ width: 620 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <span style={{ fontSize: 13, fontWeight: 600 }}>
            {isEdit ? "Edit Task" : "Create Task"}
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
            <label className="label">Name</label>
            <input
              className="input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My daily task"
            />
          </div>

          {/* Cron */}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div>
              <label className="label">Cron Expression</label>
              <input
                className="input"
                value={cronExpr}
                onChange={(e) => setCronExpr(e.target.value)}
                placeholder="0 9 * * *"
              />
            </div>
            <div>
              <label className="label">Human Readable</label>
              <input
                className="input"
                value={cronHuman}
                onChange={(e) => setCronHuman(e.target.value)}
                placeholder="Every day at 9:00"
              />
            </div>
          </div>

          {/* Cron preview */}
          {cronExpr.trim() && (
            <div>
              <label className="label">Next Runs Preview</label>
              <NextRunsPreview cronExpr={cronExpr} />
            </div>
          )}

          {/* AI Tool */}
          <div>
            <label className="label">AI Tool</label>
            <select
              className="input"
              value={aiTool}
              onChange={(e) => setAiTool(e.target.value as AiTool)}
            >
              <option value="claude">Claude (claude -p)</option>
              <option value="opencode">OpenCode</option>
              <option value="codex">Codex (full-auto)</option>
              <option value="custom">Custom Command</option>
            </select>
          </div>

          {/* Custom command template */}
          {aiTool === "custom" && (
            <div>
              <label className="label">
                Command Template{" "}
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

          {/* Prompt */}
          <div>
            <label className="label">Prompt</label>
            <textarea
              className="input"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              rows={4}
              placeholder="Describe what the AI agent should do..."
            />
          </div>

          {/* Working directory */}
          <div>
            <label className="label">Working Directory</label>
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
              label="Enabled"
              checked={enabled}
              onChange={setEnabled}
            />
            <ToggleRow
              label="Inject Context"
              checked={injectContext}
              onChange={setInjectContext}
            />
            <ToggleRow
              label="Restrict Network"
              checked={restrictNetwork}
              onChange={setRestrictNetwork}
            />
            <ToggleRow
              label="Restrict Filesystem"
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
                Environment Variables
              </label>
              <button
                className="btn btn-ghost"
                style={{ fontSize: 10, padding: "2px 8px" }}
                onClick={() =>
                  setEnvPairs([...envPairs, { key: "", value: "" }])
                }
              >
                <Plus size={10} /> Add
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
                Webhook Notification
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
                    <label className="label">Platform</label>
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
                      <option value="generic">Generic</option>
                      <option value="feishu">Feishu</option>
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
                    label="On Start"
                    checked={webhook.on_start}
                    onChange={(v) =>
                      setWebhook({ ...webhook, on_start: v })
                    }
                  />
                  <ToggleRow
                    label="On Success"
                    checked={webhook.on_success}
                    onChange={(v) =>
                      setWebhook({ ...webhook, on_success: v })
                    }
                  />
                  <ToggleRow
                    label="On Failure"
                    checked={webhook.on_failure}
                    onChange={(v) =>
                      setWebhook({ ...webhook, on_failure: v })
                    }
                  />
                  <ToggleRow
                    label="On Killed"
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
            Cancel
          </button>
          <button
            className="btn btn-primary"
            onClick={handleSave}
            disabled={saving}
          >
            {saving ? "Saving..." : isEdit ? "Save Changes" : "Create Task"}
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
