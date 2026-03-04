export type AiTool = "claude" | "opencode" | "codex" | "custom";

export type RunStatus = "queued" | "running" | "success" | "failed" | "killed";

export type TriggerSource = "scheduler" | "manual";

export interface WebhookConfig {
  url: string;
  platform: "feishu" | "generic";
  on_start: boolean;
  on_success: boolean;
  on_failure: boolean;
  on_killed: boolean;
}

export interface Task {
  id: string;
  name: string;
  cron_expression: string;
  cron_human: string;
  ai_tool: AiTool;
  custom_command?: string;
  prompt: string;
  working_directory: string;
  enabled: boolean;
  inject_context: boolean;
  restrict_network: boolean;
  restrict_filesystem: boolean;
  env_vars: Record<string, string>;
  webhook_config?: WebhookConfig;
  created_at: string;
  updated_at: string;
  last_run_at?: string;
  last_run_status?: RunStatus;
}

export interface CreateTaskRequest {
  name: string;
  cron_expression: string;
  cron_human?: string;
  ai_tool?: AiTool;
  custom_command?: string;
  prompt: string;
  working_directory: string;
  enabled?: boolean;
  inject_context?: boolean;
  restrict_network?: boolean;
  restrict_filesystem?: boolean;
  env_vars?: Record<string, string>;
  webhook_config?: WebhookConfig;
}

export interface UpdateTaskRequest {
  name?: string;
  cron_expression?: string;
  cron_human?: string;
  ai_tool?: AiTool;
  custom_command?: string;
  prompt?: string;
  working_directory?: string;
  enabled?: boolean;
  inject_context?: boolean;
  restrict_network?: boolean;
  restrict_filesystem?: boolean;
  env_vars?: Record<string, string>;
  webhook_config?: WebhookConfig | null;
}

export interface Run {
  id: string;
  task_id: string;
  status: RunStatus;
  exit_code?: number;
  stdout: string;
  stderr: string;
  started_at: string;
  ended_at?: string;
  duration_ms?: number;
  triggered_by: TriggerSource;
}

export interface RunWithTaskName {
  run: Run;
  task_name: string;
}

export interface ToolInfo {
  name: string;
  label: string;
  available: boolean;
  version?: string;
  path?: string;
  install_url: string;
  install_cmd?: string;
}

export interface AppSettings {
  nl_provider: "claude" | "openai" | "ollama";
  nl_api_key: string;
  nl_base_url: string;
  nl_model: string;
  log_retention_days: number;
  log_retention_per_task: number;
  notify_on_success: boolean;
  notify_on_failure: boolean;
}

export interface TaskDraft {
  name: string;
  cron_expression: string;
  cron_human: string;
  prompt: string;
  ai_tool: AiTool;
  suggested_directory: string;
}
