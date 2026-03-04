# AI Cron 进度记录

## 当前状态

- **总功能数**：12
- **已完成**：1 (8%)
- **进行中**：0
- **待开始**：11
- **最后更新**：2026-03-05

---

## 最近完成（最新在前）

- [2026-03-05] feature #1: 项目初始化与基础架构搭建 — Tauri 2.0 + React + SQLite + 全前后端骨架

---

## 下一步任务

- [ ] feature #2: 任务 CRUD 完整 UI（优先级：high）
- [ ] feature #3: 调度引擎 hot-reload + Runner 进程管理（优先级：high）
- [ ] feature #4: 工具检测 UI + 安装按钮（优先级：high）
- [ ] feature #5: Run History + Log Viewer 完整联调（优先级：high）
- [ ] feature #6: Webhook 通知（飞书 + Generic）（优先级：medium）
- [ ] feature #7: 全局 Logs 页（搜索/过滤/导出/保留策略）（优先级：medium）
- [ ] feature #8: 自然语言解析 UI 完整联调（优先级：medium）
- [ ] feature #9: 上下文注入开关（优先级：low）
- [ ] feature #10: 沙箱模式（网络/文件系统开关）（优先级：low）
- [ ] feature #11: 系统托盘 + 桌面通知（优先级：low）
- [ ] feature #12: 无边框标题栏 + 键盘快捷键完整（优先级：low）

---

## 已知问题 / 阻塞点

- `set_task_enabled_and_reschedule` 未与 scheduler 联动（Phase 2 实现）
- trigger_task_now 使用第二个 DB 连接（Phase 2 改为 Arc<Mutex<Connection>> 共享）
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

---

## 会话历史摘要

- [2026-03-05] Phase 1 完成：项目初始化、SQLite 建表（含 FTS5）、Task/Run CRUD、Webhook 模型、Runner 框架、调度引擎框架、前端 React 骨架、深色终端 UI、全栈编译通过
