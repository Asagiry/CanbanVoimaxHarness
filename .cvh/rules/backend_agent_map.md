---
description: Agent loops, prompts, compaction and tool executors.
---
# Agent runtime and tools

Agent orchestration assembles prompts and dispatches tools. The ReAct engine coordinates streaming drivers. Tool output spill occurs before insertion into conversation history; historical compaction is an explicit context rewrite. For provider serialization and cache diagnostics, load `backend_config_map`; for persisted history, load `backend_storage_map`.

| File | Responsibility |
| --- | --- |
| `src-tauri/src/agent.rs` | Prompt assembly, main/subagent loops, tool dispatch and output spill. |
| `src-tauri/src/agent/session_registry.rs` | Active & background subagents, completion channels, lifecycle. |
| `src-tauri/src/agent/react_engine.rs` | Shared ReAct streaming, tool dispatch and telemetry drivers. |
| `src-tauri/src/agent/compactor.rs` | Token estimates, compaction reports and history reduction. |
| `src-tauri/src/agent/sanitizer.rs` | Tag stripping, unclosed JSON repairs. |
| `src-tauri/src/agent/telemetry.rs` | Frozen TPS on generation stop, TTFT split. |
| `src-tauri/src/agent/text_match.rs` | CRLF/LF adaptation, whitespace relaxation. |
| `src-tauri/src/agent/tool_schemas.rs` | Tool parameter schemas and rule catalog descriptions. |
| `src-tauri/src/agent/tools/mod.rs` | Tool executors re-exports and dispatch contracts. |
| `src-tauri/src/agent/tools/file_ops.rs` | `read_file`, `write_file`, `edit_file`, `glob`, `grep`. |
| `src-tauri/src/agent/tools/command.rs` | `execute_command` (alias `bash`), timeout, stdio. |
| `src-tauri/src/agent/tools/pipeline.rs` | `advance_task`, `reject_task`, relay handoff. |
| `src-tauri/src/agent/tools/planning.rs` | `todo_write`, `question` (ask_user). |
| `src-tauri/src/agent/tools/rules.rs` | `load_rule` (JIT dynamic rule loader). |
| `src-tauri/src/agent/tools/subagent.rs` | `subagent`, `subagent_bg`, `collect_subagent`, `subagent_list`. |
| `src-tauri/src/agent/tools/tasks.rs` | `create_task`, `create_tasks`, `complete_task`, `list_tasks`, `get_task_history`. |
| `src-tauri/src/agent/tools/web.rs` | `web_search`, `web_fetch`. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src-tauri/src/agent/tests/compactor_tests.rs` — compactor tests.
- `src-tauri/src/agent/tests/engine_tests.rs` — engine tests.
- `src-tauri/src/agent/tests/session_tests.rs` — session tests.
- `src-tauri/src/agent/tests/telemetry_tests.rs` — telemetry tests.
- `src-tauri/src/agent/tests/tools_cmd_tests.rs` — tools cmd tests.
- `src-tauri/src/agent/tests/tools_fs_tests.rs` — tools fs tests.
- `src-tauri/src/agent/tests/tools_pipeline_tests.rs` — tools pipeline tests.
- `src-tauri/src/agent/tests/tools_policy_tests.rs` — tools policy tests.
