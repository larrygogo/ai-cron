import { useEffect, useRef, useState } from "react";
import { Save, RefreshCw, ExternalLink } from "lucide-react";
import type { AppSettings, ToolInfo } from "../lib/types";
import * as api from "../lib/tauri";
import { openUrl } from "@tauri-apps/plugin-opener";

const PROVIDERS = [
  { value: "claude", label: "Claude (Anthropic)" },
  { value: "openai", label: "OpenAI" },
  { value: "ollama", label: "Ollama (local)" },
] as const;

export function Settings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const savedTimerRef = useRef<ReturnType<typeof setTimeout>>();

  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    return () => {
      if (savedTimerRef.current) clearTimeout(savedTimerRef.current);
    };
  }, []);

  useEffect(() => {
    api.getSettings().then(setSettings).catch((e) => {
      console.error("Failed to load settings:", e);
      setError("Failed to load settings");
    });
    api.detectTools().then(setTools).catch(console.error);
  }, []);

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      await api.updateSettings(settings);
      setSaved(true);
      if (savedTimerRef.current) clearTimeout(savedTimerRef.current);
      savedTimerRef.current = setTimeout(() => setSaved(false), 2000);
    } finally {
      setSaving(false);
    }
  };

  const patch = (updates: Partial<AppSettings>) =>
    setSettings((s) => (s ? { ...s, ...updates } : s));

  if (!settings) {
    return (
      <div className="empty-state">
        <span style={{ fontSize: 12 }}>加载设置...</span>
      </div>
    );
  }

  return (
    <div style={{ flex: 1, overflowY: "auto" }}>
      <div style={{ padding: "24px 32px", maxWidth: 640 }}>
      <h2 style={{ fontSize: 14, marginBottom: 24, color: "var(--text-primary)" }}>
        设置
      </h2>

      {/* AI Tools detected */}
      <Section title="AI 工具">
        {tools.map((tool) => (
          <div
            key={tool.name}
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "8px 0",
              borderBottom: "1px solid var(--border-subtle)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
              <span
                className="status-dot"
                style={{
                  background: tool.available ? "var(--accent)" : "var(--accent-red)",
                  boxShadow: tool.available ? "0 0 5px var(--accent)" : "none",
                }}
              />
              <div>
                <div style={{ fontSize: 12 }}>{tool.label}</div>
                {tool.available && (
                  <div style={{ fontSize: 10, color: "var(--text-muted)" }}>
                    {tool.version} · {tool.path}
                  </div>
                )}
              </div>
            </div>
            {!tool.available && (
              <button
                className="btn btn-ghost"
                style={{ fontSize: 11 }}
                onClick={() => openUrl(tool.install_url)}
              >
                <ExternalLink size={10} /> 安装
              </button>
            )}
          </div>
        ))}
        <button
          className="btn btn-ghost"
          style={{ marginTop: 10, fontSize: 11 }}
          onClick={() => api.detectTools().then(setTools)}
        >
          <RefreshCw size={10} /> 重新检测
        </button>
      </Section>

      {/* NL Provider */}
      <Section title="自然语言服务">
        <div>
          <label className="label">服务商</label>
          <select
            className="input"
            value={settings.nl_provider}
            onChange={(e) =>
              patch({ nl_provider: e.target.value as AppSettings["nl_provider"] })
            }
          >
            {PROVIDERS.map((p) => (
              <option key={p.value} value={p.value}>
                {p.label}
              </option>
            ))}
          </select>
        </div>

        {settings.nl_provider !== "ollama" && (
          <div>
            <label className="label">API Key</label>
            <input
              className="input"
              type="password"
              value={settings.nl_api_key}
              onChange={(e) => patch({ nl_api_key: e.target.value })}
              placeholder="sk-..."
            />
          </div>
        )}

        {(settings.nl_provider === "ollama" || settings.nl_provider === "openai") && (
          <div>
            <label className="label">
              {settings.nl_provider === "ollama" ? "Ollama URL" : "Base URL (optional)"}
            </label>
            <input
              className="input"
              value={settings.nl_base_url}
              onChange={(e) => patch({ nl_base_url: e.target.value })}
              placeholder={
                settings.nl_provider === "ollama"
                  ? "http://localhost:11434"
                  : "https://api.openai.com"
              }
            />
          </div>
        )}

        <div>
          <label className="label">模型 (可选)</label>
          <input
            className="input"
            value={settings.nl_model}
            onChange={(e) => patch({ nl_model: e.target.value })}
            placeholder={
              settings.nl_provider === "claude"
                ? "claude-3-5-haiku-20241022"
                : settings.nl_provider === "openai"
                ? "gpt-4o-mini"
                : "llama3.2"
            }
          />
        </div>
      </Section>

      {/* Log retention */}
      <Section title="日志保留">
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          <div>
            <label className="label">日志保留天数</label>
            <input
              className="input"
              type="number"
              min={1}
              max={365}
              value={settings.log_retention_days}
              onChange={(e) =>
                patch({ log_retention_days: parseInt(e.target.value) || 30 })
              }
            />
          </div>
          <div>
            <label className="label">每任务最大记录数</label>
            <input
              className="input"
              type="number"
              min={10}
              max={1000}
              value={settings.log_retention_per_task}
              onChange={(e) =>
                patch({ log_retention_per_task: parseInt(e.target.value) || 100 })
              }
            />
          </div>
        </div>
      </Section>

      {/* Notifications */}
      <Section title="通知">
        <ToggleRow
          label="成功时通知"
          checked={settings.notify_on_success}
          onChange={(v) => patch({ notify_on_success: v })}
        />
        <ToggleRow
          label="失败时通知"
          checked={settings.notify_on_failure}
          onChange={(v) => patch({ notify_on_failure: v })}
        />
      </Section>

      {/* Save */}
      <div style={{ marginTop: 24 }}>
        <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
          <Save size={11} />
          {saved ? "已保存!" : saving ? "保存中..." : "保存设置"}
        </button>
      </div>
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: 28 }}>
      <div
        style={{
          fontSize: 10,
          color: "var(--text-muted)",
          textTransform: "uppercase",
          letterSpacing: "0.1em",
          marginBottom: 12,
          paddingBottom: 6,
          borderBottom: "1px solid var(--border-subtle)",
        }}
      >
        {title}
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        {children}
      </div>
    </div>
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
        padding: "4px 0",
      }}
    >
      <span style={{ fontSize: 12 }}>{label}</span>
      <label className="toggle">
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => onChange(e.target.checked)}
        />
        <span className="toggle-track" />
      </label>
    </div>
  );
}
