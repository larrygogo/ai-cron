import { useEffect, useRef, useState } from "react";
import { X, Download } from "lucide-react";
import AnsiToHtml from "ansi-to-html";
import { format } from "date-fns";
import type { Run } from "../../lib/types";
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

function formatDuration(ms?: number): string {
  if (!ms) return "-";
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  return `${Math.floor(s / 60)}m${s % 60}s`;
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
  }, [liveOutput]);

  const stdout = isLive ? (liveOutput?.stdout ?? "") : run.stdout;
  const stderr = isLive ? (liveOutput?.stderr ?? "") : run.stderr;
  const activeContent = activeTab === "stdout" ? stdout : stderr;

  const renderHtml = (text: string) => {
    try {
      return ansiConverter.toHtml(text);
    } catch {
      return text.replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }
  };

  const handleDownload = () => {
    const content = `=== STDOUT ===\n${stdout}\n\n=== STDERR ===\n${stderr}`;
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
              Run log
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
              {isLive ? "● live" : run.status}
            </span>
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
              title="Download log"
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
          {(["stdout", "stderr"] as const).map((tab) => (
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
              {tab}
              {tab === "stderr" && stderr.length > 0 && (
                <span
                  style={{
                    marginLeft: 5,
                    fontSize: 9,
                    color: "var(--accent-red)",
                    background: "#ff444420",
                    padding: "1px 4px",
                    borderRadius: 3,
                  }}
                >
                  {stderr.split("\n").filter(Boolean).length}
                </span>
              )}
            </button>
          ))}
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
              : '<span style="color: #555">No output</span>',
          }}
        />
      </div>
    </div>
  );
}
