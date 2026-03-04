import { useEffect, useState } from "react";
import { format } from "date-fns";
import { Clock } from "lucide-react";
import * as api from "../../lib/tauri";

interface Props {
  cronExpr: string;
}

export function NextRunsPreview({ cronExpr }: Props) {
  const [nextRuns, setNextRuns] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!cronExpr) return;
    setError(null);
    api.previewNextRuns(cronExpr, 5)
      .then(setNextRuns)
      .catch((e) => setError(String(e)));
  }, [cronExpr]);

  if (error) {
    return (
      <div style={{ fontSize: 11, color: "var(--accent-red)" }}>
        Invalid cron: {error}
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
          <span>{format(new Date(ts), "MMM d, yyyy  HH:mm")}</span>
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
              next
            </span>
          )}
        </div>
      ))}
    </div>
  );
}
