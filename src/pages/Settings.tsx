import { useEffect, useRef, useState } from "react";
import { Save, RefreshCw, ExternalLink, Wrench } from "lucide-react";
import type { AppSettings, McpStatus, ToolInfo } from "../lib/types";
import * as api from "../lib/tauri";
import { openUrl } from "@tauri-apps/plugin-opener";

const COMMON_TIMEZONES = [
  "Asia/Shanghai",
  "Asia/Tokyo",
  "Asia/Seoul",
  "Asia/Singapore",
  "Asia/Hong_Kong",
  "Asia/Taipei",
  "Asia/Kolkata",
  "Asia/Dubai",
  "Europe/London",
  "Europe/Paris",
  "Europe/Berlin",
  "Europe/Moscow",
  "America/New_York",
  "America/Chicago",
  "America/Denver",
  "America/Los_Angeles",
  "America/Sao_Paulo",
  "Pacific/Auckland",
  "Australia/Sydney",
  "UTC",
];

export function Settings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [systemTz, setSystemTz] = useState<string>("");
  const [mcpStatus, setMcpStatus] = useState<McpStatus | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [repairMsg, setRepairMsg] = useState<string>("");
  const savedTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);


  useEffect(() => {
    return () => {
      if (savedTimerRef.current) clearTimeout(savedTimerRef.current);
    };
  }, []);

  useEffect(() => {
    api.getSettings().then(setSettings).catch((e) => {
      console.error("Failed to load settings:", e);
    });
    api.detectTools().then(setTools).catch(console.error);
    api.getSystemTimezone().then(setSystemTz).catch(console.error);
    api.getMcpStatus().then(setMcpStatus).catch(console.error);
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

  const handleRepairMcp = async () => {
    try {
      const msg = await api.repairMcpConfig();
      setRepairMsg(msg);
      setTimeout(() => setRepairMsg(""), 3000);
    } catch (e) {
      setRepairMsg(String(e));
      setTimeout(() => setRepairMsg(""), 3000);
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

      {/* Timezone */}
      <Section title="时区">
        <div>
          <label className="label">调度时区</label>
          <select
            className="input"
            value={settings.timezone}
            onChange={(e) => patch({ timezone: e.target.value })}
          >
            <option value="system">
              跟随系统{systemTz ? ` (${systemTz})` : ""}
            </option>
            {COMMON_TIMEZONES.map((tz) => (
              <option key={tz} value={tz}>
                {tz}
              </option>
            ))}
          </select>
          <div
            style={{
              fontSize: 10,
              color: "var(--text-muted)",
              marginTop: 4,
              lineHeight: 1.5,
            }}
          >
            Cron 表达式将基于此时区匹配。例如设置 Asia/Shanghai 后，
            "0 9 * * *" 表示北京时间 09:00 触发。
          </div>
        </div>
      </Section>

      {/* MCP Server */}
      <Section title="MCP Server">
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "8px 0",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <span
              className="status-dot"
              style={{
                background: mcpStatus?.running ? "var(--accent)" : "var(--accent-red)",
                boxShadow: mcpStatus?.running ? "0 0 5px var(--accent)" : "none",
              }}
            />
            <div>
              <div style={{ fontSize: 12 }}>
                {mcpStatus?.running
                  ? `运行中 (端口 ${mcpStatus.port})`
                  : "未运行"}
              </div>
              <div style={{ fontSize: 10, color: "var(--text-muted)" }}>
                已自动配置到 ~/.claude.json
              </div>
            </div>
          </div>
          <button
            className="btn btn-ghost"
            style={{ fontSize: 11 }}
            onClick={handleRepairMcp}
          >
            <Wrench size={10} /> 修复配置
          </button>
        </div>
        {repairMsg && (
          <div style={{ fontSize: 11, color: "var(--text-secondary)", padding: "4px 0" }}>
            {repairMsg}
          </div>
        )}
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
