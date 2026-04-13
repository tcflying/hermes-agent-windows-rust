# Hermes-Agent-Windows-Rust

**A high-performance AI agent desktop application for Windows, built with Rust + React. Streaming tool calls, multi-provider LLM support, 19 built-in tools, and a 9-page Web UI.**

> **Based on** [NousResearch/hermes-agent](https://github.com/NousResearch/hermes-agent) (MIT License). This is a ground-up Rust rewrite targeting Windows desktop with native WebView2 GUI.

---

## Features

### Core Agent
- **Streaming Tool Calls** — Real-time streaming responses with live tool execution. Resolved a critical parsing bug in the streaming tool_call accumulator (commit `d92ca75`).
- **Interruptible** — Stop any in-progress streaming request instantly.
- **Multi-turn Sessions** — Persistent conversation history with SQLite + FTS5 full-text search.
- **Tool Auto-Skill Generation** — After complex tasks, the agent autonomously creates reusable skills.

### Multi-Provider LLM Support
- MiniMax (default)
- OpenAI (GPT-4 series)
- Anthropic (Claude series)
- models.dev registry with context-length awareness
- Per-conversation model override (`/model` command)

### Built-in Tools (19 total)
| Tool | Description |
|------|-------------|
| `terminal` | Execute PowerShell/cmd commands |
| `file_read` / `file_write` / `file_edit` | Full file operations |
| `list_dir` | Directory listing with tree view |
| `skill_create` / `skill_list` / `skill_view` / `skill_delete` | Skill lifecycle management |
| `memory_save` / `memory_search` | Persistent memory store |
| `browser_navigate` / `web_search` | Web automation |
| `execute_code` | Sandboxed code execution |
| `interrupt` | Streaming interrupt |
| `delegate` | Subagent delegation |

### Frontend UI (9 pages)
- **Chat** — Main conversation interface with tool call cards, streaming markdown, session switcher
- **Files** — Monaco editor with file tree, save/load workflow
- **Terminal** — xterm.js terminal emulator embedded in browser
- **Memory** — Persistent memory editor with section-based storage
- **Skills** — Browse, create, and manage auto-generated skills
- **Dashboard** — Real-time HUD with stats (sessions, skills, messages)
- **Inspector** — Per-session message history and token usage
- **Settings** — Theme switcher (default/ares/slate/mono), provider API keys
- **HUD** — Agent telemetry overlay (total sessions, skills, messages)

### Desktop Ready
- **Tauri 2.x** — Native Windows WebView2 packaging
- **React + Vite** — Fast HMR development, TypeScript throughout
- **Playwright** — E2E test suite included in devDependencies

---

## Quick Start

### Prerequisites
- Rust 1.93+
- Node.js 18+
- Windows 10/11

### Build

```powershell
# Clone your fork
git clone https://github.com/tcflying/hermes-agent.git
cd hermmes-agent

# Build Rust backend
cargo build --release

# Install frontend dependencies
cd crates/ui && npm install && cd ../..
```

### Run

**Option A — Scripts (simplest):**
```powershell
# Double-click or run in terminal:
start.bat    # Starts backend (port 3848) + frontend (port 1420)
stop.bat     # Stops all services
```

**Option B — Manual:**
```powershell
# Backend
./target/release/hermes.exe gateway start

# Frontend (separate terminal)
cd crates/ui && npm run dev
```

- Frontend: http://localhost:1420
- Backend API: http://localhost:3848

---

## Architecture

```
hermes-rs/
├── crates/
│   ├── agent/          # Core agent loop, tool execution, memory, prompt building
│   │   └── src/
│   │       ├── chat.rs          # Main streaming chat handler (FIXED: tool_call parser)
│   │       ├── tools.json        # 19 tool definitions
│   │       ├── memory.rs         # MemoryStore, MemoryManager, MemorySnapshot
│   │       ├── memory_nudge.rs   # Periodic nudge injector
│   │       ├── skill_commands.rs # /skill slash command handler
│   │       ├── iteration.rs      # IterationBudget (max 30 tool calls/turn)
│   │       ├── interrupt.rs      # InterruptFlag for streaming stop
│   │       └── auxiliary_client.rs # Vision + summarization clients
│   ├── cli/            # HermesCLI — slash commands, skin engine, setup wizard
│   ├── config/         # YAML config loader, multi-provider schema
│   ├── gateway/        # Axum HTTP server, SSE streaming, session routing
│   │   └── platforms/ # Stub adapters: Telegram, Discord, Slack, WhatsApp, etc.
│   ├── session/        # SQLite + FTS5 session DB, trajectory saving
│   ├── tool-registry/  # Central tool registry (stub, pending integration)
│   ├── ui/             # React + TypeScript frontend
│   │   └── src/
│   │       ├── pages/  # 9 page components (Chat, Files, Terminal, Memory...)
│   │       ├── components/ # WorkspaceLayout, FileTree, ToolCallCard...
│   │       └── App.tsx # React Router v7 routing
│   └── utils/          # Shared utilities
├── start.bat           # One-click dev startup
├── stop.bat            # Stop all services
└── Cargo.toml          # Workspace root
```

### Tech Stack

| Layer | Technology |
|-------|-----------|
| GUI Shell | Tauri 2.x (WebView2) |
| Frontend | React 18, TypeScript, Vite 6 |
| Backend | Rust, Tokio, Axum 0.8 |
| Database | SQLite + FTS5 (rusqlite, bundled) |
| Terminal | xterm.js |
| Code Editor | Monaco Editor |
| Markdown | react-markdown + shiki |
| HTTP | reqwest (streaming), tower-http |
| CLI | clap (derive) |
| Logging | tracing + tracing-subscriber |

---

## Key Fix: Streaming tool_calls Parser

A critical bug in the streaming tool_call accumulation logic caused `{"error":"invalid function arguments json string"}` on every tool call during streaming responses.

**Root Cause:** The original state machine used `index` field to detect tool call boundaries — but the index was unreliable across providers and multiple concurrent tool calls caused id flush timing errors.

**Fix (commit `d92ca75`):** Replaced index-based detection with `id`-change detection:
- Each incoming tool_call chunk is checked for a new `id`
- When `id` changes, the previous tool call is flushed to the accumulated list
- No mutexes needed for accumulation state — simple owned variables

**File:** `crates/agent/src/chat.rs` lines 183–283

---

## Configuration

Config file: `~/.hermes/config.yaml`

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

## Development

```powershell
# Run tests
cargo test

# TypeScript check (frontend)
cd crates/ui && npx tsc --noEmit

# Format
cargo fmt
cargo clippy --fix
```

---

## Project Status

| Component | Status |
|-----------|--------|
| Core chat + streaming | ✅ Working |
| Tool calling (19 tools) | ✅ Working |
| Skill auto-creation | ✅ Working |
| Memory system | ✅ Basic (MemoryStore, nudge) |
| Session DB + FTS5 | ✅ Working |
| Multi-provider LLM | ✅ Working |
| React UI (9 pages) | ✅ Working |
| HTTP Gateway | ✅ Working |
| CLI + slash commands | ✅ Working |
| Streaming interrupt | ✅ Working |
| Platform adapters | 🔧 Stub only |
| Skill self-evolution | 🔜 Not started |
| ACP adapter (VS Code) | 🔜 Not started |

---

## License

MIT — Same as [NousResearch/hermes-agent](https://github.com/NousResearch/hermes-agent).
