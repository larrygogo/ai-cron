import type { RunStatus } from "../../lib/types";

interface Props {
  status?: RunStatus | "disabled" | "unknown";
  size?: "sm" | "md";
}

const labels: Record<string, string> = {
  running: "running",
  success: "success",
  failed: "failed",
  killed: "killed",
  queued: "queued",
  disabled: "disabled",
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
