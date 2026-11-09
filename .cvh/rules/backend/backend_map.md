---
description: Backend entrypoints and topic-map router.
---
# Backend map

Rust/Tauri code lives in `src-tauri/src/`. Load only the topic needed for the task; known files can be opened directly.

| Task | Rule to load |
| --- | --- |
| Prompts, agent loop, compaction, tools and subagents | `load_rule("backend_agent_map")` |
| Providers, model fetch, cache routing, config and tool policy | `load_rule("backend_config_map")` |
| Start/stop/steer, task lifecycle, stage hooks and Git | `load_rule("backend_tasks_map")` |
| Domain states/IDs, dialogue history and exact context snapshots | `load_rule("backend_storage_map")` |
| Desktop bootstrap, IPC wiring, notifications and logging | `load_rule("backend_platform_map")` |

Exact file routes live only in the topic maps above — this router does not repeat them. Start domain work at `src-tauri/src/domain/ids.rs` and `src-tauri/src/domain/state.rs`. Runtime path: Tauri commands → task runner → agent/ReAct engine → provider client and tools. Use `load_rule("frontend_map")` for UI consumers.

Keep exact file routes current in the relevant topic map when modules move. Verification commands and build policy belong to the active role's `sensors` rule.
