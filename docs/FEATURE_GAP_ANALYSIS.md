# Hermes-RS Feature Gap Analysis

**Date**: 2026-04-13
**Source**: https://github.com/NousResearch/hermes-agent (Python, ~300K LOC)
**Target**: G:\opencode-project\hermes-rs (Rust rewrite)

---

## Executive Summary

Hermes-agent Python has **69 tool files**, **28 agent modules**, **20+ platform adapters**.
Hermes-rs currently has **4 tools**, **1 chat module**, **0 platform adapters functional**, **no memory system**, **no skill system**, **no self-evolution**.

The gap is massive. This document catalogs every missing feature.

---

## What We HAVE (hermes-rs)

| Component | Status | Files |
|-----------|--------|-------|
| Chat with tool calling | Working | `agent/src/chat.rs` |
| 4 tools (terminal, file_read, file_write, list_dir) | Working | `agent/src/chat.rs` |
| Session DB (SQLite) | Working | `session/src/db.rs` |
| Session FTS5 search | Stub | `session/src/search.rs` |
| Config loader | Working | `config/src/loader.rs`, `config/src/schema.rs` |
| HTTP gateway (Axum) | Working | `gateway/src/handlers.rs` |
| SSE streaming | Working | `gateway/src/handlers.rs` |
| Tauri GUI | Working | `ui/src-tauri/src/main.rs` |
| React frontend (8 pages) | Working | `ui/src/pages/*.tsx` |
| CLI binary | Working | `cli/src/cli.rs` |
| Platform adapter stubs | Stub only | `gateway/src/platforms/{telegram,slack,discord}.rs` |
| Tool registry | Stub | `tool-registry/src/registry.rs` |

---

## What We're MISSING — Complete Catalog

### Category 1: Self-Evolution / Memory System (P0)

The killer feature of hermes-agent. Currently **completely absent** in hermes-rs.

#### 1.1 Memory Tool — MEMORY.md + USER.md persistence
- **Python**: `tools/memory_tool.py` (~560 lines)
- Two persistent stores: MEMORY.md (agent notes) and USER.md (user profile)
- Entry delimiter: § (section sign)
- Actions: add, replace, remove, read
- Content scanning for injection/exfiltration threats
- Character limits per store
- Frozen snapshot pattern: system prompt is stable, tool responses show live state
- **hermes-rs**: NOTHING. No memory tool, no memory files, no persistence.

#### 1.2 Memory Provider Interface
- **Python**: `agent/memory_provider.py`
- Abstract base class: `MemoryProvider`
- Methods: `initialize()`, `system_prompt_block()`, `prefetch()`, `sync_turn()`, `get_tool_schemas()`, `handle_tool_call()`
- **hermes-rs**: NOTHING.

#### 1.3 Memory Manager (Orchestrator)
- **Python**: `agent/memory_manager.py` (~362 lines)
- Coordinates builtin + one external provider
- `prefetch_all()` — recall before turns
- `sync_all()` — persist after turns
- `build_system_prompt()` — inject memory into system prompt
- `<memory-context>` fencing
- **hermes-rs**: NOTHING.

#### 1.4 Memory Nudge System
- **Python**: In `run_agent.py` — periodic nudges to persist knowledge
- Agent periodically reminded to save important learnings
- **hermes-rs**: NOTHING.

#### 1.5 Skill Auto-Creation from Experience
- **Python**: `tools/skill_manager_tool.py`, `tools/skills_guard.py`, `tools/skills_sync.py`
- After complex tasks, agent autonomously creates skills
- Skills self-improve during use
- **hermes-rs**: NOTHING.

---

### Category 2: Missing Tools (P0-P1)

Python hermes-agent has 40+ tools. We have 4. Missing:

| Tool | Python File | Priority | Description |
|------|------------|----------|-------------|
| `web_search` | `tools/web_tools.py` | P0 | Web search (Parallel + Firecrawl) |
| `web_extract` | `tools/web_tools.py` | P0 | Extract content from URLs |
| `process` | `tools/process_registry.py` | P1 | Background process management |
| `patch` | `tools/file_tools.py`, `tools/patch_parser.py` | P0 | Fuzzy-matching file patching |
| `search_files` | `tools/file_tools.py` | P0 | File content search (ripgrep-like) |
| `vision_analyze` | `tools/vision_tools.py` | P1 | Image analysis via auxiliary client |
| `image_generate` | `tools/image_generation_tool.py` | P1 | Image generation |
| `skills_list` | `tools/skills_tool.py` | P1 | List available skills |
| `skill_view` | `tools/skills_tool.py` | P1 | View skill content |
| `skill_manage` | `tools/skill_manager_tool.py` | P1 | Create/edit/delete skills |
| `browser_navigate` | `tools/browser_tool.py` | P1 | Browser automation (Browserbase) |
| `browser_snapshot` | `tools/browser_tool.py` | P1 | Browser screenshot |
| `browser_click` | `tools/browser_tool.py` | P1 | Click elements |
| `browser_type` | `tools/browser_tool.py` | P1 | Type into elements |
| `browser_scroll` | `tools/browser_tool.py` | P1 | Scroll page |
| `browser_back` | `tools/browser_tool.py` | P1 | Navigate back |
| `browser_press` | `tools/browser_tool.py` | P1 | Press keys |
| `browser_get_images` | `tools/browser_tool.py` | P1 | Get page images |
| `browser_vision` | `tools/browser_tool.py` | P1 | Vision analysis of page |
| `browser_console` | `tools/browser_tool.py` | P1 | Browser console |
| `text_to_speech` | `tools/tts_tool.py` | P2 | Edge TTS / ElevenLabs / OpenAI TTS |
| `todo` | `tools/todo_tool.py` | P1 | Task planning and tracking |
| `memory` | `tools/memory_tool.py` | P0 | Persistent memory operations |
| `session_search` | `tools/session_search_tool.py` | P1 | FTS5 search across past conversations |
| `clarify` | `tools/clarify_tool.py` | P1 | Ask user clarifying questions |
| `execute_code` | `tools/code_execution_tool.py` | P1 | Execute Python scripts |
| `delegate_task` | `tools/delegate_tool.py` | P1 | Spawn subagents |
| `cronjob` | `tools/cronjob_tools.py` | P1 | Scheduled task management |
| `send_message` | `tools/send_message_tool.py` | P1 | Cross-platform messaging |
| `mixture_of_agents` | `tools/mixture_of_agents_tool.py` | P2 | Multi-agent reasoning |
| `ha_list_entities` | `tools/homeassistant_tool.py` | P2 | Home Assistant entity listing |
| `ha_get_state` | `tools/homeassistant_tool.py` | P2 | Home Assistant state |
| `ha_list_services` | `tools/homeassistant_tool.py` | P2 | Home Assistant services |
| `ha_call_service` | `tools/homeassistant_tool.py` | P2 | Home Assistant service calls |

---

### Category 3: Agent Core Features (P0)

#### 3.1 Context Compression
- **Python**: `agent/context_compressor.py`
- Auto-compresses long conversations to fit token limits
- Pre-pass pruning of tool outputs
- Structured summarization preserving head + tail
- **hermes-rs**: NOTHING. Will crash on long conversations.

#### 3.2 Prompt Caching (Anthropic)
- **Python**: `agent/prompt_caching.py`
- `apply_anthropic_cache_control()` — marks cache_control on system + last few messages
- Cuts prompt cost ~75% on multi-turn
- **hermes-rs**: NOTHING.

#### 3.3 Auxiliary Client
- **Python**: `agent/auxiliary_client.py`
- Routes vision/summarization/extraction to best provider
- Resolution order, fallback strategies, pool-based key selection
- **hermes-rs**: NOTHING.

#### 3.4 System Prompt Builder
- **Python**: `agent/prompt_builder.py` (~988 lines)
- Memory blocks, skills index, context files, platform hints
- SOUL.md loading, subdirectory hints
- Security scanning for injection in context files
- Tool use enforcement guidance
- **hermes-rs**: Simple hardcoded SYSTEM_PROMPT in chat.rs. No dynamic assembly.

#### 3.5 Model Metadata / Token Estimation
- **Python**: `agent/model_metadata.py`
- Fetches context limits from providers
- Token estimation for budgeting
- models.dev registry integration
- **hermes-rs**: NOTHING.

#### 3.6 Error Classifier + Failover
- **Python**: `agent/error_classifier.py`
- Classifies API errors (rate limit, context overflow, auth, etc.)
- Triggers provider failover
- **hermes-rs**: NOTHING.

#### 3.7 Smart Model Routing
- **Python**: `agent/smart_model_routing.py`
- Auto-selects model based on task complexity
- **hermes-rs**: NOTHING.

#### 3.8 Retry with Jitter
- **Python**: `agent/retry_utils.py`
- Exponential backoff with jitter
- **hermes-rs**: Basic retry in chat.rs, no jitter.

---

### Category 4: Gateway / Platform Adapters (P1)

#### 4.1 Platform Adapter Architecture
- **Python**: `gateway/platforms/base.py` (~1400 lines)
- Abstract base: `connect()`, `disconnect()`, `send()`, `send_image()`, `send_voice()`, `send_document()`, `edit_message()`, `send_typing()`
- `MessageEvent` dataclass with text, media, source, reply_to
- Media caching (images, audio, documents)
- Typing indicator loops
- Retry logic with exponential backoff
- **hermes-rs**: Stub files for telegram/slack/discord with no implementation.

#### 4.2 Supported Platforms (20+)
- **Python**: 20 platform adapters
- Telegram, Discord, Slack, WhatsApp, Signal, Mattermost, Matrix
- HomeAssistant, Email, SMS, DingTalk, Feishu, WeCom, Weixin
- BlueBubbles (iMessage), API Server, Webhook
- **hermes-rs**: 0 working adapters.

#### 4.3 Session Source / Session Key
- **Python**: `gateway/session.py`
- `SessionSource`: platform, chat_id, chat_type, user_id, thread_id
- Deterministic session key: `agent:main:{platform}:{chat_type}:{chat_id}:{thread_id}:{user_id}`
- **hermes-rs**: NOTHING.

#### 4.4 Gateway Runner
- **Python**: `gateway/run.py` (~7000+ lines)
- GatewayRunner orchestrates all adapters
- Authorization, command interception, agent spawning
- Background process watchers with notification modes
- Platform reconnection with exponential backoff
- **hermes-rs**: Basic Axum HTTP server only.

#### 4.5 Delivery Router
- **Python**: `gateway/delivery.py`
- Routes cron outputs to platforms
- **hermes-rs**: NOTHING.

#### 4.6 Hook Registry
- **Python**: `gateway/hooks.py`
- Event hooks: gateway:startup, session:start/end, agent:start/step/end, command:*
- **hermes-rs**: NOTHING.

#### 4.7 Pairing Store
- **Python**: `gateway/pairing.py`
- Code-based user authorization
- 8-char codes, 1-hour expiry, rate limiting
- **hermes-rs**: NOTHING.

#### 4.8 Channel Directory
- **Python**: `gateway/channel_directory.py`
- Cached map of reachable channels per platform
- Human-friendly name resolution
- **hermes-rs**: NOTHING.

#### 4.9 Token Locks
- **Python**: `gateway/status.py`
- Prevents multiple gateways using same credential
- `acquire_scoped_lock()` / `release_scoped_lock()`
- **hermes-rs**: NOTHING.

---

### Category 5: Skills System (P1)

#### 5.1 Skill Commands
- **Python**: `agent/skill_commands.py`
- Scans `~/.hermes/skills/`, injects as user message
- Slash-command based triggers
- **hermes-rs**: NOTHING.

#### 5.2 Skills Hub
- **Python**: `tools/skills_hub.py`, `hermes_cli/skills_hub.py`
- Search, browse, install skills from agentskills.io
- **hermes-rs**: NOTHING. UI has SkillsPage.tsx but it's a static mock.

#### 5.3 Skills Config
- **Python**: `hermes_cli/skills_config.py`
- Enable/disable skills per platform
- **hermes-rs**: NOTHING.

#### 5.4 Skills Guard / Sync
- **Python**: `tools/skills_guard.py`, `tools/skills_sync.py`
- Validation, syncing skills across instances
- **hermes-rs**: NOTHING.

---

### Category 6: Safety & Security (P1)

#### 6.1 Approval System
- **Python**: `tools/approval.py`
- Dangerous command detection patterns
- Per-session approval state
- CLI gateway prompts for approval
- **hermes-rs**: NOTHING.

#### 6.2 Content Scanning
- **Python**: `agent/prompt_builder.py`, `tools/memory_tool.py`
- Injection detection in context files and memory
- Invisible unicode detection
- Exfiltration pattern detection
- **hermes-rs**: NOTHING.

---

### Category 7: Infrastructure (P1-P2)

#### 7.1 Cron Scheduler
- **Python**: `cron/scheduler.py`, `cron/jobs.py`
- Scheduled task execution with platform delivery
- **hermes-rs**: NOTHING.

#### 7.2 Batch Runner
- **Python**: `batch_runner.py`
- Parallel batch processing of prompts
- **hermes-rs**: NOTHING.

#### 7.3 MCP Client
- **Python**: `tools/mcp_tool.py` (~1050 lines)
- Dynamic tool discovery from MCP servers
- stdio and HTTP transports
- OAuth support
- **hermes-rs**: NOTHING.

#### 7.4 MCP Server Bridge
- **Python**: `mcp_serve.py`
- Exposes hermes tools as MCP server
- **hermes-rs**: NOTHING.

#### 7.5 Terminal Environments
- **Python**: `tools/environments/` — 6 backends
- local, Docker, SSH, Daytona, Singularity, Modal
- Serverless persistence
- **hermes-rs**: Only local via `std::process::Command`.

#### 7.6 RL Training Integration
- **Python**: `tools/rl_training_tool.py`, `environments/`
- Atropos RL environments
- **hermes-rs**: NOTHING.

#### 7.7 Trajectory Saving
- **Python**: `agent/trajectory.py`
- Save agent trajectories for analysis/training
- **hermes-rs**: NOTHING.

#### 7.8 Usage/Pricing Estimation
- **Python**: `agent/usage_pricing.py`
- Token usage tracking and cost estimation
- **hermes-rs**: NOTHING.

#### 7.9 ACP Adapter (VS Code / Zed / JetBrains)
- **Python**: `acp_adapter/`
- Editor integration
- **hermes-rs**: NOTHING.

---

### Category 8: CLI Features (P1)

#### 8.1 Slash Commands (40+)
- **Python**: `hermes_cli/commands.py` — COMMAND_REGISTRY
- 40+ slash commands: /new, /model, /background, /approve, /compress, /usage, /insights, /retry, /undo, /personality, /skills, /platforms, /status, etc.
- **hermes-rs**: Basic CLI with `gateway start` only.

#### 8.2 Skin/Theme Engine
- **Python**: `hermes_cli/skin_engine.py`
- Data-driven CLI theming (default, ares, mono, slate)
- User skins via YAML files
- **hermes-rs**: NOTHING.

#### 8.3 Setup Wizard
- **Python**: `hermes_cli/setup.py`
- Interactive setup for API keys, models, platforms
- **hermes-rs**: NOTHING.

#### 8.4 Display / Spinner
- **Python**: `agent/display.py`
- KawaiiSpinner with animated faces
- Tool preview formatting
- Activity feed
- **hermes-rs**: Basic colored ANSI logging.

---

## Priority Implementation Order

### Wave 1 — Core Agent (Makes it usable as an agent)
1. Memory system (MEMORY.md + USER.md + tool) [P0]
2. File patching with fuzzy match [P0]
3. File content search [P0]
4. Web search + extract [P0]
5. Dynamic system prompt builder [P0]
6. Context compression [P0]

### Wave 2 — Extended Tools
7. Todo tool [P1]
8. Session search tool [P1]
9. Delegate/subagent tool [P1]
10. Code execution tool [P1]
11. Approval system [P1]
12. Browser automation [P1]

### Wave 3 — Gateway
13. Platform adapter trait + base implementation [P1]
14. Telegram adapter [P1]
15. Session source/key routing [P1]
16. Authorization + pairing [P1]

### Wave 4 — Self-Evolution
17. Memory nudge system [P1]
18. Skill auto-creation [P1]
19. Skill management tools [P1]
20. Skills Hub integration [P1]

### Wave 5 — Infrastructure
21. Cron scheduler [P1]
22. MCP client [P1]
23. Auxiliary client (vision) [P1]
24. Prompt caching [P1]
25. More platform adapters [P2]

---

## Current File Counts

| | Python (hermes-agent) | Rust (hermes-rs) | Gap |
|--|----------------------|-------------------|-----|
| Tool files | 69 | 0 (inline in chat.rs) | -69 |
| Agent modules | 28 | 3 (chat, interrupt, iteration) | -25 |
| Platform adapters | 20 | 3 (stubs) | -20 |
| CLI commands | 40+ | 1 (gateway start) | -39 |
| Total tools registered | 40+ | 4 | -36+ |
| Lines of code | ~300K | ~5K | -295K |
