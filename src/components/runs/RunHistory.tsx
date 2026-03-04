import { useEffect, useState } from "react";
import { format } from "date-fns";
import { Play, Terminal } from "lucide-react";
import { StatusBadge } from "../tasks/StatusBadge";
import { RunLogModal } from "./RunLogModal";
import type { Run } from "../../lib/types";
import * as api from "../../lib/tauri";

interface Props {
  taskId: string;
  liveRunId?: string;
}

function formatDuration(ms?: number): string {
  if (!ms) return "-";
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  return `${Math.floor(s / 60)}m${s % 60}s`;
}

export function RunHistory({ taskId, liveRunId }: Props) {
  const [runs, setRuns] = useState<Run[]>([]);
  const [loading, setLoading] = useState(true);
  const [viewRun, setViewRun] = useState<Run | null>(null);

  useEffect(() => {
    setLoading(true);
    api.getRuns(taskId, 50).then((r) => {
      setRuns(r);
      setLoading(false);
    });
  }, [taskId, liveRunId]);

  if (loading) {
    return (
      <div style={{ padding: "20px 0", color: "var(--text-muted)", fontSize: 12 }}>
        Loading history...
      </div>
    );
  }

  if (runs.length === 0) {
    return (
      <div className="empty-state" style={{ padding: "24px 0" }}>
        <Play size={20} />
        <span style={{ fontSize: 11 }}>No runs yet</span>
      </div>
    );
  }

  return (
    <>
      <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
        {runs.map((run) => {
          const isLive = run.id === liveRunId;
          return (
            <div
              key={run.id}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 10,
                padding: "7px 0",
                borderBottom: "1px solid var(--border-subtle)",
                fontSize: 11.5,
              }}
            >
              <StatusBadge
                status={isLive ? "running" : run.status}
              />
              <span style={{ color: "var(--text-secondary)", minWidth: 120 }}>
                {format(new Date(run.started_at), "MMM d, HH:mm:ss")}
              </span>
              <span
                style={{ color: "var(--text-muted)", minWidth: 40 }}
              >
                {isLive ? "live" : formatDuration(run.duration_ms)}
              </span>
              <span
                style={{
                  color:
                    run.triggered_by === "manual"
                      ? "var(--accent-blue)"
                      : "var(--text-muted)",
                  fontSize: 10,
                  flexShrink: 0,
                }}
              >
                {run.triggered_by}
              </span>
              <span style={{ flex: 1 }} />
              <button
                className="btn btn-ghost"
                style={{ padding: "2px 7px", fontSize: 10 }}
                onClick={() => setViewRun(run)}
                title="View log"
              >
                <Terminal size={10} />
                log
              </button>
            </div>
          );
        })}
      </div>

      {viewRun && (
        <RunLogModal
          run={viewRun}
          onClose={() => setViewRun(null)}
        />
      )}
    </>
  );
}
