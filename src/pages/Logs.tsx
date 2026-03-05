import { useState, useEffect, useCallback } from "react";
import { Search, Download, Trash2, RefreshCw } from "lucide-react";
import { StatusBadge } from "../components/tasks/StatusBadge";
import { RunLogModal } from "../components/runs/RunLogModal";
import * as api from "../lib/tauri";
import type { Run, RunWithTaskName, RunStatus } from "../lib/types";
import { formatDistanceToNow } from "date-fns";
import { formatDuration } from "../lib/utils";

const LIMIT = 50;
const STATUS_TABS: { label: string; value: string }[] = [
  { label: "All", value: "" },
  { label: "Running", value: "running" },
  { label: "Success", value: "success" },
  { label: "Failed", value: "failed" },
  { label: "Killed", value: "killed" },
];

export function Logs() {
  const [runs, setRuns] = useState<RunWithTaskName[]>([]);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [selectedRun, setSelectedRun] = useState<Run | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.getAllRuns({
        limit: LIMIT,
        offset,
        statusFilter: statusFilter || undefined,
        searchQuery: search || undefined,
      });
      setRuns(data);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, [search, statusFilter, offset]);

  // Debounced search
  useEffect(() => {
    const timer = setTimeout(fetchData, 300);
    return () => clearTimeout(timer);
  }, [fetchData]);

  const handleCleanup = async () => {
    if (!confirm("Clean up old runs based on retention settings?")) return;
    try {
      const count = await api.cleanupOldRuns();
      await fetchData();
      alert(`Cleaned up ${count} old runs.`);
    } catch (e) {
      console.error("Cleanup failed:", e);
    }
  };

  const handleExport = (format: "txt" | "json") => {
    let content: string;
    if (format === "json") {
      content = JSON.stringify(runs, null, 2);
    } else {
      content = runs
        .map(
          (r) =>
            `[${r.run.status}] ${r.task_name} — ${r.run.started_at}\n` +
            `stdout: ${r.run.stdout.slice(0, 500)}\n` +
            `stderr: ${r.run.stderr.slice(0, 500)}\n---`
        )
        .join("\n\n");
    }
    const blob = new Blob([content], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `ai-cron-logs.${format}`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        height: "100%",
        overflow: "hidden",
      }}
    >
      {/* Toolbar */}
      <div
        style={{
          padding: "12px 20px",
          borderBottom: "1px solid var(--border)",
          display: "flex",
          alignItems: "center",
          gap: 10,
          flexShrink: 0,
        }}
      >
        {/* Search */}
        <div style={{ position: "relative", flex: 1, maxWidth: 320 }}>
          <Search
            size={12}
            style={{
              position: "absolute",
              left: 8,
              top: "50%",
              transform: "translateY(-50%)",
              color: "var(--text-muted)",
            }}
          />
          <input
            className="input"
            style={{ paddingLeft: 26 }}
            placeholder="Search logs..."
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setOffset(0);
            }}
          />
        </div>

        {/* Status tabs */}
        <div style={{ display: "flex", gap: 2 }}>
          {STATUS_TABS.map((tab) => (
            <button
              key={tab.value}
              className="btn btn-ghost"
              style={{
                fontSize: 10,
                padding: "3px 8px",
                background:
                  statusFilter === tab.value
                    ? "var(--bg-selected)"
                    : undefined,
                color:
                  statusFilter === tab.value
                    ? "var(--text-primary)"
                    : undefined,
              }}
              onClick={() => {
                setStatusFilter(tab.value);
                setOffset(0);
              }}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div style={{ flex: 1 }} />

        {/* Actions */}
        <button className="btn btn-ghost" onClick={() => handleExport("json")} style={{ fontSize: 10 }}>
          <Download size={10} /> JSON
        </button>
        <button className="btn btn-ghost" onClick={() => handleExport("txt")} style={{ fontSize: 10 }}>
          <Download size={10} /> TXT
        </button>
        <button className="btn btn-ghost" onClick={handleCleanup} style={{ fontSize: 10 }}>
          <Trash2 size={10} /> Cleanup
        </button>
        <button className="btn btn-ghost" onClick={fetchData} style={{ fontSize: 10 }}>
          <RefreshCw size={10} />
        </button>
      </div>

      {/* Table */}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {loading && runs.length === 0 ? (
          <div className="empty-state">Loading...</div>
        ) : runs.length === 0 ? (
          <div className="empty-state">
            <div style={{ fontSize: 12 }}>No logs found</div>
          </div>
        ) : (
          <table
            style={{
              width: "100%",
              borderCollapse: "collapse",
              fontSize: 11.5,
            }}
          >
            <thead>
              <tr
                style={{
                  borderBottom: "1px solid var(--border)",
                  color: "var(--text-muted)",
                  fontSize: 10,
                  textTransform: "uppercase",
                  letterSpacing: "0.06em",
                }}
              >
                <th style={{ padding: "8px 14px", textAlign: "left" }}>
                  Status
                </th>
                <th style={{ padding: "8px 14px", textAlign: "left" }}>
                  Task
                </th>
                <th style={{ padding: "8px 14px", textAlign: "left" }}>
                  Started
                </th>
                <th style={{ padding: "8px 14px", textAlign: "left" }}>
                  Duration
                </th>
                <th style={{ padding: "8px 14px", textAlign: "left" }}>
                  Trigger
                </th>
                <th style={{ padding: "8px 14px", textAlign: "right" }}>
                  Actions
                </th>
              </tr>
            </thead>
            <tbody>
              {runs.map((item) => (
                <tr
                  key={item.run.id}
                  style={{
                    borderBottom: "1px solid var(--border-subtle)",
                    cursor: "pointer",
                  }}
                  onMouseEnter={(e) =>
                    (e.currentTarget.style.background = "var(--bg-hover)")
                  }
                  onMouseLeave={(e) =>
                    (e.currentTarget.style.background = "transparent")
                  }
                >
                  <td style={{ padding: "6px 14px" }}>
                    <StatusBadge
                      status={item.run.status as RunStatus}
                      size="sm"
                    />
                  </td>
                  <td
                    style={{
                      padding: "6px 14px",
                      color: "var(--text-primary)",
                      fontWeight: 500,
                    }}
                  >
                    {item.task_name}
                  </td>
                  <td
                    style={{
                      padding: "6px 14px",
                      color: "var(--text-secondary)",
                    }}
                  >
                    {formatDistanceToNow(new Date(item.run.started_at), {
                      addSuffix: true,
                    })}
                  </td>
                  <td
                    style={{
                      padding: "6px 14px",
                      color: "var(--text-secondary)",
                    }}
                  >
                    {formatDuration(item.run.duration_ms)}
                  </td>
                  <td style={{ padding: "6px 14px" }}>
                    <span
                      style={{
                        fontSize: 10,
                        padding: "1px 6px",
                        borderRadius: 3,
                        background:
                          item.run.triggered_by === "manual"
                            ? "var(--accent-dim)"
                            : "transparent",
                        color:
                          item.run.triggered_by === "manual"
                            ? "var(--accent)"
                            : "var(--text-muted)",
                        border: `1px solid ${
                          item.run.triggered_by === "manual"
                            ? "var(--accent)"
                            : "var(--border)"
                        }`,
                      }}
                    >
                      {item.run.triggered_by}
                    </span>
                  </td>
                  <td
                    style={{
                      padding: "6px 14px",
                      textAlign: "right",
                    }}
                  >
                    <button
                      className="btn btn-ghost"
                      style={{ fontSize: 10, padding: "2px 8px" }}
                      onClick={() => setSelectedRun(item.run)}
                    >
                      log
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Pagination */}
      <div
        style={{
          padding: "8px 20px",
          borderTop: "1px solid var(--border)",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          fontSize: 11,
          color: "var(--text-muted)",
          flexShrink: 0,
        }}
      >
        <span>
          {runs.length === 0
            ? "No results"
            : `Showing ${offset + 1}-${offset + runs.length}`}
        </span>
        <div style={{ display: "flex", gap: 6 }}>
          <button
            className="btn btn-ghost"
            style={{ fontSize: 10 }}
            disabled={offset === 0}
            onClick={() => setOffset(Math.max(0, offset - LIMIT))}
          >
            ← Prev
          </button>
          <button
            className="btn btn-ghost"
            style={{ fontSize: 10 }}
            disabled={runs.length < LIMIT}
            onClick={() => setOffset(offset + LIMIT)}
          >
            Next →
          </button>
        </div>
      </div>

      {/* Log modal */}
      {selectedRun && (
        <RunLogModal run={selectedRun} onClose={() => setSelectedRun(null)} />
      )}
    </div>
  );
}
