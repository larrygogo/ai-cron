import type { Task } from "../../lib/types";
import { StatusBadge } from "./StatusBadge";

interface Props {
  task: Task;
  selected: boolean;
  onClick: () => void;
}

const toolColors: Record<string, string> = {
  claude: "#cc785c",
  opencode: "#4488ff",
  codex: "#00a67e",
  custom: "#888",
};

export function TaskItem({ task, selected, onClick }: Props) {
  const statusKey = !task.enabled
    ? "disabled"
    : task.last_run_status ?? "unknown";

  return (
    <div
      onClick={onClick}
      style={{
        padding: "8px 14px",
        cursor: "pointer",
        background: selected ? "var(--bg-selected)" : "transparent",
        borderLeft: selected
          ? "2px solid var(--accent)"
          : "2px solid transparent",
        display: "flex",
        flexDirection: "column",
        gap: 3,
        transition: "background 0.1s",
        userSelect: "none",
      }}
      onMouseEnter={(e) => {
        if (!selected)
          (e.currentTarget as HTMLDivElement).style.background =
            "var(--bg-hover)";
      }}
      onMouseLeave={(e) => {
        if (!selected)
          (e.currentTarget as HTMLDivElement).style.background = "transparent";
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 8,
        }}
      >
        <span
          style={{
            fontSize: 12.5,
            color: task.enabled
              ? "var(--text-primary)"
              : "var(--text-muted)",
            fontWeight: selected ? 500 : 400,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {task.name}
        </span>
        <span
          style={{
            fontSize: 10,
            color: toolColors[task.ai_tool] ?? "#888",
            flexShrink: 0,
            opacity: 0.8,
          }}
        >
          {task.ai_tool}
        </span>
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <StatusBadge status={statusKey as Parameters<typeof StatusBadge>[0]["status"]} />
        <span style={{ fontSize: 10, color: "var(--text-muted)" }}>
          {task.cron_human || task.cron_expression}
        </span>
      </div>
    </div>
  );
}
