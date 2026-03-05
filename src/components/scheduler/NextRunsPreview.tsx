import { useEffect, useState } from "react";
import { format } from "date-fns";
import { Clock } from "lucide-react";
import * as api from "../../lib/tauri";

interface Props {
  cronExpr: string;
  timezone?: string;
}

export function NextRunsPreview({ cronExpr, timezone }: Props) {
  const [nextRuns, setNextRuns] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [resolvedTz, setResolvedTz] = useState<string | undefined>(timezone);

  // If no timezone prop, load from settings
  useEffect(() => {
    if (timezone !== undefined) {
      setResolvedTz(timezone);
      return;
    }
    api.getSettings().then((s) => setResolvedTz(s.timezone)).catch(() => {});
  }, [timezone]);

  useEffect(() => {
    if (!cronExpr) return;
    setError(null);
    api.previewNextRuns(cronExpr, 5, resolvedTz)
      .then(setNextRuns)
      .catch((e) => setError(String(e)));
  }, [cronExpr, resolvedTz]);

  if (error) {
    return (
      <div style={{ fontSize: 11, color: "var(--accent-red)" }}>
        无效的 Cron 表达式：{error}
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {nextRuns.map((ts, i) => (
        <div
          key={i}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            fontSize: 11.5,
            color: i === 0 ? "var(--text-primary)" : "var(--text-secondary)",
          }}
        >
          <Clock size={10} style={{ color: "var(--text-muted)" }} />
          <span>{format(new Date(ts), "yyyy年M月d日  HH:mm")}</span>
          {i === 0 && (
            <span
              style={{
                fontSize: 9,
                color: "var(--accent)",
                background: "var(--accent-dim)",
                padding: "1px 5px",
                borderRadius: 3,
              }}
            >
              下次
            </span>
          )}
        </div>
      ))}
    </div>
  );
}
