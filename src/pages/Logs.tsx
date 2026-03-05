import { useState, useEffect, useCallback } from "react";
import { Search, Download, Trash2, RefreshCw } from "lucide-react";
import { ConfirmDialog } from "../components/ui/ConfirmDialog";
import { StatusBadge } from "../components/tasks/StatusBadge";
import { RunLogModal } from "../components/runs/RunLogModal";
import * as api from "../lib/tauri";
import type { Run, RunWithTaskName, RunStatus } from "../lib/types";
import { formatDistanceToNow } from "date-fns";
import { zhCN } from "date-fns/locale";
import { formatDuration } from "../lib/utils";

const LIMIT = 50;
const STATUS_TABS: { label: string; value: string }[] = [
  { label: "全部", value: "" },
  { label: "运行中", value: "running" },
  { label: "成功", value: "success" },
  { label: "失败", value: "failed" },
  { label: "已终止", value: "killed" },
];

export function Logs() {
  const [runs, setRuns] = useState<RunWithTaskName[]>([]);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [selectedRun, setSelectedRun] = useState<Run | null>(null);
  const [showCleanupConfirm, setShowCleanupConfirm] = useState(false);

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
    try {
      const count = await api.cleanupOldRuns();
      await fetchData();
      alert(`已清理 ${count} 条旧记录。`);
    } catch (e) {
      console.error("Cleanup failed:", e);
    }
  };

  const handleExport = async (format: "txt" | "json") => {
    const content =
      format === "json"
        ? JSON.stringify(runs, null, 2)
        : runs
            .map(
              (r) =>
                `[${r.run.status}] ${r.task_name} — ${r.run.started_at}\n` +
                `输出: ${r.run.stdout.slice(0, 500)}\n` +
                `错误: ${r.run.stderr.slice(0, 500)}\n---`
            )
            .join("\n\n");

    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const { writeTextFile } = await import("@tauri-apps/plugin-fs");
      const path = await save({
        defaultPath: `ai-cron-logs.${format}`,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      if (path) {
        await writeTextFile(path, content);
      }
    } catch (e) {
      console.error("Export failed:", e);
    }
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
            placeholder="搜索日志..."
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
        <button className="btn btn-ghost" onClick={() => setShowCleanupConfirm(true)} style={{ fontSize: 10 }}>
          <Trash2 size={10} /> 清理
        </button>
        <button className="btn btn-ghost" onClick={fetchData} style={{ fontSize: 10 }}>
          <RefreshCw size={10} />
        </button>
      </div>

      {/* Table */}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {loading && runs.length === 0 ? (
          <div className="empty-state">加载中...</div>
        ) : runs.length === 0 ? (
          <div className="empty-state">
            <div style={{ fontSize: 12 }}>暂无日志</div>
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
                  状态
                </th>
                <th style={{ padding: "8px 14px", textAlign: "left" }}>
                  任务
                </th>
                <th style={{ padding: "8px 14px", textAlign: "left" }}>
                  开始时间
                </th>
                <th style={{ padding: "8px 14px", textAlign: "left" }}>
                  耗时
                </th>
                <th style={{ padding: "8px 14px", textAlign: "left" }}>
                  触发方式
                </th>
                <th style={{ padding: "8px 14px", textAlign: "right" }}>
                  操作
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
                      locale: zhCN,
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
                      {item.run.triggered_by === "manual" ? "手动" : "计划"}
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
                      日志
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
            ? "无结果"
            : `显示 ${offset + 1}-${offset + runs.length}`}
        </span>
        <div style={{ display: "flex", gap: 6 }}>
          <button
            className="btn btn-ghost"
            style={{ fontSize: 10 }}
            disabled={offset === 0}
            onClick={() => setOffset(Math.max(0, offset - LIMIT))}
          >
            ← 上一页
          </button>
          <button
            className="btn btn-ghost"
            style={{ fontSize: 10 }}
            disabled={runs.length < LIMIT}
            onClick={() => setOffset(offset + LIMIT)}
          >
            下一页 →
          </button>
        </div>
      </div>

      {/* Log modal */}
      {selectedRun && (
        <RunLogModal run={selectedRun} onClose={() => setSelectedRun(null)} />
      )}
      {showCleanupConfirm && (
        <ConfirmDialog
          title="清理日志"
          message="根据保留策略清理旧运行记录？"
          confirmLabel="清理"
          danger
          onConfirm={() => {
            setShowCleanupConfirm(false);
            handleCleanup();
          }}
          onCancel={() => setShowCleanupConfirm(false)}
        />
      )}
    </div>
  );
}
