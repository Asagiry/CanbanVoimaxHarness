---
description: Agent loops, prompts, compaction, ReAct engine, and tool executors.
---
# Agent runtime and tools

Agent orchestration assembles prompts and dispatches tools. The ReAct engine coordinates streaming drivers. Tool output spill occurs before insertion into conversation history; historical compaction is an explicit context rewrite. All submodules strictly observe the <=500 lines ceiling. For provider serialization and cache diagnostics, load `backend_config_map`; for persisted history, load `backend_storage_map`.

| File | Responsibility |
| --- | --- |
| `src-tauri/src/agent.rs` | Public facade re-exporting agent engine, types, and subagent runtime. |
| `src-tauri/src/agent/types.rs` | Agent types (`AgentSessionDriver`, `TurnOutcome`, `ToolContext`, etc.). |
| `src-tauri/src/agent/engine.rs` | Core agent execution loop and turn driver. |
| `src-tauri/src/agent/engine_context.rs` | In-memory conversation state assembly and context pruning. |
| `src-tauri/src/agent/engine_tools.rs` | Tool execution orchestration and spill wrapping within the agent loop. |
| `src-tauri/src/agent/tool_dispatch.rs` | Policy enforcement and canonical tool call routing to executors. |
| `src-tauri/src/agent/prompts.rs` | System prompt assembly, persona injection, and rule compilation. |
| `src-tauri/src/agent/spill.rs` | Output spill threshold management (>500 lines / >64KB to `.cvh/spill/`). |
| `src-tauri/src/agent/command_runner.rs` | Subprocess execution with timeout, CP866 fallback, and stdio pipes. |
| `src-tauri/src/agent/steering.rs` | User steer queue injection into running agent loop. |
| `src-tauri/src/agent/thinking_parser.rs` | `<thought>` and reasoning block token extraction. |
| `src-tauri/src/agent/rule_tools.rs` | Dynamic rule discovery and frontmatter extraction for `load_rule`. |
| `src-tauri/src/agent/subagent_runtime.rs` | Subagent runtime coordination and parent-child session binding. |
| `src-tauri/src/agent/subagent_setup.rs` | Subagent workspace isolation, prompt setup, and tool allowlist. |
| `src-tauri/src/agent/subagent_turn.rs` | Subagent ReAct turn execution and step limits. |
| `src-tauri/src/agent/subagent_synth.rs` | Subagent completion report synthesis and handoff assembly. |
| `src-tauri/src/agent/subagent_exec.rs` | Subagent task execution loop and channel dispatch. |
| `src-tauri/src/agent/subagent_retry.rs` | Subagent transient failure retry with exponential backoff. |
| `src-tauri/src/agent/session_registry.rs` | Active & background subagents, completion channels, and lifecycle registry. |
| `src-tauri/src/agent/compactor.rs` | Public facade re-exporting context compaction orchestrator and extractor. |
| `src-tauri/src/agent/compactor/tokens.rs` | Fast token estimation and threshold comparison. |
| `src-tauri/src/agent/compactor/extractor.rs` | Typed tool call summary extractor (CC=1) replacing raw tool logs. |
| `src-tauri/src/agent/compactor/evaporation.rs` | Zero-LLM fast evaporation of stale tool outputs (>500 chars older than 2 turns). |
| `src-tauri/src/agent/compactor/multi_tier.rs` | Multi-tier compaction passes targeting 75% context reduction. |
| `src-tauri/src/agent/compactor/orchestrator.rs` | Compaction pass coordinator and snapshot persistence synchronization. |
| `src-tauri/src/agent/compactor/report.rs` | Compaction token savings reporting and telemetry data structures. |
| `src-tauri/src/agent/react_engine.rs` | Public facade re-exporting generic `ReActEngine<Driver>`. |
| `src-tauri/src/agent/react_engine/engine.rs` | Generic ReAct engine core loop parameterization. |
| `src-tauri/src/agent/react_engine/drivers.rs` | Session drivers implementing `AgentSessionDriver` trait. |
| `src-tauri/src/agent/react_engine/stream_consumer.rs` | Unified SSE streaming chunk consumer, TTFT calculation, and telemetry. |
| `src-tauri/src/agent/sanitizer.rs` | `sanitize_unclosed_tool_calls` for broken JSON tool frames. |
| `src-tauri/src/agent/telemetry.rs` | Frozen TPS on generation stop, TTFT split, and non-decaying cadence metrics. |
| `src-tauri/src/agent/text_match.rs` | Public facade for text similarity and edit matching. |
| `src-tauri/src/agent/text_match/diff_mismatch.rs` | Unified diff mismatch diagnostics and reporting. |
| `src-tauri/src/agent/text_match/replace.rs` | 5-tier fuzzy text matching and replacement algorithm. |
| `src-tauri/src/agent/tool_schemas.rs` | Tool parameter schemas and rule catalog descriptions. |
| `src-tauri/src/agent/tools/mod.rs` | Tool executors re-exports and dispatch contracts. |
| `src-tauri/src/agent/tools/file_ops.rs` | `read_file`, `write_file`, `edit_file`, `glob`, `grep`. |
| `src-tauri/src/agent/tools/command.rs` | `execute_command` (alias `bash`), timeout, stdio. |
| `src-tauri/src/agent/tools/pipeline.rs` | `advance_task`, `reject_task`, relay handoff. |
| `src-tauri/src/agent/tools/planning.rs` | `todo_write`, `question` (ask_user). |
| `src-tauri/src/agent/tools/rules.rs` | `load_rule` (JIT dynamic rule loader). |
| `src-tauri/src/agent/tools/subagent.rs` | Public facade for subagent tools (`subagent`, `subagent_bg`, `collect_subagent`, `subagent_list`). |
| `src-tauri/src/agent/tools/subagent/background.rs` | Background subagent asynchronous execution (`subagent_bg`). |
| `src-tauri/src/agent/tools/subagent/collect.rs` | Collection and report consumption for background subagents (`collect_subagent`). |
| `src-tauri/src/agent/tools/subagent/guards.rs` | Concurrency guards, depth limits, and cycle prevention for subagents. |
| `src-tauri/src/agent/tools/subagent/list.rs` | Subagent status query tool (`subagent_list`). |
| `src-tauri/src/agent/tools/subagent/run.rs` | Synchronous blocking subagent invocation (`subagent`). |
| `src-tauri/src/agent/tools/tasks.rs` | Public facade for task tools (`create_task`, `complete_task`, `list_tasks`, etc.). |
| `src-tauri/src/agent/tools/tasks/params.rs` | Argument parsing and normalization for task tools. |
| `src-tauri/src/agent/tools/tasks/create.rs` | `create_task` and `create_tasks` execution handlers. |
| `src-tauri/src/agent/tools/tasks/complete.rs` | `complete_task` execution handler. |
| `src-tauri/src/agent/tools/tasks/query.rs` | `list_tasks` and `get_task_history` execution handlers. |
| `src-tauri/src/agent/tools/web.rs` | `web_search`, `web_fetch`. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src-tauri/src/agent/tests/mod.rs` — Agent test suite router.
- `src-tauri/src/agent/tests/compactor_tests.rs` — compactor tests.
- `src-tauri/src/agent/tests/engine_tests.rs` — engine tests.
- `src-tauri/src/agent/tests/session_tests.rs` — session tests.
- `src-tauri/src/agent/tests/telemetry_tests.rs` — telemetry tests.
- `src-tauri/src/agent/tests/tools_cmd_tests.rs` — tools cmd tests.
- `src-tauri/src/agent/tests/tools_fs_tests.rs` — tools fs tests.
- `src-tauri/src/agent/tests/tools_pipeline_tests.rs` — tools pipeline tests.
- `src-tauri/src/agent/tests/tools_policy_tests.rs` — tools policy tests.
