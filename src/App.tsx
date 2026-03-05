import { useState, useEffect } from "react";
import { Settings as SettingsIcon, Clock, AlignLeft, FileText } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";
import { Dashboard } from "./pages/Dashboard";
import { Settings } from "./pages/Settings";
import { Logs } from "./pages/Logs";
import { ToolStatusBar } from "./components/tools/ToolStatusBar";
import { Toast } from "./components/common/Toast";
import { UpdateChecker } from "./components/updater/UpdateChecker";
import "./index.css";

type Page = "dashboard" | "settings" | "logs";

const appWindow = getCurrentWindow();
const os = platform();
const isMac = os === "macos";
const isWin = os === "windows";

function handleTitleBarMouseDown(e: React.MouseEvent) {
  // Only drag on left mouse button, and not on interactive elements
  if (e.button !== 0) return;
  const target = e.target as HTMLElement;
  if (target.closest("button, a, input, select, textarea, [data-no-drag]")) return;
  e.preventDefault();
  appWindow.startDragging();
}

function handleTitleBarDoubleClick(e: React.MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, a, input, select, textarea, [data-no-drag]")) return;
  appWindow.toggleMaximize();
}

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
        onMouseDown={handleTitleBarMouseDown}
        onDoubleClick={handleTitleBarDoubleClick}
        style={{
          height: 44,
          background: "var(--bg-panel)",
          borderBottom: "1px solid var(--border)",
          display: "flex",
          alignItems: "center",
          padding: "0 16px",
          paddingLeft: isMac ? 78 : 16,
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
            label="任务"
          />
          <NavBtn
            active={page === "logs"}
            onClick={() => setPage("logs")}
            icon={<FileText size={11} />}
            label="日志"
          />
          <NavBtn
            active={page === "settings"}
            onClick={() => setPage("settings")}
            icon={<SettingsIcon size={11} />}
            label="设置"
          />
        </div>

        <div style={{ flex: 1 }} />

        <ToolStatusBar />

        <div
          style={{
            fontSize: 10,
            color: "var(--text-muted)",
            fontFamily: "monospace",
          }}
        >
          v0.1.0
        </div>

        {isWin && <WindowControls />}
      </div>

      {/* Page content */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        {page === "dashboard" && <Dashboard />}
        {page === "logs" && <Logs />}
        {page === "settings" && <Settings />}
      </div>

      {/* Global toast */}
      <Toast />
      <UpdateChecker />
    </div>
  );
}

/* ─── Windows 11 style window controls ─── */

function WindowControls() {
  const appWindow = getCurrentWindow();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    appWindow.isMaximized().then(setMaximized);
    const unlisten = appWindow.onResized(() => {
      appWindow.isMaximized().then(setMaximized);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <div
      style={{
        display: "flex",
        height: "100%",
        marginLeft: 8,
        marginRight: -16, // flush with header right edge
      }}
    >
      <WinBtn onClick={() => appWindow.minimize()} title="最小化">
        <svg width="10" height="1" viewBox="0 0 10 1">
          <rect width="10" height="1" fill="currentColor" />
        </svg>
      </WinBtn>
      <WinBtn
        onClick={() => appWindow.toggleMaximize()}
        title={maximized ? "还原" : "最大化"}
      >
        {maximized ? (
          // Restore icon (overlapping rectangles)
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path
              d="M2 3h5v5H2zM3 3V1h5v5h-2"
              fill="none"
              stroke="currentColor"
              strokeWidth="1"
            />
          </svg>
        ) : (
          // Maximize icon (single rectangle)
          <svg width="10" height="10" viewBox="0 0 10 10">
            <rect
              x="0.5"
              y="0.5"
              width="9"
              height="9"
              fill="none"
              stroke="currentColor"
              strokeWidth="1"
            />
          </svg>
        )}
      </WinBtn>
      <WinBtn onClick={() => appWindow.close()} title="关闭" isClose>
        <svg width="10" height="10" viewBox="0 0 10 10">
          <path d="M1 1l8 8M9 1l-8 8" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      </WinBtn>
    </div>
  );
}

function WinBtn({
  onClick,
  title,
  isClose,
  children,
}: {
  onClick: () => void;
  title: string;
  isClose?: boolean;
  children: React.ReactNode;
}) {
  const [hovered, setHovered] = useState(false);

  return (
    <button
      onClick={onClick}
      title={title}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        width: 46,
        height: "100%",
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        border: "none",
        borderRadius: 0,
        background: hovered
          ? isClose
            ? "#e81123"
            : "rgba(255,255,255,0.06)"
          : "transparent",
        color:
          hovered && isClose ? "#fff" : "var(--text-secondary)",
        cursor: "default",
        padding: 0,
        fontFamily: "inherit",
        transition: "background 0.1s",
      }}
    >
      {children}
    </button>
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
