import { useState } from "react";
import { Settings as SettingsIcon, Clock, AlignLeft } from "lucide-react";
import { Dashboard } from "./pages/Dashboard";
import { Settings } from "./pages/Settings";
import "./index.css";

type Page = "dashboard" | "settings";

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        width: "100vw",
        overflow: "hidden",
        background: "var(--bg-base)",
      }}
    >
      {/* Title bar / nav */}
      <div
        data-tauri-drag-region
        style={{
          height: 44,
          background: "var(--bg-panel)",
          borderBottom: "1px solid var(--border)",
          display: "flex",
          alignItems: "center",
          padding: "0 16px",
          flexShrink: 0,
          userSelect: "none",
        }}
      >
        {/* Logo */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 7,
            marginRight: 24,
          }}
        >
          <Clock size={14} style={{ color: "var(--accent)" }} />
          <span
            style={{
              fontSize: 12.5,
              fontWeight: 600,
              color: "var(--text-primary)",
              letterSpacing: "0.02em",
            }}
          >
            AI Cron
          </span>
        </div>

        {/* Nav */}
        <div style={{ display: "flex", gap: 2 }}>
          <NavBtn
            active={page === "dashboard"}
            onClick={() => setPage("dashboard")}
            icon={<AlignLeft size={11} />}
            label="Tasks"
          />
          <NavBtn
            active={page === "settings"}
            onClick={() => setPage("settings")}
            icon={<SettingsIcon size={11} />}
            label="Settings"
          />
        </div>

        <div style={{ flex: 1 }} data-tauri-drag-region />

        <div
          style={{
            fontSize: 10,
            color: "var(--text-muted)",
            fontFamily: "monospace",
          }}
        >
          v0.1.0
        </div>
      </div>

      {/* Page content */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        {page === "dashboard" && <Dashboard />}
        {page === "settings" && <Settings />}
      </div>
    </div>
  );
}

function NavBtn({
  active,
  onClick,
  icon,
  label,
}: {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 5,
        padding: "4px 10px",
        borderRadius: 4,
        border: "none",
        background: active ? "var(--bg-selected)" : "transparent",
        color: active ? "var(--text-primary)" : "var(--text-secondary)",
        fontSize: 11,
        cursor: "pointer",
        fontFamily: "inherit",
        transition: "background 0.1s",
      }}
      onMouseEnter={(e) => {
        if (!active)
          (e.currentTarget as HTMLButtonElement).style.background =
            "var(--bg-hover)";
      }}
      onMouseLeave={(e) => {
        if (!active)
          (e.currentTarget as HTMLButtonElement).style.background = "transparent";
      }}
    >
      {icon}
      {label}
    </button>
  );
}
