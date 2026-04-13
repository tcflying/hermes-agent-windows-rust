# Hermes-Agent-Windows-Rust

**面向 Windows 的高性能 AI Agent 桌面应用，基于 Rust + React 构建。流式工具调用、多提供商 LLM 支持、19 个内置工具、9 页 Web UI。**

> **基于** [NousResearch/hermes-agent](https://github.com/NousResearch/hermes-agent)（MIT License）。从零重写，目标 Windows 桌面，带原生 WebView2 GUI。

---

## 功能一览

### 核心 Agent
- **流式工具调用** — 实时流式响应，工具边生成边执行。修复了流式 tool_call 解析器的关键 bug（commit `d92ca75`）。
- **可中断** — 一键停止任意进行中的流式请求。
- **多轮会话** — 持久化对话历史，SQLite + FTS5 全文搜索。
- **工具自动生成 Skill** — 复杂任务完成后，Agent 自动创建可复用的 Skill。

### 多提供商 LLM 支持
- MiniMax（默认）
- OpenAI（GPT-4 系列）
- Anthropic（Claude 系列）
- models.dev 注册表，支持上下文长度感知
- 会话级模型切换（`/model` 命令）

### 内置工具（19 个）
| 工具 | 说明 |
|------|------|
| `terminal` | 执行 PowerShell/cmd 命令 |
| `file_read` / `file_write` / `file_edit` | 文件读写编辑 |
| `list_dir` | 目录树形列表 |
| `skill_create` / `skill_list` / `skill_view` / `skill_delete` | Skill 生命周期管理 |
| `memory_save` / `memory_search` | 持久化记忆存储 |
| `browser_navigate` / `web_search` | 浏览器自动化 |
| `execute_code` | 沙箱代码执行 |
| `interrupt` | 流式中断 |
| `delegate` | 子 Agent 委托 |

### 前端 UI（9 个页面）
- **Chat** — 主对话界面，工具调用卡片、流式 Markdown、会话切换
- **Files** — Monaco 编辑器 + 文件树，保存/加载工作流
- **Terminal** — 嵌入浏览器的 xterm.js 终端
- **Memory** — 持久化记忆编辑器（基于 section 存储）
- **Skills** — 浏览、创建、管理自动生成的 Skills
- **Dashboard** — 实时 HUD，统计面板（会话数、Skills 数、消息数）
- **Inspector** — 分会话消息历史、Token 用量
- **Settings** — 主题切换（default/ares/slate/mono）、Provider API Key 配置
- **HUD** — Agent 遥测覆盖层

### 桌面打包
- **Tauri 2.x** — 原生 Windows WebView2 打包
- **React + Vite** — 快速热更新开发，全程 TypeScript
- **Playwright** — E2E 测试套件（已加入 devDependencies）

---

## 快速开始

### 环境要求
- Rust 1.93+
- Node.js 18+
- Windows 10/11

### 构建

```powershell
# 克隆你的 fork
git clone https://github.com/tcflying/hermes-agent.git
cd hermes-agent

# 构建 Rust 后端
cargo build --release

# 安装前端依赖
cd crates/ui && npm install && cd ../..
```

### 运行

**方式一 — 脚本（最简单）：**
```powershell
# 双击或在终端运行：
start.bat    # 启动后端（3848 端口）+ 前端（1420 端口）
stop.bat     # 停止所有服务
```

**方式二 — 手动：**
```powershell
# 后端
./target/release/hermes.exe gateway start

# 前端（另一个终端）
cd crates/ui && npm run dev
```

- 前端：http://localhost:1420
- 后端 API：http://localhost:3848

---

## 项目架构

```
hermes-rs/
├── crates/
│   ├── agent/          # 核心 Agent 循环、工具执行、记忆、Prompt 构建
│   │   └── src/
│   │       ├── chat.rs          # 主流式聊天处理器（已修复 tool_call 解析器）
│   │       ├── tools.json        # 19 个工具定义
│   │       ├── memory.rs         # MemoryStore, MemoryManager, MemorySnapshot
│   │       ├── memory_nudge.rs   # 定期 nudge 注入器
│   │       ├── skill_commands.rs # /skill 斜杠命令处理器
│   │       ├── iteration.rs      # IterationBudget（最多 30 次工具调用/轮）
│   │       ├── interrupt.rs      # InterruptFlag 流式中断
│   │       └── auxiliary_client.rs # 视觉 + 摘要辅助客户端
│   ├── cli/            # HermesCLI — 斜杠命令、皮肤引擎、安装向导
│   ├── config/         # YAML 配置加载器，多提供商 schema
│   ├── gateway/        # Axum HTTP 服务器、SSE 流式、Session 路由
│   │   └── platforms/ # 存根适配器：Telegram, Discord, Slack, WhatsApp 等
│   ├── session/        # SQLite + FTS5 Session 数据库，轨迹保存
│   ├── tool-registry/  # 中心工具注册表（存根，待集成）
│   ├── ui/             # React + TypeScript 前端
│   │   └── src/
│   │       ├── pages/  # 9 个页面组件（Chat、Files、Terminal、Memory...）
│   │       ├── components/ # WorkspaceLayout、FileTree、ToolCallCard...
│   │       └── App.tsx # React Router v7 路由
│   └── utils/          # 共享工具函数
├── start.bat           # 一键启动开发环境
├── stop.bat            # 停止所有服务
└── Cargo.toml          # Workspace 根配置
```

### 技术栈

| 层次 | 技术 |
|------|------|
| GUI Shell | Tauri 2.x (WebView2) |
| 前端 | React 18, TypeScript, Vite 6 |
| 后端 | Rust, Tokio, Axum 0.8 |
| 数据库 | SQLite + FTS5 (rusqlite, bundled) |
| 终端 | xterm.js |
| 代码编辑器 | Monaco Editor |
| Markdown | react-markdown + shiki |
| HTTP | reqwest (流式), tower-http |
| CLI | clap (derive) |
| 日志 | tracing + tracing-subscriber |

---

## 关键修复：流式 tool_calls 解析器

流式 tool_call 累积逻辑中存在一个关键 bug，导致每次工具调用都报错：
`{"error":"invalid function arguments json string"}`

**根本原因：** 原有状态机依赖 `index` 字段判断工具调用边界——但 `index` 在不同 Provider 间不可靠，多个并发工具调用时 id flush 时机错误。

**修复（commit `d92ca75`）：** 用 `id` 变化检测替代 index 判断：
- 每次收到新的 tool_call chunk，检查 `id` 是否变化
- `id` 变化时，将前一个工具调用 flush 到累积列表
- 累积状态无需 Mutex，用简单的拥有型变量即可

**文件：** `crates/agent/src/chat.rs` 第 183–283 行

---

## 配置

配置文件：`~/.hermes/config.yaml`（**不会上传到 Git**）

```yaml
llm:
  provider: minimax
  model: MiniMax-M2.7-highspeed
  api_key: your-api-key

display:
  skin: default        # default / ares / slate / mono
  tool_progress: true
  streaming_thinking: true

gateway:
  host: 0.0.0.0
  port: 3848

memory:
  max_size: 50000
  nudge_interval: 50
```

---

## 开发命令

```powershell
# 运行测试
cargo test

# 前端 TypeScript 检查
cd crates/ui && npx tsc --noEmit

# 格式化
cargo fmt
cargo clippy --fix
```

---

## 项目状态

| 组件 | 状态 |
|------|------|
| 核心聊天 + 流式 | ✅ 正常运行 |
| 工具调用（19 个工具） | ✅ 正常运行 |
| Skill 自动创建 | ✅ 正常运行 |
| Memory 系统 | ✅ 基础可用（MemoryStore + nudge） |
| Session DB + FTS5 | ✅ 正常运行 |
| 多提供商 LLM | ✅ 正常运行 |
| React UI（9 个页面） | ✅ 正常运行 |
| HTTP Gateway | ✅ 正常运行 |
| CLI + 斜杠命令 | ✅ 正常运行 |
| 流式中断 | ✅ 正常运行 |
| 平台适配器 | 🔧 仅存根 |
| Skill 自我进化 | 🔜 未开始 |
| ACP 适配器（VS Code） | 🔜 未开始 |

---

## 许可证

MIT — 与 [NousResearch/hermes-agent](https://github.com/NousResearch/hermes-agent) 相同。
