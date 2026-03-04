import { describe, it, expect, vi, beforeEach } from "vitest";
import { useRunStore } from "../../stores/runs";
import type { Run } from "../../lib/types";

vi.mock("../../lib/tauri", () => ({
  getRuns: vi.fn(),
}));

import * as api from "../../lib/tauri";

const mockRun = (overrides: Partial<Run> = {}): Run => ({
  id: "r1",
  task_id: "t1",
  status: "success",
  stdout: "",
  stderr: "",
  started_at: "2025-01-01T09:00:00Z",
  triggered_by: "scheduler",
  ...overrides,
});

describe("useRunStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useRunStore.setState({
      runsByTask: {},
      liveOutput: {},
      loading: {},
    });
  });

  describe("fetchRuns", () => {
    it("should set loading and populate runsByTask on success", async () => {
      const runs = [mockRun(), mockRun({ id: "r2" })];
      vi.mocked(api.getRuns).mockResolvedValue(runs);

      const promise = useRunStore.getState().fetchRuns("t1");
      expect(useRunStore.getState().loading["t1"]).toBe(true);

      await promise;
      expect(useRunStore.getState().loading["t1"]).toBe(false);
      expect(useRunStore.getState().runsByTask["t1"]).toEqual(runs);
    });

    it("should set loading to false on failure", async () => {
      vi.mocked(api.getRuns).mockRejectedValue(new Error("fail"));

      await useRunStore.getState().fetchRuns("t1");
      expect(useRunStore.getState().loading["t1"]).toBe(false);
    });
  });

  describe("updateRunStatus", () => {
    it("should update status, duration_ms, exit_code, and ended_at", () => {
      useRunStore.setState({
        runsByTask: { t1: [mockRun({ id: "r1", status: "running" })] },
      });

      useRunStore
        .getState()
        .updateRunStatus("r1", "t1", "success", 5000, 0);

      const run = useRunStore.getState().runsByTask["t1"][0];
      expect(run.status).toBe("success");
      expect(run.duration_ms).toBe(5000);
      expect(run.exit_code).toBe(0);
      expect(run.ended_at).toBeDefined();
    });
  });

  describe("appendOutput", () => {
    it("should append to stdout", () => {
      useRunStore.getState().appendOutput("r1", "t1", "hello", "stdout");

      const output = useRunStore.getState().liveOutput["r1"];
      expect(output.stdout).toContain("hello");
      expect(output.stderr).toBe("");
    });

    it("should append to stderr", () => {
      useRunStore.getState().appendOutput("r1", "t1", "error msg", "stderr");

      const output = useRunStore.getState().liveOutput["r1"];
      expect(output.stderr).toContain("error msg");
      expect(output.stdout).toBe("");
    });

    it("should accumulate multiple chunks", () => {
      const store = useRunStore.getState();
      store.appendOutput("r1", "t1", "line1", "stdout");
      useRunStore.getState().appendOutput("r1", "t1", "line2", "stdout");

      const output = useRunStore.getState().liveOutput["r1"];
      expect(output.stdout).toContain("line1");
      expect(output.stdout).toContain("line2");
    });
  });

  describe("clearLiveOutput", () => {
    it("should remove liveOutput for the given runId", () => {
      useRunStore.setState({
        liveOutput: {
          r1: { stdout: "data", stderr: "" },
          r2: { stdout: "other", stderr: "" },
        },
      });

      useRunStore.getState().clearLiveOutput("r1");

      const output = useRunStore.getState().liveOutput;
      expect(output["r1"]).toBeUndefined();
      expect(output["r2"]).toBeDefined();
    });
  });
});
