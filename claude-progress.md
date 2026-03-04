# AI Cron 进度记录

## 当前状态

- **总功能数**：12
- **已完成**：12 (100%)
- **进行中**：0
- **待开始**：0
- **最后更新**：2026-03-05

---

## 最近完成（最新在前）

- [2026-03-05] test: 三层验证方案 Step 1-3 — 前端 25 tests (Vitest + React Testing Library) + 后端 26 tests (cargo test + 内存 SQLite)，共 51 个测试全部通过
- [2026-03-05] feature #2-#12: Phase 2-4 全功能实现 — TaskFormModal 编辑弹窗、调度引擎 hot-reload、进程 kill、全局 Logs 页、ToolStatusBar、系统托盘、桌面通知、键盘快捷键、Toast
- [2026-03-05] feature #1: 项目初始化与基础架构搭建 — Tauri 2.0 + React + SQLite + 全前后端骨架

---

## 下一步任务

所有计划功能已完成，前端+后端单元测试已添加。后续可考虑：
- [x] 添加单元测试（51 tests 已完成）
- [ ] E2E 测试（WebdriverIO + tauri-driver，需 build 后运行）
- [ ] 端到端功能测试（cargo tauri dev 验证所有流程）
- [ ] 发布 v0.1.0 版本

---

## 已知问题 / 阻塞点

- Windows 沙箱模式（restrict_network/restrict_filesystem）标记为实验性，Job Object 网络限制能力有限
- cron preview 用简单步进算法，精度够用但不支持所有 cron 扩展语法

---

## 技术决策记录

### 2026-03-05：项目技术栈
**背景**：需要跨平台桌面应用承载 AI 定时任务管理
**决策**：Tauri 2.0 (Rust) + React + TypeScript + SQLite + tokio-cron-scheduler
**原因**：Tauri 包体小、性能好；SQLite 零依赖本地存储；tokio-cron-scheduler 与 Tauri async runtime 天然兼容

### 2026-03-05：多 provider NL 解析
**背景**：用户需要通过自然语言添加定时任务
**决策**：支持 Claude / OpenAI / Ollama 三种 provider，统一 system prompt，返回结构化 JSON
**原因**：覆盖有 API key 和本地模型的用户

### 2026-03-05：调度引擎 hot-reload 方案
**背景**：Phase 1 中调度器不支持运行时增删任务
**决策**：使用 job_map（task_id → job_uuid）追踪 job，通过 try_state 延迟获取 SchedulerState
**原因**：app.manage() 在 async setup 中注册，需要 try_state 而非 state

### 2026-03-05：进程 kill 方案
**背景**：需要终止运行中的任务进程
**决策**：全局 LazyLock<Mutex<HashMap<run_id, PID>>>，Windows 用 taskkill，Unix 用 kill 命令
**原因**：跨平台兼容，不引入额外 crate

---

## 会话历史摘要

- [2026-03-05] Phase 1 完成：项目初始化、SQLite 建表（含 FTS5）、Task/Run CRUD、Webhook 模型、Runner 框架、调度引擎框架、前端 React 骨架、深色终端 UI、全栈编译通过
- [2026-03-05] Phase 2-4 完成：TaskFormModal、调度引擎 hot-reload、进程注册表+kill、全局 Logs 页、ToolStatusBar、系统托盘+通知、键盘快捷键、Toast，编译验证通过，推送到 GitHub
