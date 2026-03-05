import { useEffect, useState } from "react";
import { format } from "date-fns";
import { Play, Terminal } from "lucide-react";
import { StatusBadge } from "../tasks/StatusBadge";
import { RunLogModal } from "./RunLogModal";
import type { Run } from "../../lib/types";
import { formatDuration } from "../../lib/utils";
import * as api from "../../lib/tauri";

interface Props {
  taskId: string;
  liveRunId?: string;
}

export function RunHistory({ taskId, liveRunId }: Props) {
  const [runs, setRuns] = useState<Run[]>([]);
  const [loading, setLoading] = useState(true);
  const [viewRun, setViewRun] = useState<Run | null>(null);

  useEffect(() => {
    setLoading(true);
    api.getRuns(taskId, 50)
      .then((r) => {
        setRuns(r);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [taskId, liveRunId]);

  if (loading) {
    return (
      <div style={{ padding: "20px 0", color: "var(--text-muted)", fontSize: 12 }}>
        加载历史...
      </div>
    );
  }

  if (runs.length === 0) {
    return (
      <div className="empty-state" style={{ padding: "24px 0" }}>
        <Play size={20} />
        <span style={{ fontSize: 11 }}>暂无运行记录</span>
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
                {format(new Date(run.started_at), "M月d日 HH:mm:ss")}
              </span>
              <span
                style={{ color: "var(--text-muted)", minWidth: 40 }}
              >
                {isLive ? "实时" : formatDuration(run.duration_ms)}
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
                {run.triggered_by === "manual" ? "手动" : "计划"}
              </span>
              {run.goal_evaluation && (() => {
                try {
                  const evalData = JSON.parse(run.goal_evaluation);
                  return (
                    <span style={{
                      fontSize: 10,
                      color: evalData.passed ? "var(--accent)" : "#e8a838",
                      flexShrink: 0,
                    }}>
                      {evalData.passed ? "✓ 目标达成" : "⚠ 目标未达成"}
                    </span>
                  );
                } catch { return null; }
              })()}
              <span style={{ flex: 1 }} />
              <button
                className="btn btn-ghost"
                style={{ padding: "2px 7px", fontSize: 10 }}
                onClick={() => setViewRun(run)}
                title="查看日志"
              >
                <Terminal size={10} />
                日志
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
