import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { TaskFormModal } from "../components/tasks/TaskFormModal";
import type { Task } from "../lib/types";

// Mock the tauri API module
vi.mock("../lib/tauri", () => ({
  createTask: vi.fn(),
  updateTask: vi.fn(),
  previewNextRuns: vi.fn().mockResolvedValue([]),
}));

// Mock NextRunsPreview to avoid side effects
vi.mock("../components/scheduler/NextRunsPreview", () => ({
  NextRunsPreview: () => <div data-testid="next-runs-preview">Preview</div>,
}));

import * as api from "../lib/tauri";

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
  created_at: "2025-01-01T00:00:00Z",
  updated_at: "2025-01-01T00:00:00Z",
  ...overrides,
});

describe("TaskFormModal", () => {
  const onClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders in create mode with correct title", () => {
    render(<TaskFormModal onClose={onClose} />);
    // Title is in a span, button also says "Create Task" — use getAllByText
    const elements = screen.getAllByText("Create Task");
    expect(elements.length).toBeGreaterThanOrEqual(2); // header + button
    expect(elements[0]).toBeInTheDocument();
  });

  it("renders in edit mode with task data filled in", () => {
    const task = mockTask({ name: "My Task", prompt: "Test prompt" });
    render(<TaskFormModal task={task} onClose={onClose} />);

    expect(screen.getByText("Edit Task")).toBeInTheDocument();
    expect(screen.getByDisplayValue("My Task")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Test prompt")).toBeInTheDocument();
    expect(screen.getByText("Save Changes")).toBeInTheDocument();
  });

  it("shows validation error when required fields are empty", async () => {
    render(<TaskFormModal onClose={onClose} />);

    // Clear default cron and leave name/prompt empty
    const nameInput = screen.getByPlaceholderText("My daily task");
    fireEvent.change(nameInput, { target: { value: "" } });

    // Find the create/submit button
    const buttons = screen.getAllByRole("button");
    const submitBtn = buttons.find((b) => b.textContent === "Create Task");
    fireEvent.click(submitBtn!);

    await waitFor(() => {
      expect(
        screen.getByText("Name, Cron expression, and Prompt are required.")
      ).toBeInTheDocument();
    });
    expect(api.createTask).not.toHaveBeenCalled();
  });

  it("calls createTask and onClose on successful create", async () => {
    const created = mockTask({ id: "new-1" });
    vi.mocked(api.createTask).mockResolvedValue(created);

    render(<TaskFormModal onClose={onClose} />);

    fireEvent.change(screen.getByPlaceholderText("My daily task"), {
      target: { value: "New Task" },
    });
    fireEvent.change(
      screen.getByPlaceholderText("Describe what the AI agent should do..."),
      { target: { value: "Run tests" } }
    );
    fireEvent.change(screen.getByPlaceholderText("/path/to/project"), {
      target: { value: "/home/user/project" },
    });

    const buttons = screen.getAllByRole("button");
    const submitBtn = buttons.find((b) => b.textContent === "Create Task");
    fireEvent.click(submitBtn!);

    await waitFor(() => {
      expect(api.createTask).toHaveBeenCalledTimes(1);
      expect(onClose).toHaveBeenCalled();
    });
  });

  it("calls updateTask on successful edit", async () => {
    const task = mockTask();
    const updated = mockTask({ name: "Updated" });
    vi.mocked(api.updateTask).mockResolvedValue(updated);

    render(<TaskFormModal task={task} onClose={onClose} />);

    fireEvent.change(screen.getByDisplayValue("Test Task"), {
      target: { value: "Updated" },
    });
    fireEvent.click(screen.getByText("Save Changes"));

    await waitFor(() => {
      expect(api.updateTask).toHaveBeenCalledWith("t1", expect.any(Object));
      expect(onClose).toHaveBeenCalled();
    });
  });

  it("calls onClose when Cancel is clicked", () => {
    render(<TaskFormModal onClose={onClose} />);
    fireEvent.click(screen.getByText("Cancel"));
    expect(onClose).toHaveBeenCalled();
  });
});
