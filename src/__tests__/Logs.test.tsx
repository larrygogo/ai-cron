import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { Logs } from "../pages/Logs";
import type { RunWithTaskName } from "../lib/types";

vi.mock("../lib/tauri", () => ({
  getAllRuns: vi.fn(),
  cleanupOldRuns: vi.fn(),
}));

// Mock RunLogModal to avoid complex rendering
vi.mock("../components/runs/RunLogModal", () => ({
  RunLogModal: ({ onClose }: { onClose: () => void }) => (
    <div data-testid="run-log-modal">
      <button onClick={onClose}>Close</button>
    </div>
  ),
}));

import * as api from "../lib/tauri";

const mockRunWithTaskName = (
  overrides: Partial<RunWithTaskName> = {}
): RunWithTaskName => ({
  run: {
    id: "r1",
    task_id: "t1",
    status: "success",
    stdout: "output text",
    stderr: "",
    started_at: new Date().toISOString(),
    triggered_by: "scheduler",
    duration_ms: 5000,
    ...overrides.run,
  },
  task_name: overrides.task_name ?? "Test Task",
});

describe("Logs", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.getAllRuns).mockResolvedValue([]);
  });

  it("shows empty state when no runs exist", async () => {
    render(<Logs />);

    await waitFor(() => {
      expect(screen.getByText("暂无日志")).toBeInTheDocument();
    });
  });

  it("displays run data when runs exist", async () => {
    const runs = [
      mockRunWithTaskName({ task_name: "Daily Report" }),
      mockRunWithTaskName({
        run: {
          id: "r2",
          task_id: "t2",
          status: "failed",
          stdout: "",
          stderr: "err",
          started_at: new Date().toISOString(),
          triggered_by: "manual",
        },
        task_name: "Code Review",
      }),
    ];
    vi.mocked(api.getAllRuns).mockResolvedValue(runs);

    render(<Logs />);

    await waitFor(() => {
      expect(screen.getByText("Daily Report")).toBeInTheDocument();
      expect(screen.getByText("Code Review")).toBeInTheDocument();
    });
  });

  it("debounces search input (300ms)", async () => {
    vi.useFakeTimers();
    vi.mocked(api.getAllRuns).mockResolvedValue([]);

    render(<Logs />);

    // Wait for initial load
    await vi.advanceTimersByTimeAsync(350);

    const callCountAfterInit = vi.mocked(api.getAllRuns).mock.calls.length;

    const searchInput = screen.getByPlaceholderText("搜索日志...");
    fireEvent.change(searchInput, { target: { value: "test" } });

    // Advance past debounce
    await vi.advanceTimersByTimeAsync(350);

    const callsAfterSearch = vi.mocked(api.getAllRuns).mock.calls;
    const lastCall = callsAfterSearch[callsAfterSearch.length - 1];
    expect(lastCall?.[0]).toEqual(
      expect.objectContaining({ searchQuery: "test" })
    );
    expect(callsAfterSearch.length).toBeGreaterThan(callCountAfterInit);

    vi.useRealTimers();
  });

  it("filters by status when tab is clicked", async () => {
    vi.mocked(api.getAllRuns).mockResolvedValue([]);

    render(<Logs />);

    await waitFor(() => {
      expect(api.getAllRuns).toHaveBeenCalled();
    });

    vi.clearAllMocks();
    vi.mocked(api.getAllRuns).mockResolvedValue([]);

    fireEvent.click(screen.getByText("失败"));

    await waitFor(() => {
      expect(api.getAllRuns).toHaveBeenCalledWith(
        expect.objectContaining({ statusFilter: "failed" })
      );
    });
  });
});
