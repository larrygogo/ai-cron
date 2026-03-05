# AI Cron — AI Agent 定时任务调度器

## 简介

AI Cron 是一个桌面应用，用于管理和调度 AI Agent 定时任务。支持 cron 表达式调度、自然语言创建任务、多种 AI 工具（Claude CLI、OpenAI、Ollama）、运行历史查询、内嵌 MCP Server 等。

## 技术栈

- **前端**: React + TypeScript + Vite
- **后端**: Rust + Tauri v2
- **数据库**: SQLite (rusqlite)
- **调度**: tokio-cron-scheduler
- **MCP**: rmcp (Rust MCP SDK) + axum (Streamable HTTP)

## 项目结构

```
src/                        # 前端 React
  components/               # UI 组件
  lib/                      # API 封装 (tauri.ts)、类型 (types.ts)
  pages/                    # Dashboard, Logs, Settings
src-tauri/                  # Rust 后端
  src/
    commands/               # Tauri 命令
      tasks.rs              # 任务 CRUD
      runs.rs               # 运行记录查询
      runner.rs             # AI 工具执行引擎
      scheduler.rs          # 调度预览
      ai_parse.rs           # 自然语言解析
      tools.rs              # 工具检测 + 设置管理
    db/                     # 数据库连接 + 迁移
    models/                 # Task, Run, TriggerSource 等模型
    scheduler/              # 调度引擎 (SchedulerState)
    mcp/                    # MCP Server 模块
      mod.rs                # start_mcp_server()
      server.rs             # AiCronMcp + ServerHandler
      tools.rs              # 15 个 MCP tools
      resources.rs          # 5 种 MCP resources
      prompts.rs            # 4 个 MCP prompts
    lib.rs                  # Tauri 应用入口
```

## 构建与运行

```bash
# 开发模式
npm run tauri dev

# 构建发布
npm run tauri build

# 前端类型检查
npx tsc --noEmit

# Rust 编译检查
cd src-tauri && cargo check

# Rust 测试
cd src-tauri && cargo test
```

## MCP Server

AI Cron 内嵌 MCP Server（Streamable HTTP 传输），应用运行时自动启动。

### 连接配置

在 `~/.claude.json` 的 `mcpServers` 中添加：

```json
"ai-cron": {
  "type": "stdio",
  "command": "node",
  "args": ["<安装路径>/mcp-bridge.mjs"]
}
```

`mcp-bridge.mjs` 是 stdio-to-HTTP 桥接脚本，通过 Node.js 直连 `127.0.0.1:23987`，不受系统代理影响。端口可在 Settings 页面修改（默认 23987），同时需设置 `AI_CRON_MCP_PORT` 环境变量。

### MCP Tools (15 个)

| Tool | 说明 |
|------|------|
| `list_tasks` | 列出所有任务 |
| `get_task` | 获取任务详情 |
| `create_task` | 创建任务并注册调度 |
| `update_task` | 更新任务配置 |
| `delete_task` | 删除任务 |
| `set_task_enabled` | 启用/禁用任务 |
| `trigger_task` | 手动触发执行 |
| `kill_run` | 终止运行中的任务 |
| `list_runs` | 查询运行历史 |
| `get_run` | 获取运行详情（含输出） |
| `cleanup_runs` | 清理旧记录 |
| `preview_schedule` | 预览 cron 下次运行时间 |
| `parse_natural_language` | 自然语言 → 任务配置 |
| `get_settings` | 获取应用设置 |
| `update_settings` | 更新应用设置 |

### MCP Resources (5 种)

| URI | 说明 |
|-----|------|
| `aicron://tasks` | 所有任务列表 |
| `aicron://tasks/{task_id}` | 单个任务详情 |
| `aicron://tasks/{task_id}/runs` | 任务最近 20 次运行 |
| `aicron://runs/{run_id}` | 运行详情（含完整日志） |
| `aicron://settings` | 当前应用设置 |

### MCP Prompts (4 个)

| Prompt | 参数 | 功能 |
|--------|------|------|
| `create_task_guide` | `description` | 引导创建任务 |
| `diagnose_run` | `run_id` | 诊断失败运行 |
| `task_status_report` | 无 | 全局状态报告 |
| `optimize_schedule` | `task_id` | 调度优化建议 |

## 代码规范

- Rust 风格遵循 `cargo clippy` 建议
- UI 文案使用中文
- Git commit message 使用中文
- Cron 表达式使用标准 5 字段格式 (minute hour dom month dow)
