import { describe, it, expect, vi, beforeEach } from "vitest";
import { useTaskStore } from "../../stores/tasks";
import type { Task } from "../../lib/types";

vi.mock("../../lib/tauri", () => ({
  getTasks: vi.fn(),
}));

import * as api from "../../lib/tauri";

const mockTask = (overrides: Partial<Task> = {}): Task => ({
  id: "t1",
  name: "Test Task",
  cron_expression: "0 9 * * *",
  cron_human: "Every day at 9am",
  ai_tool: "claude",
  prompt: "Do something",
  working_directory: "/tmp",
  enabled: true,
  inject_context: false,
  restrict_network: false,
  restrict_filesystem: false,
  env_vars: {},
  allowed_tools: [],
  skip_permissions: false,
  execution_plan: "",
  consecutive_failures: 0,
  created_at: "2025-01-01T00:00:00Z",
  updated_at: "2025-01-01T00:00:00Z",
  ...overrides,
});

describe("useTaskStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store state
    useTaskStore.setState({
      tasks: [],
      selectedId: null,
      loading: false,
      error: null,
    });
  });

  describe("fetchTasks", () => {
    it("should set loading then populate tasks on success", async () => {
      const tasks = [mockTask(), mockTask({ id: "t2", name: "Task 2" })];
      vi.mocked(api.getTasks).mockResolvedValue(tasks);

      const promise = useTaskStore.getState().fetchTasks();
      expect(useTaskStore.getState().loading).toBe(true);

      await promise;
      expect(useTaskStore.getState().loading).toBe(false);
      expect(useTaskStore.getState().tasks).toEqual(tasks);
      expect(useTaskStore.getState().error).toBeNull();
    });

    it("should set error on failure", async () => {
      vi.mocked(api.getTasks).mockRejectedValue(new Error("Network error"));

      await useTaskStore.getState().fetchTasks();
      expect(useTaskStore.getState().loading).toBe(false);
      expect(useTaskStore.getState().error).toBe("Error: Network error");
    });
  });

  describe("addTaskToStore", () => {
    it("should prepend task and set selectedId", () => {
      const existing = mockTask({ id: "t1" });
      useTaskStore.setState({ tasks: [existing] });

      const newTask = mockTask({ id: "t2", name: "New Task" });
      useTaskStore.getState().addTaskToStore(newTask);

      const state = useTaskStore.getState();
      expect(state.tasks[0]).toEqual(newTask);
      expect(state.tasks).toHaveLength(2);
      expect(state.selectedId).toBe("t2");
    });
  });

  describe("removeTaskFromStore", () => {
    it("should remove task by id", () => {
      useTaskStore.setState({
        tasks: [mockTask({ id: "t1" }), mockTask({ id: "t2" })],
        selectedId: "t1",
      });

      useTaskStore.getState().removeTaskFromStore("t2");
      expect(useTaskStore.getState().tasks).toHaveLength(1);
      expect(useTaskStore.getState().tasks[0].id).toBe("t1");
      expect(useTaskStore.getState().selectedId).toBe("t1");
    });

    it("should clear selectedId when removing selected task", () => {
      useTaskStore.setState({
        tasks: [mockTask({ id: "t1" }), mockTask({ id: "t2" })],
        selectedId: "t1",
      });

      useTaskStore.getState().removeTaskFromStore("t1");
      expect(useTaskStore.getState().selectedId).toBeNull();
    });
  });

  describe("updateTaskInStore", () => {
    it("should replace task with matching id", () => {
      useTaskStore.setState({
        tasks: [mockTask({ id: "t1", name: "Old" })],
      });

      const updated = mockTask({ id: "t1", name: "Updated" });
      useTaskStore.getState().updateTaskInStore(updated);

      expect(useTaskStore.getState().tasks[0].name).toBe("Updated");
    });
  });

  describe("setSelected", () => {
    it("should update selectedId", () => {
      useTaskStore.getState().setSelected("t1");
      expect(useTaskStore.getState().selectedId).toBe("t1");
    });

    it("should allow setting to null", () => {
      useTaskStore.setState({ selectedId: "t1" });
      useTaskStore.getState().setSelected(null);
      expect(useTaskStore.getState().selectedId).toBeNull();
    });
  });
});
