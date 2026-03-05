import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { TaskDetail } from "../components/tasks/TaskDetail";
import type { Task } from "../lib/types";

// Mock tauri API
vi.mock("../lib/tauri", () => ({
  setTaskEnabled: vi.fn(),
  triggerTaskNow: vi.fn(),
  deleteTask: vi.fn(),
  generatePlan: vi.fn(),
  updatePlan: vi.fn(),
  getTask: vi.fn(),
  getRuns: vi.fn().mockResolvedValue([]),
  previewNextRuns: vi.fn().mockResolvedValue([]),
  onPlanGenerated: vi.fn().mockResolvedValue(() => {}),
  onRunStarted: vi.fn().mockResolvedValue(() => {}),
  onRunOutput: vi.fn().mockResolvedValue(() => {}),
  onRunCompleted: vi.fn().mockResolvedValue(() => {}),
}));

// Mock child components to isolate TaskDetail
vi.mock("../components/runs/RunHistory", () => ({
  RunHistory: () => <div data-testid="run-history">RunHistory</div>,
}));
vi.mock("../components/scheduler/NextRunsPreview", () => ({
  NextRunsPreview: () => <div data-testid="next-runs-preview">Preview</div>,
}));
vi.mock("../components/ui/ConfirmDialog", () => ({
  ConfirmDialog: ({ onConfirm, onCancel }: { onConfirm: () => void; onCancel: () => void }) => (
    <div data-testid="confirm-dialog">
      <button onClick={onConfirm}>确认</button>
      <button onClick={onCancel}>取消</button>
    </div>
  ),
}));

// Mock stores
vi.mock("../stores/tasks", () => ({
  useTaskStore: () => ({
    updateTaskInStore: vi.fn(),
    removeTaskFromStore: vi.fn(),
  }),
}));

import * as api from "../lib/tauri";

const mockTask = (overrides: Partial<Task> = {}): Task => ({
  id: "t1",
  name: "Test Task",
  cron_expression: "0 9 * * *",
  cron_human: "每天 09:00",
  ai_tool: "claude",
  prompt: "Run tests",
  working_directory: "/home/user/project",
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

describe("TaskDetail - Execution Plan", () => {
  const onEdit = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows empty plan message when no plan exists", () => {
    const task = mockTask({ execution_plan: "" });
    render(<TaskDetail task={task} onEdit={onEdit} />);

    expect(screen.getByText("执行计划")).toBeInTheDocument();
    expect(screen.getByText(/暂无执行计划/)).toBeInTheDocument();
    expect(screen.getByText("生成计划")).toBeInTheDocument();
  });

  it("displays existing execution plan content", () => {
    const task = mockTask({ execution_plan: "## Steps\n1. Run tests\n2. Check results" });
    render(<TaskDetail task={task} onEdit={onEdit} />);

    expect(screen.getByText("执行计划")).toBeInTheDocument();
    expect(screen.getByText(/## Steps/)).toBeInTheDocument();
    expect(screen.getByText("重新生成")).toBeInTheDocument();
    // There are two "编辑" buttons: header edit + plan edit
    const editButtons = screen.getAllByText("编辑");
    expect(editButtons.length).toBe(2);
  });

  it("calls generatePlan when generate button is clicked", async () => {
    const task = mockTask({ execution_plan: "" });
    vi.mocked(api.generatePlan).mockResolvedValue("Generated plan content");

    render(<TaskDetail task={task} onEdit={onEdit} />);

    fireEvent.click(screen.getByText("生成计划"));

    await waitFor(() => {
      expect(api.generatePlan).toHaveBeenCalledWith("t1");
    });
  });

  it("calls generatePlan when regenerate button is clicked", async () => {
    const task = mockTask({ execution_plan: "Old plan" });
    vi.mocked(api.generatePlan).mockResolvedValue("New plan");

    render(<TaskDetail task={task} onEdit={onEdit} />);

    fireEvent.click(screen.getByText("重新生成"));

    await waitFor(() => {
      expect(api.generatePlan).toHaveBeenCalledWith("t1");
    });
  });

  it("enters edit mode and saves plan", async () => {
    const task = mockTask({ execution_plan: "Original plan" });
    vi.mocked(api.updatePlan).mockResolvedValue(undefined);

    render(<TaskDetail task={task} onEdit={onEdit} />);

    // Click plan edit button (the smaller one with font-size 10)
    const editButtons = screen.getAllByText("编辑");
    // The plan edit button is the second one (plan section)
    fireEvent.click(editButtons[1]);

    // Should show textarea with plan content
    const textarea = screen.getByDisplayValue("Original plan");
    expect(textarea).toBeInTheDocument();

    // Modify the plan
    fireEvent.change(textarea, { target: { value: "Modified plan" } });

    // Click save
    fireEvent.click(screen.getByText("保存"));

    await waitFor(() => {
      expect(api.updatePlan).toHaveBeenCalledWith("t1", "Modified plan");
    });
  });

  it("cancels edit mode without saving", () => {
    const task = mockTask({ execution_plan: "Original plan" });
    render(<TaskDetail task={task} onEdit={onEdit} />);

    // Enter edit mode (plan edit button is the second "编辑")
    const editButtons = screen.getAllByText("编辑");
    fireEvent.click(editButtons[1]);
    expect(screen.getByDisplayValue("Original plan")).toBeInTheDocument();

    // Cancel (the "取消" in the plan section, not the confirm dialog)
    fireEvent.click(screen.getByText("取消"));

    // Should be back to display mode — no textarea visible
    const textareas = screen.queryAllByRole("textbox");
    expect(textareas).toHaveLength(0);
    expect(screen.getByText(/Original plan/)).toBeInTheDocument();
  });

  it("disables plan edit button when plan is empty", () => {
    const task = mockTask({ execution_plan: "" });
    render(<TaskDetail task={task} onEdit={onEdit} />);

    // Plan edit button is the second "编辑"
    const editButtons = screen.getAllByText("编辑");
    const planEditBtn = editButtons[1].closest("button");
    expect(planEditBtn).toBeDisabled();
  });

  it("defaults to detail tab with prompt and plan visible", () => {
    const task = mockTask({ execution_plan: "My plan", prompt: "Run tests" });
    render(<TaskDetail task={task} onEdit={onEdit} />);

    expect(screen.getByText("提示词")).toBeInTheDocument();
    expect(screen.getByText("执行计划")).toBeInTheDocument();
    expect(screen.queryByTestId("run-history")).not.toBeInTheDocument();
  });

  it("switches to history tab and back", () => {
    const task = mockTask({ execution_plan: "My plan", prompt: "Run tests" });
    render(<TaskDetail task={task} onEdit={onEdit} />);

    // Switch to history tab
    fireEvent.click(screen.getByText("运行历史"));
    expect(screen.getByTestId("run-history")).toBeInTheDocument();
    expect(screen.queryByText("提示词")).not.toBeInTheDocument();

    // Switch back to detail tab
    fireEvent.click(screen.getByText("详情"));
    expect(screen.getByText("提示词")).toBeInTheDocument();
    expect(screen.queryByTestId("run-history")).not.toBeInTheDocument();
  });

  it("shows task info fields correctly", () => {
    const task = mockTask({
      name: "My Test Task",
      cron_human: "每天 09:00",
      cron_expression: "0 9 * * *",
      working_directory: "/home/user",
    });
    render(<TaskDetail task={task} onEdit={onEdit} />);

    expect(screen.getByText("My Test Task")).toBeInTheDocument();
    expect(screen.getByText("每天 09:00")).toBeInTheDocument();
    expect(screen.getByText(/\/home\/user/)).toBeInTheDocument();
  });
});
