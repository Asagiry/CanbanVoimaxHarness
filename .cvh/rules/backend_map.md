---
description: Backend entrypoints and topic-map router.
---
# Backend map

Rust/Tauri code lives in `src-tauri/src/`. Load only the topic needed for the task; known files can be opened directly.

| Task | Rule to load | Start here |
| --- | --- | --- |
| Prompts, agent loop, compaction, tools and subagents | `load_rule("backend_agent_map")` | `src-tauri/src/agent.rs` (`session_registry.rs`) |
| Providers, model fetch, cache routing, config and tool policy | `load_rule("backend_config_map")` | `src-tauri/src/llm/client.rs` (`providers.rs`) |
| Start/stop/steer, task lifecycle, stage hooks and Git | `load_rule("backend_tasks_map")` | `src-tauri/src/commands/tasks/runner.rs` (`git/mod.rs`) |
| Domain states/IDs, dialogue history and exact context snapshots | `load_rule("backend_storage_map")` | `src-tauri/src/db/context_repo.rs` |
| Desktop bootstrap, IPC wiring, notifications and logging | `load_rule("backend_platform_map")` | `src-tauri/src/lib.rs` |

Runtime path: Tauri commands → task runner → agent/ReAct engine (`session_registry.rs`) → provider client (`providers.rs`) and tools. SQLite repositories (`context_repo.rs`) and Git worktree modules (`git/mod.rs`) own persistence and isolation; typed events carry updates to the frontend. Use `load_rule("frontend_map")` for UI consumers.

Keep exact file routes current in the relevant topic map when modules move. Verification commands and build policy belong to the active role's `sensors` rule.
