import type { RunStatus } from "../../lib/types";

interface Props {
  status?: RunStatus | "disabled" | "unknown";
  size?: "sm" | "md";
}

const labels: Record<string, string> = {
  running: "运行中",
  success: "成功",
  failed: "失败",
  killed: "已终止",
  queued: "排队中",
  disabled: "已禁用",
  unknown: "—",
};

export function StatusBadge({ status = "unknown", size = "sm" }: Props) {
  const dotSize = size === "sm" ? 7 : 9;
  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 5,
        fontSize: size === "sm" ? 11 : 12,
        color: "var(--text-secondary)",
      }}
    >
      <span
        className={`status-dot ${status}`}
        style={{ width: dotSize, height: dotSize }}
      />
      {labels[status] ?? status}
    </span>
  );
}
