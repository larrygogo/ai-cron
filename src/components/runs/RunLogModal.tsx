import { useEffect, useRef, useState } from "react";
import { X, Download } from "lucide-react";
import AnsiToHtml from "ansi-to-html";
import { format } from "date-fns";
import type { Run } from "../../lib/types";
import { formatDuration } from "../../lib/utils";
import { useRunStore } from "../../stores/runs";

const ansiConverter = new AnsiToHtml({
  fg: "#e8e8e8",
  bg: "#080808",
  newline: true,
  escapeXML: true,
});

interface Props {
  run: Run;
  onClose: () => void;
}

export function RunLogModal({ run, onClose }: Props) {
  const liveOutput = useRunStore((s) => s.liveOutput[run.id]);
  const [activeTab, setActiveTab] = useState<"stdout" | "stderr">("stdout");
  const logRef = useRef<HTMLDivElement>(null);
  const isLive = run.status === "running";

  // Scroll to bottom on new output
  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [liveOutput, activeTab]);

  const stdout = isLive ? (liveOutput?.stdout ?? "") : run.stdout;
  const rawStderr = isLive ? (liveOutput?.stderr ?? "") : run.stderr;
  const activeContent = activeTab === "stdout" ? stdout : rawStderr;

  const renderHtml = (text: string) => {
    try {
      let html = ansiConverter.toHtml(text);
      // Style [ai-cron] phase lines differently
      html = html.replace(
        /^(\[ai-cron\].*)$/gm,
        '<span style="color: #888; font-style: italic;">$1</span>'
      );
      return html;
    } catch {
      return text.replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }
  };

  const tabLabels: Record<typeof activeTab, string> = { stdout: "结果", stderr: "过程" };

  const handleDownload = () => {
    const content = `=== 结果 ===\n${stdout}\n\n=== 过程 ===\n${rawStderr}`;
    const blob = new Blob([content], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `run-${run.id.slice(0, 8)}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal"
        style={{ width: 760, maxHeight: "85vh" }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            <span style={{ fontSize: 12, color: "var(--text-primary)" }}>
              运行日志
            </span>
            <span
              style={{
                fontSize: 10,
                color: "var(--text-muted)",
                fontFamily: "monospace",
              }}
            >
              {run.id.slice(0, 8)}
            </span>
            <span
              style={{
                fontSize: 10,
                padding: "2px 6px",
                borderRadius: 3,
                background:
                  run.status === "success"
                    ? "var(--accent-dim)"
                    : run.status === "failed"
                    ? "#ff444420"
                    : "var(--border)",
                color:
                  run.status === "success"
                    ? "var(--accent)"
                    : run.status === "failed"
                    ? "var(--accent-red)"
                    : "var(--text-secondary)",
              }}
            >
              {isLive ? "● 实时" : run.status}
            </span>
            {run.goal_evaluation && (() => {
              try {
                const evalData = JSON.parse(run.goal_evaluation);
                return (
                  <span
                    title={evalData.post_check || ""}
                    style={{
                      fontSize: 10,
                      padding: "2px 6px",
                      borderRadius: 3,
                      background: evalData.passed ? "var(--accent-dim)" : "#e8a83820",
                      color: evalData.passed ? "var(--accent)" : "#e8a838",
                      cursor: "help",
                    }}
                  >
                    {evalData.passed ? "✓ 目标达成" : "⚠ 目标未达成"}
                  </span>
                );
              } catch { return null; }
            })()}
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span style={{ fontSize: 10, color: "var(--text-muted)" }}>
              {format(new Date(run.started_at), "MMM d HH:mm:ss")}
              {run.duration_ms && ` · ${formatDuration(run.duration_ms)}`}
            </span>
            <button
              className="btn btn-ghost"
              style={{ padding: "3px 8px" }}
              onClick={handleDownload}
              title="下载日志"
            >
              <Download size={11} />
            </button>
            <button
              className="btn btn-ghost"
              style={{ padding: "3px 8px" }}
              onClick={onClose}
            >
              <X size={12} />
            </button>
          </div>
        </div>

        {/* Tabs */}
        <div
          style={{
            display: "flex",
            borderBottom: "1px solid var(--border)",
            padding: "0 16px",
          }}
        >
          {(["stdout", "stderr"] as const).map((tab) => {
            const stderrLines = rawStderr.split("\n").filter(Boolean);
            const hasNonSystemLines = stderrLines.some((l) => !l.startsWith("[ai-cron]"));
            const isError = hasNonSystemLines && run.status === "failed";
            return (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                style={{
                  padding: "8px 14px",
                  fontSize: 11,
                  background: "transparent",
                  border: "none",
                  borderBottom:
                    activeTab === tab
                      ? "2px solid var(--accent)"
                      : "2px solid transparent",
                  color:
                    activeTab === tab
                      ? "var(--text-primary)"
                      : "var(--text-muted)",
                  cursor: "pointer",
                  fontFamily: "inherit",
                }}
              >
                {tabLabels[tab]}
                {tab === "stderr" && stderrLines.length > 0 && (
                  <span
                    style={{
                      marginLeft: 5,
                      fontSize: 9,
                      color: isError ? "var(--accent-red)" : "var(--text-muted)",
                      background: isError ? "#ff444420" : "var(--border)",
                      padding: "1px 4px",
                      borderRadius: 3,
                    }}
                  >
                    {stderrLines.length}
                  </span>
                )}
              </button>
            );
          })}
        </div>

        {/* Log content */}
        <div
          ref={logRef}
          className="log-viewer"
          style={{
            margin: 16,
            maxHeight: "55vh",
            borderRadius: 4,
          }}
          dangerouslySetInnerHTML={{
            __html: activeContent
              ? renderHtml(activeContent)
              : '<span style="color: #555">暂无输出</span>',
          }}
        />
      </div>
    </div>
  );
}
