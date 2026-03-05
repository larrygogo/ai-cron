import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { RunLogModal } from "../components/runs/RunLogModal";
import type { Run } from "../lib/types";

// Mock the run store
vi.mock("../stores/runs", () => ({
  useRunStore: () => ({
    liveOutput: {},
  }),
}));

const mockRun = (overrides: Partial<Run> = {}): Run => ({
  id: "run-1",
  task_id: "t1",
  status: "success",
  exit_code: 0,
  stdout: "Hello world output",
  stderr: "",
  started_at: "2025-01-01T09:00:00Z",
  ended_at: "2025-01-01T09:01:00Z",
  duration_ms: 60000,
  triggered_by: "scheduler",
  ...overrides,
});

describe("RunLogModal", () => {
  const onClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders run log with stdout content", () => {
    const run = mockRun({ stdout: "Test output content" });
    render(<RunLogModal run={run} onClose={onClose} />);

    expect(screen.getByText("运行日志")).toBeInTheDocument();
    expect(screen.getByText(/run-1/)).toBeInTheDocument();
    expect(screen.getByText(/Test output content/)).toBeInTheDocument();
  });

  it("displays 2 tabs with Chinese labels", () => {
    const run = mockRun();
    render(<RunLogModal run={run} onClose={onClose} />);

    expect(screen.getByText("结果")).toBeInTheDocument();
    expect(screen.getByText("过程")).toBeInTheDocument();
  });

  it("shows complete stderr in process tab including [ai-cron] lines and other output", () => {
    const run = mockRun({
      stderr: "[ai-cron] ▶ 开始执行任务: Test\n[ai-cron] ✗ 进程异常退出\nerror detail",
    });
    const { container } = render(<RunLogModal run={run} onClose={onClose} />);

    // Click process tab — should show complete stderr
    fireEvent.click(screen.getByText("过程"));
    const logViewer = container.querySelector(".log-viewer");
    expect(logViewer!.textContent).toContain("[ai-cron]");
    expect(logViewer!.textContent).toContain("error detail");
  });

  it("badge counts all stderr lines", () => {
    const run = mockRun({
      stderr: "[ai-cron] ▶ 开始执行\n[ai-cron] ✓ 完成\nreal error 1\nreal error 2",
    });
    render(<RunLogModal run={run} onClose={onClose} />);

    // Badge should show 4 (all non-empty lines)
    const badge = screen.getByText("4");
    expect(badge).toBeInTheDocument();
  });

  it("badge uses red color only when failed and has non-system stderr lines", () => {
    const run = mockRun({
      status: "failed",
      stderr: "[ai-cron] ▶ 开始执行\nerror detail",
    });
    const { container } = render(<RunLogModal run={run} onClose={onClose} />);

    const badge = screen.getByText("2");
    expect(badge).toBeInTheDocument();
    // Red badge for failed runs with non-[ai-cron] lines
    expect(badge.style.color).toContain("var(--accent-red)");
  });

  it("badge uses neutral color for success runs with stderr", () => {
    const run = mockRun({
      status: "success",
      stderr: "[ai-cron] ▶ 开始执行\nsome cli progress info",
    });
    render(<RunLogModal run={run} onClose={onClose} />);

    const badge = screen.getByText("2");
    expect(badge.style.color).toContain("var(--text-muted)");
  });

  it("styles [ai-cron] lines with italic styling in rendered HTML", () => {
    const run = mockRun({
      stdout: "[ai-cron] ▶ 开始执行任务: Test Task\nActual output here",
    });
    const { container } = render(<RunLogModal run={run} onClose={onClose} />);

    // The log viewer should contain the styled [ai-cron] line
    const logViewer = container.querySelector(".log-viewer");
    expect(logViewer).not.toBeNull();
    const html = logViewer!.innerHTML;
    // [ai-cron] lines should be wrapped with italic styling
    expect(html).toContain("font-style: italic");
    expect(html).toContain("[ai-cron]");
  });

  it("renders normal output without italic styling", () => {
    const run = mockRun({
      stdout: "Normal output without special prefix",
    });
    const { container } = render(<RunLogModal run={run} onClose={onClose} />);

    const logViewer = container.querySelector(".log-viewer");
    expect(logViewer).not.toBeNull();
    const html = logViewer!.innerHTML;
    expect(html).not.toContain("font-style: italic");
    expect(html).toContain("Normal output");
  });

  it("shows status badge for failed runs", () => {
    const run = mockRun({ status: "failed", exit_code: 1 });
    render(<RunLogModal run={run} onClose={onClose} />);

    expect(screen.getByText("failed")).toBeInTheDocument();
  });

  it("shows live indicator for running status", () => {
    const run = mockRun({ status: "running", ended_at: undefined, duration_ms: undefined });
    render(<RunLogModal run={run} onClose={onClose} />);

    expect(screen.getByText("● 实时")).toBeInTheDocument();
  });
});
