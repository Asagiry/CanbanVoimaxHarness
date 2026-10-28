pub mod compactor;
pub use compactor::*;

use crate::config::{
    agent_allows_tool, is_tool_allowed, load_agent_setting, load_tool_policies,
    ToolPolicyConfig,
};
use crate::db::ActivityItem;
use crate::hooks::kill_process_tree;
use crate::llm::{ChatMessage, LlmClient, OpenAIChatRequest, Role, StreamChunk, ToolCall, ToolDefinition};
use anyhow::{bail, Context, Result};
use tauri::{AppHandle, Emitter, Manager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

pub type QuestionSenderMap = HashMap<String, tokio::sync::oneshot::Sender<String>>;

pub static QUESTION_REGISTRY: LazyLock<Arc<tokio::sync::Mutex<QuestionSenderMap>>> =
    LazyLock::new(|| Arc::new(tokio::sync::Mutex::new(HashMap::new())));

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    #[serde(default = "default_todo_id")]
    pub id: String,
    pub content: String,
    pub status: String,
}

fn default_todo_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "stageId", alias = "stage_id", alias = "columnId")]
    pub stage_id: String,
    #[serde(rename = "boardId", alias = "board_id", default)]
    pub board_id: Option<String>,
    #[serde(rename = "worktreePath", alias = "worktree_path", default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(rename = "customFlow", alias = "custom_flow", default)]
    pub custom_flow: Option<Vec<String>>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(rename = "parentId", alias = "parent_id", default)]
    pub parent_id: Option<String>,
    #[serde(rename = "subtaskIndex", alias = "subtask_index", default)]
    pub subtask_index: Option<u32>,
    #[serde(rename = "waitingForSubtaskId", alias = "waiting_for_subtask_id", default)]
    pub waiting_for_subtask_id: Option<String>,
    #[serde(rename = "handoverSummary", alias = "handover_summary", default)]
    pub handover_summary: Option<String>,
    #[serde(rename = "coverImage", alias = "cover_image", default)]
    pub cover_image: Option<String>,
    #[serde(default)]
    pub attachments: Option<Vec<String>>,
    #[serde(default)]
    pub todos: Option<Vec<TodoItem>>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "currentTokens", alias = "current_tokens", default)]
    pub current_tokens: Option<usize>,
    #[serde(
        rename = "skipWorktree",
        alias = "skip_worktree",
        alias = "skipBacklogHook",
        alias = "skip_backlog_hook",
        default
    )]
    pub skip_worktree: Option<bool>,
    #[serde(rename = "createdAt", alias = "created_at", default)]
    pub created_at: i64,
    #[serde(rename = "updatedAt", alias = "updated_at", default)]
    pub updated_at: i64,
    #[serde(rename = "completedAt", alias = "completed_at", default)]
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub position: Option<f64>,
    #[serde(rename = "hookStates", alias = "hook_states", default)]
    pub hook_states: Option<serde_json::Value>,
}

pub type KanbanTask = Task;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum AgentEvent {
    Token(String),
    Output(String),
    Thought(String),
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    ToolResult {
        id: String,
        name: String,
        result: String,
        success: bool,
    },
    TurnStart {
        turn: usize,
    },
    Telemetry {
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_creation_tokens: u64,
        cache_hit_rate: f32,
        duration_ms: f64,
        tps: f32,
        ttft_ms: f64,
        system_tokens: u64,
        tool_tokens: u64,
    },
    Compacted {
        saved_chars: usize,
        estimated_saved_tokens: usize,
        evaporated_count: usize,
        tokens_before: usize,
        tokens_after: usize,
    },
    Finished {
        final_response: String,
    },
    TaskUpdated {
        task_id: String,
        todos: Vec<TodoItem>,
    },
    Aborted,
    Error(String),
}

/// Lean router System Persona for `cvh-dev` (≤150 lines, ≤8KB).
/// Progressive disclosure: identity + triggers + paths + skeletons here;
/// full detail lives lazy in `.cvh/agents/<role>/rules/*.md` by slug.
pub const BUILTIN_CVH_DEV_PROMPT: &str = r#"# Role: CVH-DEV (Project Configuration & Environment Specialist)
You are EXCLUSIVELY responsible for project configuration, rules, and architecture in CanbanVoimaxHarness (`.cvh/`):
- Board configurations (`.cvh/boards.json`) and runtime pipelines (`.cvh/pipeline.json`).
- Agent personas, roles, and modular rules (`.cvh/agents/<role>/persona.md`, `rules/*.md`).
- Tool permission policies (`.cvh/settings/tool-policy.json`) and stage transition hooks (`hooks/`).
You do NOT create task cards or manage user tasks (reserved for users and the pipeline orchestrator). Your objective is preparing the optimal execution environment for the engineering team.

Identity:
- Columns = Agents: board IS the control plane; column `persona: <role>` on-enter runs `run_agent(taskId)`; flow `backlog` -> ... -> `ready`.
- Configuration-only: you may inspect and configure .cvh/ via read_file/write_file/edit_file/glob/grep/execute_command/web_search/web_fetch/todo_write/subagent/advance_task. You do NOT create or complete task cards.
- 1 ticket = 1 vertical slice = 1 commit; tiers T0-spec / T1-scaffold / T2-verify / T3-commit; redact secrets at ingest, never log keys.
Overview: bootstrap or repair CVH harness (boards, pipeline, personas, policy, hooks) then stop. No monolith prompt, no docs dump.
When NOT: not a coder for feature logic; not a reviewer (separate clean context); not a docs writer; not a task creator or task manager. If task needs code, delegate to `developer`, if audit to `reviewer`.

Paths (repo-root relative, single source of truth):
- `.cvh/boards.json {activeBoardId, boards[{id,name,columns}]}` defines `development` + `kanban` boards; `activeBoardId` selects one.
- `.cvh/pipeline.json {id,title,columns[{id,title,persona,gatekeeperSensor,allowedTools,terminal}]}` mirrors active board columns.
- `.cvh/settings/tool-policy.json {globalEnabled, stageOverrides}` is the only policy file; `.cvh/agents/<role>/persona.md` (legacy `.cvh/agents/<role>.md` replaced) + `<role>/setting {contextWindow,model,temperature}` + `<role>/rules/*.md` hold personas; `.cvh/tools/<tool>.json` custom tools; `.cvh/hooks/` transition scripts; task work only in `worktrees/<slug>`.

Frontmatter template for every generated `.cvh/agents/<role>/persona.md` (keep ≤30 lines, link detail, never copypaste):
```yaml
---
name: <role>
description: Use PROACTIVELY when <trigger condition for this role>
model: inherit
scope: files-only | read-only | gated-writes
activation: on-enter <column-id> runs run_agent(<task-id>)
---
```

Identity: 3 bullets max (who, does, refuses).
Load: rules/<topic>.md (progressive disclosure, each rule file ≤8KB).
- If asked to configure boards: read rules/boards.md (.cvh/agents/cvh-dev/rules/boards.md).
- If asked to configure agents/personas: read rules/agents.md (.cvh/agents/cvh-dev/rules/agents.md).
- If asked to configure hooks: read rules/hooks.md (.cvh/agents/cvh-dev/rules/hooks.md).

Boards skeleton (2 boards, minimal):
```json
{"activeBoardId":"development","boards":[{"id":"development","name":"Development","columns":[]},{"id":"kanban","name":"Kanban","columns":[]}]}
```
Pipeline skeleton (ids/order must match board; backlog first, ready last):
```json
{"id":"development","title":"Development","columns":[{"id":"dev","title":"Dev","persona":"developer"},{"id":"review","title":"Review","persona":"reviewer","gatekeeperSensor":"cargo check"},{"id":"ready","title":"Ready","terminal":true}]}
```
Tool-policy skeleton (authoritative stageOverrides per role/column; tools dynamically assembled in backend):
```json
{"stageOverrides":{"dev":{"read_file":true,"write_file":true,"edit_file":true,"advance_task":true},"review":{"read_file":true,"edit_file":false,"write_file":false,"advance_task":true}}}
```

Rules (apply to every step):
- English-Only Personas & Rules Invariant: You MUST author ALL persona definitions (.cvh/agents/<role>/persona.md) and modular rule files (.cvh/agents/<role>/rules/*.md, .cvh/rules/*.md) STRICTLY in concise, professional English. Never write rule or persona files in Russian. English achieves 2.5-3x denser BPE token compression, saving thousands of prompt tokens on every agent step and maximizing cache hit rates. Your chat commentary to the user can be in Russian, but all persistent project rules, system prompts, and personas MUST be 100% English.
- Initial Board Protection: The initial board 'cvh-board' must NEVER be deleted or renamed. When configuring new boards (e.g. 'fast', 'complex'), keep 'cvh-board' intact or configure its columns.
- Board Reconfiguration Invariant: When creating, renaming, or modifying boards in .cvh/boards.json or .cvh/pipeline.json, NEVER delete the board or column where your current task is running. If you create new boards, ensure all existing tasks in SQLite WAL (.cvh/harness.db) have their boardId cleanly preserved or mapped to the intended boards.
- Hooks/policy live outside prompt: read them from disk, never inline full dumps; tiers: files-only (glob/read), read-only (+grep/fetch), gated-writes (+edit/write/execute inside worktree after sensor).
- Normalize (commands.rs:47): backlog always first if present, ready always last, middle keeps order; duplicate column id returns Err and blocks move.
- Sync boards<->pipeline: same column ids/order; boards.json is index, pipeline.json is active execution copy.
- 1:1 Invariants: column.persona in boards/pipeline MUST STRICTLY MATCH directory name .cvh/agents/<role>/ and frontmatter name.
- 1:1 Invariants: stageOverrides.<stage_id> in tool-policy.json MUST STRICTLY MATCH column.id.
- Write ONLY inside `worktrees/<slug>`; from repo root you may touch only `.cvh/` (boards/pipeline/personas/tools/hooks); never write outside worktree; every file op goes via `resolve_safe_path` (strip leading /\, deny `:` drive-absolute, deny `..` escape).
- Rationalization vs Reality: state only what sensors prove; no invented files, no assumed green build.

Initializer flow (empty project):
1. Explore: glob/read `package.json`/`Cargo.toml`/`pyproject.toml`/`go.mod` to detect stack + build/test cmds.
2. Spec: draft minimal boards.json + pipeline.json mapping statuses to columns (todo->backlog, review, done->ready).
3. Scaffold: write personas (frontmatter above) + `<role>/rules/*.md` + hooks + sensors.
4. Verify before declare done.

Process (Explore->Plan->Code->Verify->Commit): Explore (glob/grep), Plan: ALWAYS call todo_write on Turn 1 or Turn 2 to publish your complete step-by-step execution plan (e.g. 1. Explore stack, 2. Design boards & pipeline, 3. Create personas, 4. Configure tool policies & hooks, 5. Verify). Mark each todo as completed as you finish it. Code (edit in worktree), Verify (gate sensors: `cargo check`, `cargo clippy -- -D warnings`, `tsc`, `vite build`), Commit (1 slice = 1 commit).
- Ironclad Completion Rule: Before calling advance_task, you MUST mark all TODO items as completed via todo_write. After completing all planned items, you MUST cease tool calls and provide a terminal findings report, OR invoke advance_task(summary) to hand over to the next stage. NEVER re-publish completed items in todo_write or enter recursive planning loops!

Verdicts (reviewer is separate clean context; personas never invoke personas): `APPROVED` = auto-advance via routes.on_approved; `APPROVED_WITH_CAVEATS` = advance + log notes; `CHANGES_REQUESTED` = unlock dev, return `file:line -> fix` checklist, re-run sensors.
Key patterns/recipes: scaffold-then-sense; sensor-first promotion (gatekeeperSensor must exit 0 or move rejected); spill large outputs (>100 lines/8KB) to `.cvh/spill/`, return head/tail 20.
Pitfalls: monolith agent.md (>30 lines); copypaste instead of Load-link; writing outside worktree; re-enabling globally-disabled tool; stale board/pipeline drift.
What to ask when blocked: stack + build cmd, target board template, allowed tools per stage, sensor to gate on.
Red Flags: prompt >150 lines; any rule telling you to read root AGENTS.md/.agents/skills/docs into context; absolute path outside worktree; `terminal:true` missing on ready; duplicate column id.
Verification[evidence]: paste `cargo check`, `cargo clippy -- -D warnings`, changed files as `path:line`, verdict token; no evidence = no APPROVED.
Instructions for Empty & New Projects: run Initializer flow above; Columns = Agents invariant holds from turn 1.
"#;

pub const BUILTIN_DEVELOPER_PROMPT: &str = r#"# Role Persona: developer (Software Engineer)
Identity & Core Directives:
- Specialized software engineer working strictly inside isolated git worktree (`worktrees/<task-slug>`).
- Absolute Invariant: NEVER write or edit code outside the active worktree directory.
- Ponytail Minimalism & Anti-Slop: zero dead code, no unused imports, no placeholder functions, clean idiomatic architecture.
- Strict Error Handling: zero naked unwraps in production paths; use typed errors (`Result<T, AppError>`, `anyhow`, or `thiserror`).

Workflow (Explore -> Plan -> Code -> Verify -> Commit):
1. Explore: locate relevant files using `glob` and `grep`. Never assume paths or API signatures.
2. Plan: formulate targeted checklist with `path:line` targets before editing.
3. Code: execute surgical edits inside `worktrees/<task-slug>`.
4. Verify with Ground Truth Sensors:
   - Backend/Rust: `cargo check` and `cargo clippy -- -D warnings` must exit 0.
   - Frontend/TS: `pnpm run build` (`tsc && vite build`) must exit 0.
   - Subprocesses: test real subprocesses and verify stdout/stderr without assuming success.
5. Commit: 1 atomic task = 1 feature commit. Never commit broken or unverified code.

Rationalization vs Reality:
- State only what runtime sensors prove. If a sensor fails, report the exact error and fix root causes immediately.
"#;

/// Zero-Turn Memory Priming: compiles strictly static SystemMessage from the
/// role Persona (directory `.cvh/agents/<role>/persona.md` → built-in fallback), per-role
/// `.cvh/agents/<role>/rules/*.md` (alphabetical) and Tool Instructions.
/// Strictly static System Prompt: Zero dynamic timestamps, zero random UUIDs in the System prompt to prevent cache invalidation.
/// Dynamic context (task ID, title, description, worktree path) belongs exclusively in the user message.
/// NOTE: root-level `AGENTS.md` and `.agents/skills` are deliberately NOT
/// injected here — the model receives only `.cvh` persona + rules + tool instructions
/// (custom tools themselves still load from `.cvh/tools` via get_all_tools).
pub async fn compile_primed_system_prompt(
    repo_root: &Path,
    role: &str,
) -> Result<String> {
    compile_primed_system_prompt_with_tools(repo_root, role, None).await
}

/// Determines if a tool is active/permitted for an agent, honoring the Two-Level Tool Policy (§ 2.5)
/// and resolving task card operations (create_task / complete_task) based on explicit overrides.
pub fn is_tool_active_for_agent(
    name: &str,
    tool_policy: &crate::config::ToolPolicyConfig,
    role: &str,
    stage_id: Option<&str>,
    _agent_allowed_tools: Option<&[String]>,
) -> bool {
    let effective_stage = stage_id.unwrap_or(role);
    let is_orch = role.to_lowercase().contains("orchestrat")
        || role.to_lowercase().contains("оркестрат")
        || effective_stage.to_lowercase().contains("orchestrat")
        || effective_stage.to_lowercase().contains("оркестрат")
        || effective_stage == "backlog";

    let is_card_op = name == "create_task" || name == "create_tasks" || name == "complete_task";
    if is_card_op && !is_orch {
        let stage_override = tool_policy
            .stage_overrides
            .get(effective_stage)
            .and_then(|m| m.get(name).or_else(|| m.get(crate::config::normalize_tool_name(name))))
            .copied();

        let role_override = tool_policy
            .stage_overrides
            .get(role)
            .and_then(|m| m.get(name).or_else(|| m.get(crate::config::normalize_tool_name(name))))
            .copied();

        let alt_override = if stage_override.is_none() && role_override.is_none() {
            let alt = match role {
                "developer" | "cvh-dev" => Some("dev"),
                "dev" => Some("developer"),
                "reviewer" => Some("review"),
                "review" => Some("reviewer"),
                "retro" | "retrospective" => Some("retro"),
                _ => None,
            };
            alt.and_then(|a| {
                tool_policy
                    .stage_overrides
                    .get(a)
                    .and_then(|m| m.get(name).or_else(|| m.get(crate::config::normalize_tool_name(name))))
                    .copied()
            })
        } else {
            None
        };

        let explicit_override = role_override.or(stage_override).or(alt_override);
        match explicit_override {
            Some(true) => {
                // Explicitly allowed in tool policy! Proceed to standard is_tool_allowed checks.
            }
            Some(false) | None => {
                return false;
            }
        }
    }

    if !is_tool_allowed(tool_policy, Some(effective_stage), name) {
        return false;
    }
    if tool_policy.stage_overrides.contains_key(role)
        && !is_tool_allowed(tool_policy, Some(role), name)
    {
        return false;
    }
    // ToolPolicyConfig (stage_overrides / per-role) is the authoritative source of truth.
    // Persona YAML frontmatter 'tools: [...]' is no longer required or enforced as a gate.
    true
}

pub async fn compile_primed_system_prompt_with_tools(
    repo_root: &Path,
    role: &str,
    active_tools: Option<&[String]>,
) -> Result<String> {
    let mut sections = Vec::new();

    // Resolve active tool names: either explicitly provided, or dynamically from tool-policy
    let resolved_tools: Vec<String> = if let Some(tools) = active_tools {
        tools.to_vec()
    } else {
        let tool_policy = load_tool_policies(repo_root).await.unwrap_or_default();
        let agent_setting = load_agent_setting(repo_root, role).await;
        crate::config::CANONICAL_TOOLS
            .iter()
            .filter(|&&name| {
                is_tool_active_for_agent(
                    name,
                    &tool_policy,
                    role,
                    None,
                    agent_setting.allowed_tools.as_deref(),
                )
            })
            .map(|&s| s.to_string())
            .collect()
    };

    // 1. Role Persona: directory path (.cvh/agents/<role>/persona.md) → built-in fallback.
    // YAML frontmatter does NOT store allowed tools; any legacy 'tools:' line is stripped.
    let persona_path = crate::config::agent_persona_path(repo_root, role);
    if persona_path.is_file() {
        if let Ok(persona_text) = tokio::fs::read_to_string(&persona_path).await {
            let sanitized_persona = crate::config::strip_frontmatter_tools(&persona_text);
            sections.push(format!("# Role Persona: {}\n\n{}", role, sanitized_persona.trim()));
        } else if role == "cvh-dev" {
            let sanitized = crate::config::strip_frontmatter_tools(BUILTIN_CVH_DEV_PROMPT);
            sections.push(sanitized);
        } else if role == "developer" {
            let sanitized = crate::config::strip_frontmatter_tools(BUILTIN_DEVELOPER_PROMPT);
            sections.push(sanitized);
        } else {
            sections.push(format!("# Role Persona: {}\nYou are an expert AI agent in {}.", role, role));
        }
    } else if role == "cvh-dev" {
        let sanitized = crate::config::strip_frontmatter_tools(BUILTIN_CVH_DEV_PROMPT);
        sections.push(sanitized);
    } else if role == "developer" {
        let sanitized = crate::config::strip_frontmatter_tools(BUILTIN_DEVELOPER_PROMPT);
        sections.push(sanitized);
    } else {
        sections.push(format!("# Role Persona: {}\nYou are an expert AI agent in {}.", role, role));
    }

    // 1b. Per-agent rules: `.cvh/agents/<role>/rules/*.md`, alphabetical.
    // Modular Rules (JIT On-Demand): If `load_rule` is active, decouple rule bodies from prompt to optimize tokens.
    let has_load_rule = resolved_tools.iter().any(|t| t == "load_rule");
    if has_load_rule {
        sections.push(
            "# Modular Rules (JIT On-Demand)\nArchitectural standards, domain invariants, and tech guidelines are decoupled from the prompt to optimize tokens.\nUse the `load_rule` tool to dynamically load rules when working on specific topics or technologies.".to_string(),
        );
    } else {
        let rules_dir = crate::config::agent_rules_dir(repo_root, role);
        if rules_dir.is_dir() {
            if let Ok(mut entries) = tokio::fs::read_dir(&rules_dir).await {
                let mut rules: Vec<(String, String)> = Vec::new();
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    if path.extension().and_then(|e| e.to_str()) != Some("md") {
                        continue;
                    }
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        rules.push((fname, content));
                    }
                }
                rules.sort_by(|a, b| a.0.cmp(&b.0));
                for (fname, content) in rules {
                    sections.push(format!("## Rule: {}\n\n{}", fname, content.trim()));
                }
            }
        }
    }

    // 2. Dynamic Tool Instructions & Execution Mandates ("отдельная склейка" backend-authoritative)
    let mut tool_sections = Vec::new();
    let tools_line = format!("tools: [{}]", resolved_tools.join(", "));
    tool_sections.push(format!(
        "# Authorized Tools (Backend-Authoritative)\n{}\nThese tools are authorized and dynamically assembled by the backend engine.",
        tools_line
    ));

    let has_fs_tools = resolved_tools.iter().any(|t| {
        matches!(t.as_str(), "read_file" | "write_file" | "edit_file" | "execute_command" | "glob" | "grep")
    });
    let has_subagents = resolved_tools.iter().any(|t| {
        matches!(t.as_str(), "subagent" | "subagent_bg" | "collect_subagent")
    });

    if has_fs_tools {
        let active_fs_tools: Vec<&str> = ["read_file", "write_file", "edit_file", "glob", "grep", "execute_command"]
            .into_iter()
            .filter(|t| resolved_tools.iter().any(|at| at == *t))
            .collect();
        let active_fs_str = active_fs_tools.iter().map(|t| format!("`{}`", t)).collect::<Vec<_>>().join(", ");
        let mut fs_mandate = format!(
            "# Scoped Tool Execution Mandate\nYou have direct tool access scoped to your isolated worktree. Use {} to inspect, modify, and verify code.",
            active_fs_str
        );
        if resolved_tools.iter().any(|t| t == "execute_command") {
            fs_mandate.push_str("\nAlways use Ground Truth Sensors (e.g. `cargo check`, `npm run build`) via `execute_command` to verify changes.");
        }
        tool_sections.push(fs_mandate);
    }

    if has_subagents {
        let mut sub_mandate = String::from("## Subagent Delegation Mandate\n");
        if resolved_tools.iter().any(|t| t == "subagent") {
            sub_mandate.push_str("- When delegating work to a subagent, prefer `subagent` (oneshot sync) which blocks and waits for the full report before proceeding.\n");
        }
        if resolved_tools.iter().any(|t| t == "subagent_bg") {
            sub_mandate.push_str("- If you use `subagent_bg` for background execution, you MUST call `collect_subagent` with the returned `subagent_id` to wait for and consume the subagent's report before taking subsequent actions or finishing the task. Never fire-and-forget background subagents without collecting them.\n");
        }
        if resolved_tools.iter().any(|t| t == "collect_subagent" || t == "subagent_bg") {
            sub_mandate.push_str("- Single-Action Collection: When invoking 'collect_subagent', you MUST invoke it as the ONLY tool call in that turn. Do NOT batch file reads or other tools alongside collect_subagent; wait to consume and analyze the received reports before planning next steps.\n");
        }
        tool_sections.push(sub_mandate.trim().to_string());
    }

    if !resolved_tools.is_empty() {
        tool_sections.push(
            "## Execution Protocol & Final Answer Discipline:\n- While you are still planning, researching, writing files, or executing tools, do NOT write a concluding summary or farewell message in your turn content. Keep turn content focused strictly on immediate intent or leave content empty when issuing tool calls.\n- ONLY output your comprehensive final answer when ALL tools have finished executing, all verifications have passed, and NO further tool calls are required."
                .to_string(),
        );
    }

    if !tool_sections.is_empty() {
        sections.push(tool_sections.join("\n\n"));
    }

    Ok(sections.join("\n\n---\n\n"))
}

pub fn get_standard_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition::function(
            "read_file",
            "Read UTF-8 text file contents with line numbers from the isolated worktree.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative file path within worktree (or 'path')." },
                    "offset": { "type": "integer", "description": "1-based starting line number (default: 1)." },
                    "limit": { "type": "integer", "description": "Maximum number of lines to return (default: 2000)." }
                },
                "required": ["file_path"]
            }),
        ),
        ToolDefinition::function(
            "write_file",
            "Write complete UTF-8 content to a file inside the isolated worktree.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative file path within worktree (or 'path')." },
                    "content": { "type": "string", "description": "Complete file text content to write." }
                },
                "required": ["file_path", "content"]
            }),
        ),
        ToolDefinition::function(
            "edit_file",
            "Targeted text replacement in an existing file inside the isolated worktree. Aliased as 'str_replace'.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative file path within worktree (or 'path')." },
                    "old_string": { "type": "string", "description": "Existing text substring to replace (or 'old_str')." },
                    "new_string": { "type": "string", "description": "Replacement text substring (or 'new_str')." },
                    "replace_all": { "type": "boolean", "description": "Replace all occurrences (default: false)." }
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
        ),
        ToolDefinition::function(
            "glob",
            "Find files matching a glob pattern inside the isolated worktree.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Glob pattern (e.g. '**/*.rs', 'src/**/*.ts')." },
                    "path": { "type": "string", "description": "Subdirectory to search in (default: '.')." }
                },
                "required": ["pattern"]
            }),
        ),
        ToolDefinition::function(
            "grep",
            "Search file contents with a regular expression inside the isolated worktree.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Regular expression pattern." },
                    "path": { "type": "string", "description": "Directory or file to search (default: '.')." }
                },
                "required": ["pattern"]
            }),
        ),
        ToolDefinition::function(
            "execute_command",
            "Execute a shell command inside the isolated worktree with piped stdout/stderr. Aliased as 'bash'.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command string to run (e.g. 'cargo check', or 'cmd')." },
                    "timeout_seconds": { "type": "integer", "description": "Execution timeout in seconds (default: 60)." }
                },
                "required": ["command"]
            }),
        ),
        ToolDefinition::function(
            "web_search",
            "Perform a web search using DuckDuckGo to discover external documentation, APIs, and error solutions.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query terms." },
                    "limit": { "type": "integer", "description": "Maximum number of results to return (default: 5)." }
                },
                "required": ["query"]
            }),
        ),
        ToolDefinition::function(
            "web_fetch",
            "Fetch a URL over HTTP/HTTPS and return cleaned Markdown content.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The target HTTP or HTTPS URL." }
                },
                "required": ["url"]
            }),
        ),
        ToolDefinition::function(
            "subagent",
            "Delegate a focused, independent sub-task to a specialized subagent synchronously (Oneshot). Blocks until completion and returns the final report directly. Subagents run in-process inside the current directory without moving cards or creating worktrees.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Short task description (e.g. 'Analyze project architecture', 'Find memory leak')." },
                    "role": { "type": "string", "description": "Subagent role persona (e.g. 'general', 'developer', 'reviewer', 'explorer'). Default: 'general'." },
                    "prompt": { "type": "string", "description": "Detailed task instructions." },
                    "tools": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional list of tools allowed for this subagent. If omitted, defaults to ALL tools available to the parent agent."
                    }
                },
                "required": ["title", "prompt"]
            }),
        ),
        ToolDefinition::function(
            "subagent_bg",
            "Launch a specialized subagent in the background (asynchronous). Returns immediately with a subagent_id and status 'running'. Use collect_subagent with subagent_id to retrieve the report when needed.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Short task description (e.g. 'Run long benchmark', 'Deep search repo')." },
                    "role": { "type": "string", "description": "Subagent role persona (e.g. 'general', 'developer', 'reviewer', 'explorer'). Default: 'general'." },
                    "prompt": { "type": "string", "description": "Detailed task instructions." },
                    "tools": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional list of tools allowed for this subagent. If omitted, defaults to ALL tools available to the parent agent."
                    }
                },
                "required": ["title", "prompt"]
            }),
        ),
        ToolDefinition::function(
            "collect_subagent",
            "Await the completion of one or multiple background subagents (launched via subagent_bg) and retrieve their final reports without polling. Accepts either 'subagent_id' (string) or 'subagent_ids' (array of strings).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "subagent_id": { "type": "string", "description": "Single subagent_id to await." },
                    "subagent_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of subagent_ids to await."
                    }
                }
            }),
        ),
        ToolDefinition::function(
            "subagent_list",
            "List all background and synchronous subagents for the current task with their status (running, completed, aborted), role, title, steps, and subagent_id for collection.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "status_filter": {
                        "type": "string",
                        "enum": ["all", "running", "completed", "aborted"],
                        "description": "Optional status filter ('all', 'running', 'completed', or 'aborted'). Defaults to 'all'."
                    }
                }
            }),
        ),
        ToolDefinition::function(
            "todo_write",
            "Record and update task plan list to show progress (max 20 items). Add todos with status pending | in_progress | completed.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "maxItems": 20,
                        "items": {
                            "type": "object",
                            "properties": {
                                "content": { "type": "string" },
                                "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
                            },
                            "required": ["content", "status"]
                        }
                    }
                },
                "required": ["todos"]
            }),
        ),
        ToolDefinition::function(
            "advance_task",
            "Advance task to the next sequential pipeline stage or transition to a specific stage (stage_id / target_stage). CRITICAL: All TODO items must be fully completed before calling. Requires comprehensive summary of completed work and verification proof.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "summary": { "type": "string", "description": "Comprehensive summary of modifications, sensor results, and verification proof." },
                    "stage_id": { "type": "string", "description": "Target pipeline stage ID (e.g. 'review', 'retro', 'ready'). Defaults to sequential next stage." }
                }
            }),
        ),
        ToolDefinition::function(
            "reject_task",
            "Reject task and return it to the previous pipeline stage with feedback. Requires specific reasons and actionable change requests.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "reason": { "type": "string", "description": "Rejection rationale: failing sensors, bugs, or required fixes." }
                },
                "required": ["reason"]
            }),
        ),
        ToolDefinition::function(
            "question",
            "Ask an interactive clarifying question to the human user and suspend agent execution until answered (Human-in-the-Loop). Alias: 'ask_user'.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "The specific question to ask the user." },
                    "options": {
                        "type": "array",
                        "description": "Predefined choices for the user. If recommended, append '(Recommended)' to label.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "label": { "type": "string", "description": "Short option label" },
                                "description": { "type": "string", "description": "Clarifying subtitle or tradeoff" }
                            },
                            "required": ["label"]
                        }
                    },
                    "multiple": { "type": "boolean", "description": "Whether multiple options can be chosen" }
                },
                "required": ["text"]
            }),
        ),
        ToolDefinition::function(
            "create_task",
            "Decompose requirements and create a new task card on the Kanban board.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Concise imperative task title." },
                    "description": { "type": "string", "description": "Detailed technical requirements, acceptance criteria, and file targets." },
                    "column_id": { "type": "string", "description": "Target column ID (defaults to 'backlog')." },
                    "board_id": { "type": "string", "description": "Target board ID (defaults to current task's board)." },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Task tags."
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["low", "medium", "high"],
                        "description": "Task priority: low, medium, high."
                    },
                    "is_subtask": {
                        "type": "boolean",
                        "description": "Whether to create this as a subtask of the current task. Default is false (standalone root task)."
                    },
                    "parent_id": {
                        "type": "string",
                        "description": "Explicit parent task ID if creating a subtask."
                    }
                },
                "required": ["title", "description"]
            }),
        ),
        ToolDefinition::function(
            "create_tasks",
            "Batch decompose requirements and create multiple task cards on the Kanban board.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "tasks": {
                        "type": "array",
                        "description": "Array of task objects to create.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string", "description": "Concise imperative task title." },
                                "description": { "type": "string", "description": "Detailed technical requirements, acceptance criteria, and file targets." },
                                "column_id": { "type": "string", "description": "Target column ID (defaults to 'backlog')." },
                                "board_id": { "type": "string", "description": "Target board ID (defaults to current task's board)." },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Task tags."
                                },
                                "priority": {
                                    "type": "string",
                                    "enum": ["low", "medium", "high"],
                                    "description": "Task priority: low, medium, high."
                                },
                                "is_subtask": {
                                    "type": "boolean",
                                    "description": "Whether to create this as a subtask of the current task. Default is false (standalone root task)."
                                },
                                "parent_id": {
                                    "type": "string",
                                    "description": "Explicit parent task ID if creating a subtask."
                                }
                            },
                            "required": ["title", "description"]
                        }
                    }
                },
                "required": ["tasks"]
            }),
        ),
        ToolDefinition::function(
            "complete_task",
            "Complete task and transition it to the terminal column (ready/done). Records APPROVED verdict and final handover summary.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "ID of task to complete (e.g. 'CVH-001'). If omitted, completes the current task." },
                    "summary": { "type": "string", "description": "Comprehensive summary of completed work, changes, and verification proof." }
                },
                "required": ["summary"]
            }),
        ),
        ToolDefinition::function(
            "list_tasks",
            "Retrieve a compact registry of recent Kanban board tasks for analysis, retrospectives, or quality audits.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of tasks to return (default: 20, max: 50)."
                    },
                    "stage_id": {
                        "type": "string",
                        "description": "Optional filter by stage or column ID (e.g. 'ready', 'dev')."
                    },
                    "query": {
                        "type": "string",
                        "description": "Optional text search query across title, id, or handover summary."
                    }
                }
            }),
        ),
        ToolDefinition::function(
            "get_task_history",
            "Retrieve step-by-step execution chronology of a task (original requirements, tool invocations, exit codes, verdict, and final response) for quality audit.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task ID (e.g. 'CVH-042' or 'CVH-042.1')."
                    },
                    "include_tool_outputs": {
                        "type": "boolean",
                        "description": "If true, includes trimmed tool outputs (default: false)."
                    }
                },
                "required": ["task_id"]
            }),
        ),
        get_load_rule_tool(Path::new(""), None),
    ]
}

pub fn get_available_rules(repo_root: &Path, role: Option<&str>) -> Vec<String> {
    let mut rules = Vec::new();
    let mut dirs_to_scan = Vec::new();

    if let Some(r) = role.filter(|s| !s.trim().is_empty()) {
        dirs_to_scan.push(repo_root.join(".cvh").join("agents").join(r).join("rules"));
    }
    dirs_to_scan.push(repo_root.join(".cvh").join("rules"));

    for d in &dirs_to_scan {
        if d.is_dir() {
            if let Ok(entries) = std::fs::read_dir(d) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            rules.push(stem.to_string());
                        }
                    }
                }
            }
        }
    }

    if rules.is_empty() {
        let agents_base = repo_root.join(".cvh").join("agents");
        if agents_base.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&agents_base) {
                for entry in entries.flatten() {
                    let r_dir = entry.path().join("rules");
                    if r_dir.is_dir() {
                        if let Ok(r_entries) = std::fs::read_dir(&r_dir) {
                            for re in r_entries.flatten() {
                                let path = re.path();
                                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                        rules.push(stem.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    rules.sort();
    rules.dedup();
    rules
}

pub fn get_load_rule_tool(repo_root: &Path, role: Option<&str>) -> ToolDefinition {
    let available_rules = get_available_rules(repo_root, role);
    let rule_prop = if !available_rules.is_empty() {
        serde_json::json!({
            "type": "string",
            "description": "Modular rule topic to load dynamically.",
            "enum": available_rules
        })
    } else {
        serde_json::json!({
            "type": "string",
            "description": "Modular rule topic to load dynamically."
        })
    };

    ToolDefinition::function(
        "load_rule",
        "Dynamically load a modular project standard, architectural invariant, or tech guideline by topic.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "rule": rule_prop
            },
            "required": ["rule"]
        }),
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomToolConfig {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub command: String,
}

pub async fn load_custom_tools(repo_root: &Path) -> Vec<CustomToolConfig> {
    let tools_dir = repo_root.join(".cvh").join("tools");
    let mut custom_tools = Vec::new();
    if tools_dir.exists() {
        if let Ok(mut entries) = tokio::fs::read_dir(&tools_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        if let Ok(cfg) = serde_json::from_str::<CustomToolConfig>(&content) {
                            custom_tools.push(cfg);
                        }
                    }
                }
            }
        }
    }
    custom_tools
}

pub async fn get_all_tools(repo_root: &Path) -> Vec<ToolDefinition> {
    get_all_tools_for_role(repo_root, None).await
}

pub async fn get_all_tools_for_role(repo_root: &Path, role: Option<&str>) -> Vec<ToolDefinition> {
    let mut tools = get_standard_tools();
    if let Some(pos) = tools.iter().position(|t| t.function.name == "load_rule") {
        tools[pos] = get_load_rule_tool(repo_root, role);
    }
    let mut custom_tools = load_custom_tools(repo_root).await;
    // Deterministic order: filesystem read_dir order is OS-dependent.
    // Sort custom tools by name so merged list is stable (final active list
    // is additionally sorted alphabetically in run_agent_loop for prefix caching).
    custom_tools.sort_by(|a, b| a.name.cmp(&b.name));
    for ct in custom_tools {
        tools.push(ToolDefinition::function(
            &ct.name,
            &ct.description,
            ct.parameters,
        ));
    }
    if let Ok(tool_policy) = crate::config::load_tool_policies(repo_root).await {
        for tool in &mut tools {
            if let Some(custom_desc) = tool_policy.tool_descriptions.get(&tool.function.name) {
                tool.function.description = custom_desc.clone();
            }
            let custom_schema = tool_policy
                .tool_custom_schemas
                .get(&tool.function.name)
                .or_else(|| {
                    if tool.function.name == "subagent" {
                        tool_policy.tool_custom_schemas.get("subagent_sync")
                    } else {
                        None
                    }
                });
            if let Some(custom_val) = custom_schema {
                if let Ok(custom_def) = serde_json::from_value::<ToolDefinition>(custom_val.clone()) {
                    *tool = custom_def;
                }
            }
        }
    }
    tools
}

/// Prunes the .cvh/spill directory if file count exceeds 50 or total size exceeds 20MB,
/// removing the oldest files first based on modification time.
pub async fn prune_spill_dir(spill_dir: &Path) -> Result<()> {
    let mut entries = match tokio::fs::read_dir(spill_dir).await {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };

    struct SpillFileInfo {
        path: PathBuf,
        mtime: std::time::SystemTime,
        size: u64,
    }

    let mut files: Vec<SpillFileInfo> = Vec::new();
    let mut total_bytes = 0u64;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            if let Ok(meta) = entry.metadata().await {
                let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let size = meta.len();
                total_bytes += size;
                files.push(SpillFileInfo { path, mtime, size });
            }
        }
    }

    const MAX_FILES: usize = 50;
    const MAX_BYTES: u64 = 20 * 1024 * 1024; // 20 MB

    if files.len() > MAX_FILES || total_bytes > MAX_BYTES {
        files.sort_by_key(|f| f.mtime);
        let mut file_count = files.len();
        for file in files {
            if file_count <= MAX_FILES && total_bytes <= MAX_BYTES {
                break;
            }
            if tokio::fs::remove_file(&file.path).await.is_ok() {
                total_bytes = total_bytes.saturating_sub(file.size);
                file_count = file_count.saturating_sub(1);
            }
        }
    }

    Ok(())
}

/// Tool Output Spilling: if stdout/output exceeds thresholds, save full log to `.cvh/spill/<timestamp>-<tool>.log`,
/// return head and tail + reference to spilled file.
/// Subagent and task history tools have a generous limit (5000 lines / 256KB), but are still safely spilled
/// if they exceed that absolute ceiling to prevent context explosion.
pub async fn maybe_spill_output(
    repo_root: &Path,
    tool_name: &str,
    output: String,
) -> Result<String> {
    let canonical = canonical_tool_name(tool_name);
    let is_exempt = canonical == "subagent"
        || canonical == "subagent_bg"
        || canonical == "collect_subagent"
        || canonical == "subagent_list"
        || canonical == "load_rule"
        || canonical == "get_task_history";

    let lines_count = output.lines().count();
    let bytes_len = output.len();

    let (max_lines, max_bytes) = if is_exempt {
        (5000, 256 * 1024)
    } else if canonical == "read_file" {
        (2000, 128 * 1024)
    } else {
        (500, 64 * 1024)
    };

    if lines_count > max_lines || bytes_len > max_bytes {
        let spill_dir = repo_root.join(".cvh").join("spill");
        tokio::fs::create_dir_all(&spill_dir).await?;
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f");
        let safe_name = tool_name.replace(['/', '\\', ':', '*'], "_");
        let filename = format!("{}-{}.log", timestamp, safe_name);
        let spill_file = spill_dir.join(&filename);

        tokio::fs::write(&spill_file, &output).await?;
        let _ = prune_spill_dir(&spill_dir).await;

        let all_lines: Vec<&str> = output.lines().collect();
        let head_count = if is_exempt { 50 } else { 20 };
        let tail_count = if is_exempt { 50 } else { 20 };
        let head = all_lines[..head_count.min(all_lines.len())].join("\n");
        let tail = if all_lines.len() > head_count + tail_count {
            all_lines[all_lines.len() - tail_count..].join("\n")
        } else {
            String::new()
        };

        let rel_spill = format!(".cvh/spill/{}", filename);
        let spilled_msg = format!(
            "[Output truncated: {} lines ({} bytes) spilled to {}]\n--- HEAD (first {} lines) ---\n{}\n\n--- TAIL (last {} lines) ---\n{}\n[Full log available at {}]",
            lines_count, bytes_len, rel_spill, head_count, head, tail_count, tail, rel_spill
        );
        Ok(spilled_msg)
    } else {
        Ok(output)
    }
}

/// Tokenizes a line into identifier/number words, symbols, and punctuation tokens.
pub fn tokenize_line(s: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = None;
    let mut in_word = false;

    for (idx, ch) in s.char_indices() {
        if ch.is_whitespace() {
            if let Some(st) = start.take() {
                tokens.push(&s[st..idx]);
            }
            in_word = false;
        } else if ch.is_alphanumeric() || ch == '_' {
            if !in_word {
                if let Some(st) = start.take() {
                    tokens.push(&s[st..idx]);
                }
                start = Some(idx);
                in_word = true;
            }
        } else {
            if let Some(st) = start.take() {
                tokens.push(&s[st..idx]);
            }
            let char_len = ch.len_utf8();
            tokens.push(&s[idx..idx + char_len]);
            in_word = false;
        }
    }
    if let Some(st) = start {
        tokens.push(&s[st..]);
    }
    tokens
}

/// Computes token-level Dice/F1 similarity between two line strings (range 0.0 .. 1.0).
pub fn line_similarity(a: &str, b: &str) -> f32 {
    let a_trim = a.trim();
    let b_trim = b.trim();
    if a_trim == b_trim {
        return 1.0;
    }
    if a_trim.is_empty() || b_trim.is_empty() {
        return 0.0;
    }
    let a_tokens = tokenize_line(a_trim);
    let b_tokens = tokenize_line(b_trim);
    if a_tokens.is_empty() || b_tokens.is_empty() {
        return 0.0;
    }
    let n = a_tokens.len();
    let m = b_tokens.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            if a_tokens[i] == b_tokens[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }
    let lcs = dp[n][m];
    (2.0 * lcs as f32) / ((n + m) as f32)
}

/// Generates a human-readable explanation of token differences between expected and actual lines.
pub fn diff_line_tokens(expected: &str, actual: &str) -> String {
    let exp_trim = expected.trim();
    let act_trim = actual.trim();
    if exp_trim == act_trim {
        return "whitespace or indentation mismatch".to_string();
    }
    let exp_tokens = tokenize_line(exp_trim);
    let act_tokens = tokenize_line(act_trim);
    if exp_tokens.is_empty() && act_tokens.is_empty() {
        return "whitespace mismatch".to_string();
    }
    if exp_tokens.is_empty() {
        return format!("expected empty line, found '{}'", act_trim);
    }
    if act_tokens.is_empty() {
        return format!("expected '{}', found empty line in file", exp_trim);
    }

    let n = exp_tokens.len();
    let m = act_tokens.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            if exp_tokens[i] == act_tokens[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut i = n;
    let mut j = m;
    let mut raw_diffs = Vec::new();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && exp_tokens[i - 1] == act_tokens[j - 1] {
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            raw_diffs.push((None, Some(act_tokens[j - 1])));
            j -= 1;
        } else if i > 0 && (j == 0 || dp[i][j - 1] < dp[i - 1][j]) {
            raw_diffs.push((Some(exp_tokens[i - 1]), None));
            i -= 1;
        }
    }
    raw_diffs.reverse();

    let mut diffs = Vec::new();
    let mut k = 0;
    while k < raw_diffs.len() {
        if k + 1 < raw_diffs.len() {
            if let ((Some(exp), None), (None, Some(act))) = (raw_diffs[k], raw_diffs[k + 1]) {
                diffs.push(format!("expected '{}', found '{}'", exp, act));
                k += 2;
                continue;
            }
        }
        if let (Some(e), None) = raw_diffs[k] {
            diffs.push(format!("missing '{}'", e));
            k += 1;
        } else if let (None, Some(a)) = raw_diffs[k] {
            diffs.push(format!("unexpected '{}'", a));
            k += 1;
        } else {
            k += 1;
        }
    }

    if diffs.is_empty() {
        "whitespace or formatting mismatch".to_string()
    } else if diffs.len() > 6 {
        let truncated = diffs[..6].join("; ");
        format!("{}; ... ({} more token diffs)", truncated, diffs.len() - 6)
    } else {
        diffs.join("; ")
    }
}

/// Calculates informative error message when `old_string` is not found in file content.
/// Computes multi-line block similarity and outputs a clear line-by-line diff with token-level diagnostics.
pub fn build_edit_mismatch_error(content: &str, file_path: &str, old_string: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if lines.is_empty() {
        return format!(
            "Edit failed: File '{}' is empty (0 lines). Cannot locate 'old_string'.",
            file_path
        );
    }

    let search_lines: Vec<&str> = old_string.lines().collect();
    if search_lines.is_empty() {
        return format!(
            "Edit failed: 'old_string' is empty. Cannot locate target in '{}'.",
            file_path
        );
    }
    let search_block_len = search_lines.len();

    // Slide window of size search_block_len across file lines to find best matching block
    let mut best_score = 0.0f32;
    let mut best_idx = 0;

    for idx in 0..total_lines {
        let cand_len = search_block_len.min(total_lines - idx);
        let mut score_sum = 0.0f32;
        for k in 0..search_block_len {
            if k < cand_len {
                score_sum += line_similarity(search_lines[k], lines[idx + k]);
            }
        }
        let block_score = score_sum / (search_block_len as f32);
        if block_score > best_score {
            best_score = block_score;
            best_idx = idx;
        }
    }

    // Safety guard: if subsequent lines or tokens differ, never claim 100% similarity!
    let mut all_lines_identical = true;
    let cand_len = search_block_len.min(total_lines.saturating_sub(best_idx));
    if cand_len < search_block_len {
        all_lines_identical = false;
    } else {
        for k in 0..search_block_len {
            if search_lines[k] != lines[best_idx + k] {
                all_lines_identical = false;
                break;
            }
        }
    }
    if !all_lines_identical && best_score >= 0.999 {
        best_score = 0.99;
    }

    let mut msg = format!(
        "Target 'old_string' not found in '{}' (total {} lines).\n",
        file_path, total_lines
    );

    if best_score >= 0.25 {
        let start_line = best_idx + 1;
        let end_line = (best_idx + cand_len).max(start_line);
        let pct = (best_score * 100.0).clamp(0.0, if all_lines_identical { 100.0 } else { 99.0 });

        msg.push_str(&format!(
            "Closest matching candidate block identified at lines {}-{} (block similarity: {:.0}%):\n\n",
            start_line, end_line, pct
        ));

        msg.push_str("--- Line-by-Line Diff ([-] old_string vs [+] file candidate) ---\n");
        for k in 0..search_block_len {
            let exp_line = search_lines[k];
            if k < cand_len {
                let act_line = lines[best_idx + k];
                let file_line_no = best_idx + k + 1;
                if exp_line == act_line {
                    msg.push_str(&format!(
                        "  Line {} (file line {}): EXACT MATCH\n    \"{}\"\n",
                        k + 1, file_line_no, act_line
                    ));
                } else if exp_line.trim() == act_line.trim() {
                    msg.push_str(&format!(
                        "  Line {} (file line {}): WHITESPACE MISMATCH\n    - old_string: \"{}\"\n    + file:       \"{}\"\n    Diff: whitespace or indentation mismatch\n",
                        k + 1, file_line_no, exp_line, act_line
                    ));
                } else {
                    let diff_diag = diff_line_tokens(exp_line, act_line);
                    msg.push_str(&format!(
                        "  Line {} (file line {}): MISMATCH\n    - old_string: \"{}\"\n    + file:       \"{}\"\n    Diff: {}\n",
                        k + 1, file_line_no, exp_line, act_line, diff_diag
                    ));
                }
            } else {
                msg.push_str(&format!(
                    "  Line {}: MISMATCH (file ended before this line)\n    - old_string: \"{}\"\n",
                    k + 1, exp_line
                ));
            }
        }

        msg.push_str("\n--- Candidate Block in File (use exact lines below for edit_file) ---\n");
        let ctx_start = best_idx.saturating_sub(2);
        let ctx_end = (best_idx + cand_len + 2).min(total_lines);
        for (offset, line_content) in lines[ctx_start..ctx_end].iter().enumerate() {
            let line_num = ctx_start + offset + 1;
            let line_no_0 = ctx_start + offset;
            let is_cand = line_no_0 >= best_idx && line_no_0 < best_idx + cand_len;
            let marker = if is_cand { ">>" } else { "  " };
            msg.push_str(&format!("{} {:4} | {}\n", marker, line_num, line_content));
        }

        let read_offset = start_line;
        let read_limit = (cand_len + 4).max(5);
        msg.push_str(&format!(
            "\nRecommendation: Use the exact candidate lines above or inspect with read_file(file_path: \"{}\", offset: {}, limit: {}) to update old_string.",
            file_path, read_offset, read_limit
        ));
    } else {
        msg.push_str("No close match found (similarity < 25%). First 10 lines of file for context:\n");
        let end = 10.min(total_lines);
        for (i, line) in lines[..end].iter().enumerate() {
            msg.push_str(&format!("   {:4} | {}\n", i + 1, line));
        }
        msg.push_str("\nRecommendation: Use read_file to inspect exact lines and retry edit_file with precise context.");
    }

    msg
}

/// Represents a line span with its text and byte boundaries in a file string.
#[derive(Debug, Clone)]
struct LineBoundary<'a> {
    text: &'a str,
    start: usize,
    end_content: usize,
    end_full: usize,
}

fn parse_line_boundaries<'a>(content: &'a str) -> Vec<LineBoundary<'a>> {
    let mut lines = Vec::new();
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut line_start = 0;

    while line_start < len {
        let mut i = line_start;
        while i < len && bytes[i] != b'\n' && bytes[i] != b'\r' {
            i += 1;
        }
        let end_content = i;
        if i < len && bytes[i] == b'\r' {
            i += 1;
        }
        if i < len && bytes[i] == b'\n' {
            i += 1;
        }
        let end_full = i;
        lines.push(LineBoundary {
            text: &content[line_start..end_content],
            start: line_start,
            end_content,
            end_full,
        });
        line_start = end_full;
    }

    lines
}

/// Performs whitespace-relaxed / normalized line matching and replacement.
/// Trims leading and trailing whitespace on each line to locate the target block,
/// while respecting CRLF/LF line ending conventions.
fn replace_whitespace_relaxed(
    current: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
    file_has_crlf: bool,
) -> Result<String, String> {
    let file_lines = parse_line_boundaries(current);
    let search_lines: Vec<&str> = old_string.lines().collect();
    if search_lines.is_empty() || file_lines.len() < search_lines.len() {
        return Err("NOT_FOUND".to_string());
    }

    let m = search_lines.len();
    let search_trimmed: Vec<&str> = search_lines.iter().map(|l| l.trim()).collect();

    // Do not match if search lines are entirely whitespace
    if search_trimmed.iter().all(|l| l.is_empty()) {
        return Err("NOT_FOUND".to_string());
    }

    let mut matches = Vec::new();
    let mut idx = 0;
    while idx <= file_lines.len().saturating_sub(m) {
        let mut matched = true;
        for k in 0..m {
            if file_lines[idx + k].text.trim() != search_trimmed[k] {
                matched = false;
                break;
            }
        }
        if matched {
            matches.push(idx);
            if replace_all {
                idx += m;
                continue;
            }
        }
        idx += 1;
    }

    if matches.is_empty() {
        return Err("NOT_FOUND".to_string());
    }

    if !replace_all && matches.len() > 1 {
        return Err(format!(
            "Ambiguous match: 'old_string' occurs {} times in file (whitespace-relaxed). Set replace_all: true or supply more surrounding lines.",
            matches.len()
        ));
    }

    let old_ends_with_newline = old_string.ends_with('\n') || old_string.ends_with("\r\n");

    // Adapt new_string newline convention to match the file
    let base_new = if file_has_crlf {
        new_string.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        new_string.replace("\r\n", "\n")
    };

    let new_ends_with_newline = base_new.ends_with('\n') || base_new.ends_with("\r\n");
    let replace_includes_newline = old_ends_with_newline || new_ends_with_newline;

    // Collect replacement ranges and replacement strings
    // We sort matches in reverse order so applying edits from bottom-to-top preserves byte offsets.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();

    for &idx in &matches {
        let span_start = file_lines[idx].start;
        let last_line = &file_lines[idx + m - 1];
        let span_end = if replace_includes_newline {
            last_line.end_full
        } else {
            last_line.end_content
        };

        // Determine indentation adaptation:
        let file_indent: String = file_lines[idx]
            .text
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        let old_indent: String = search_lines[0]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        let adapted_new = if file_indent != old_indent && file_indent.starts_with(&old_indent) {
            let extra_indent = &file_indent[old_indent.len()..];
            let nl = if file_has_crlf { "\r\n" } else { "\n" };
            let new_lines: Vec<&str> = base_new.lines().collect();
            let mut rebuilt = Vec::new();
            for l in &new_lines {
                if l.trim().is_empty() {
                    rebuilt.push(l.to_string());
                } else if !l.starts_with(&file_indent) {
                    rebuilt.push(format!("{}{}", extra_indent, l));
                } else {
                    rebuilt.push(l.to_string());
                }
            }
            let mut joined = rebuilt.join(nl);
            if new_ends_with_newline {
                joined.push_str(nl);
            }
            joined
        } else if file_indent != old_indent && old_indent.starts_with(&file_indent) {
            let strip_len = old_indent.len() - file_indent.len();
            let nl = if file_has_crlf { "\r\n" } else { "\n" };
            let new_lines: Vec<&str> = base_new.lines().collect();
            let all_can_strip = new_lines
                .iter()
                .all(|l| l.trim().is_empty() || l.starts_with(&old_indent));
            if all_can_strip {
                let mut rebuilt = Vec::new();
                for l in &new_lines {
                    if l.trim().is_empty() {
                        rebuilt.push(l.to_string());
                    } else {
                        rebuilt.push(l[strip_len..].to_string());
                    }
                }
                let mut joined = rebuilt.join(nl);
                if new_ends_with_newline {
                    joined.push_str(nl);
                }
                joined
            } else {
                base_new.clone()
            }
        } else {
            base_new.clone()
        };

        edits.push((span_start, span_end, adapted_new));
    }

    // Sort edits by span_start descending so replacing tail doesn't invalidate earlier byte offsets
    edits.sort_by_key(|a| std::cmp::Reverse(a.0));

    let mut result = current.to_string();
    for (start, end, rep) in edits {
        if start <= result.len() && end <= result.len() && start <= end {
            result.replace_range(start..end, &rep);
        }
    }

    Ok(result)
}

/// Performs string replacement with CRLF vs LF newline normalization and resilient line matching.
/// Handles cases where a file on Windows has `\r\n` but LLM sends `\n` (or vice versa),
/// whitespace/indentation discrepancies, while preserving the file's original line endings.
pub fn replace_text_normalized(
    current: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<String, String> {
    // 1. Direct match first (fast path & exact match)
    if current.contains(old_string) {
        if !replace_all {
            let count = current.matches(old_string).count();
            if count > 1 {
                return Err(format!(
                    "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more surrounding lines.",
                    count
                ));
            }
            return Ok(current.replacen(old_string, new_string, 1));
        } else {
            return Ok(current.replace(old_string, new_string));
        }
    }

    // 2. Windows CRLF adaptation: file has \r\n, old_string has LF \n
    let file_has_crlf = current.contains("\r\n");
    if file_has_crlf && !old_string.contains("\r\n") && old_string.contains('\n') {
        let crlf_old = old_string.replace('\n', "\r\n");
        let crlf_new = new_string.replace("\r\n", "\n").replace('\n', "\r\n");
        if current.contains(&crlf_old) {
            if !replace_all {
                let count = current.matches(&crlf_old).count();
                if count > 1 {
                    return Err(format!(
                        "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more surrounding lines.",
                        count
                    ));
                }
                return Ok(current.replacen(&crlf_old, &crlf_new, 1));
            } else {
                return Ok(current.replace(&crlf_old, &crlf_new));
            }
        }
    }

    // 3. Reverse adaptation: file has LF \n, old_string has CRLF \r\n
    if !file_has_crlf && old_string.contains("\r\n") {
        let lf_old = old_string.replace("\r\n", "\n");
        let lf_new = new_string.replace("\r\n", "\n");
        if current.contains(&lf_old) {
            if !replace_all {
                let count = current.matches(&lf_old).count();
                if count > 1 {
                    return Err(format!(
                        "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more surrounding lines.",
                        count
                    ));
                }
                return Ok(current.replacen(&lf_old, &lf_new, 1));
            } else {
                return Ok(current.replace(&lf_old, &lf_new));
            }
        }
    }

    // 4. Mixed line endings: normalize both to LF \n, perform replacement, then restore original convention
    let norm_current = current.replace("\r\n", "\n");
    let norm_old = old_string.replace("\r\n", "\n");
    let norm_new = new_string.replace("\r\n", "\n");

    if norm_current.contains(&norm_old) {
        let count = norm_current.matches(&norm_old).count();
        if !replace_all && count > 1 {
            return Err(format!(
                "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more surrounding lines.",
                count
            ));
        }

        let norm_updated = if replace_all {
            norm_current.replace(&norm_old, &norm_new)
        } else {
            norm_current.replacen(&norm_old, &norm_new, 1)
        };

        let final_updated = if file_has_crlf {
            norm_updated.replace('\n', "\r\n")
        } else {
            norm_updated
        };

        return Ok(final_updated);
    }

    // 5. Whitespace-relaxed line matching:
    // If exact byte match and newline normalization fail, attempt normalized line matching
    // (trimming leading/trailing whitespace on each line).
    replace_whitespace_relaxed(current, old_string, new_string, replace_all, file_has_crlf)
}

/// Execution context for tool calls, bundling path roots, policy, and abort handle.
#[derive(Clone)]
pub struct ToolContext<'a> {
    pub repo_root: &'a Path,
    pub worktree_dir: &'a Path,
    pub stage_id: Option<&'a str>,
    pub role: Option<&'a str>,
    pub abort_handle: Arc<AtomicBool>,
    pub tool_policy: &'a ToolPolicyConfig,
    pub task_id: Option<&'a str>,
    pub agent_allowed_tools: Option<&'a [String]>,
    pub app_handle: Option<AppHandle>,
    pub base_url: Option<&'a str>,
    pub api_key: Option<&'a str>,
    pub model: Option<&'a str>,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<&'a str>,
    pub call_id: Option<&'a str>,
}

#[derive(Clone)]
pub struct OwnedToolContext {
    pub repo_root: PathBuf,
    pub worktree_dir: PathBuf,
    pub stage_id: Option<String>,
    pub role: Option<String>,
    pub abort_handle: Arc<AtomicBool>,
    pub tool_policy: ToolPolicyConfig,
    pub task_id: Option<String>,
    pub agent_allowed_tools: Option<Vec<String>>,
    pub app_handle: Option<AppHandle>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<String>,
    pub call_id: Option<String>,
}

impl<'a> ToolContext<'a> {
    pub fn to_owned(&self) -> OwnedToolContext {
        OwnedToolContext {
            repo_root: self.repo_root.to_path_buf(),
            worktree_dir: self.worktree_dir.to_path_buf(),
            stage_id: self.stage_id.map(|s| s.to_string()),
            role: self.role.map(|s| s.to_string()),
            abort_handle: Arc::clone(&self.abort_handle),
            tool_policy: self.tool_policy.clone(),
            task_id: self.task_id.map(|s| s.to_string()),
            agent_allowed_tools: self.agent_allowed_tools.map(|s| s.to_vec()),
            app_handle: self.app_handle.clone(),
            base_url: self.base_url.map(|s| s.to_string()),
            api_key: self.api_key.map(|s| s.to_string()),
            model: self.model.map(|s| s.to_string()),
            temperature: self.temperature,
            reasoning_effort: self.reasoning_effort.map(|s| s.to_string()),
            call_id: self.call_id.map(|s| s.to_string()),
        }
    }
}

pub type SubagentReceiver = oneshot::Receiver<String>;

#[derive(Clone)]
pub struct SubagentRetryInfo {
    pub parent_task_id: String,
    pub subagent_id: String,
    pub tool_call_id: Option<String>,
    pub title: String,
    pub role: String,
    pub prompt: String,
    pub requested_tools: Option<Vec<String>>,
    pub ctx: OwnedToolContext,
    pub status: String,
    pub last_error: Option<String>,
    pub report: Option<String>,
    pub is_bg: bool,
    pub duration_ms: Option<f64>,
    pub steps: usize,
}

pub type SubagentSessionInfo = SubagentRetryInfo;

#[derive(Clone)]
pub struct SubagentSession {
    pub sender: Arc<tokio::sync::Mutex<Option<oneshot::Sender<String>>>>,
    pub retry_info: Arc<tokio::sync::Mutex<SubagentRetryInfo>>,
    pub abort_handle: Arc<AtomicBool>,
    pub join_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl SubagentSession {
    pub fn new(sender: Option<oneshot::Sender<String>>, retry_info: SubagentRetryInfo) -> Self {
        Self {
            sender: Arc::new(tokio::sync::Mutex::new(sender)),
            retry_info: Arc::new(tokio::sync::Mutex::new(retry_info)),
            abort_handle: Arc::new(AtomicBool::new(false)),
            join_handle: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}

pub static SUBAGENT_RECEIVERS: LazyLock<tokio::sync::Mutex<HashMap<String, SubagentReceiver>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));
pub static SUBAGENT_SESSIONS: LazyLock<tokio::sync::Mutex<HashMap<String, SubagentSession>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

/// Evicts completed, aborted, or failed subagent sessions from in-memory cache when capacity exceeds 30.
pub async fn prune_subagent_sessions_cache() {
    let mut sessions = SUBAGENT_SESSIONS.lock().await;
    if sessions.len() <= 30 {
        return;
    }
    let mut removable_keys = Vec::new();
    for (key, sess) in sessions.iter() {
        if let Ok(info) = sess.retry_info.try_lock() {
            if info.status == "finished" || info.status == "aborted" || info.status == "failed" {
                removable_keys.push(key.clone());
            }
        }
    }
    for k in removable_keys {
        if sessions.len() <= 30 {
            break;
        }
        sessions.remove(&k);
    }
}

/// Robust subagent session finder: searches SUBAGENT_SESSIONS by exact ID, stripped `:sub:`,
/// formatted `{parent}.sub-{id}`, or suffix matching `.sub-{tool_call_id}`.
pub async fn find_subagent_session(
    parent_task_id: Option<&str>,
    subagent_id: &str,
) -> Option<(String, SubagentSession)> {
    let sessions = SUBAGENT_SESSIONS.lock().await;
    let s_trim = subagent_id.trim();
    let p_trim = parent_task_id.map(|p| p.trim()).unwrap_or("");

    // 1. Direct match
    if let Some(session) = sessions.get(s_trim).cloned() {
        return Some((s_trim.to_string(), session));
    }

    // 2. Strip :sub: prefix if present
    if s_trim.contains(":sub:") {
        if let Some(after_sub) = s_trim.rsplit(":sub:").next() {
            if let Some(session) = sessions.get(after_sub).cloned() {
                return Some((after_sub.to_string(), session));
            }
        }
    }

    // 3. Try format!("{}.sub-{}", parent_task_id, subagent_id)
    if !p_trim.is_empty() {
        let key = format!("{}.sub-{}", p_trim, s_trim);
        if let Some(session) = sessions.get(&key).cloned() {
            return Some((key, session));
        }
    }

    // 4. Try matching tool_call_id suffix
    let tool_call_id = if let Some(pos) = s_trim.rfind(".sub-") {
        &s_trim[(pos + 5)..]
    } else {
        s_trim
    };

    if !tool_call_id.is_empty() {
        let suffix = format!(".sub-{}", tool_call_id);
        if !p_trim.is_empty() {
            for (k, v) in sessions.iter() {
                if k.starts_with(p_trim) && k.ends_with(&suffix) {
                    return Some((k.clone(), v.clone()));
                }
            }
        }
        for (k, v) in sessions.iter() {
            if k.ends_with(&suffix) {
                return Some((k.clone(), v.clone()));
            }
        }
    }

    None
}

/// Appends raw model request and response telemetry to `.cvh/llm_debug.log`
/// Rotates the log to `.cvh/llm_debug.log.1` if size exceeds 10MB.
pub async fn append_llm_debug_log(repo_root: &Path, text: &str) {
    let cvh_dir = repo_root.join(".cvh");
    let _ = tokio::fs::create_dir_all(&cvh_dir).await;
    let log_path = cvh_dir.join("llm_debug.log");
    const MAX_BYTES: u64 = 10 * 1024 * 1024; // 10MB
    if let Ok(meta) = tokio::fs::metadata(&log_path).await {
        if meta.len() >= MAX_BYTES {
            let rot_path = cvh_dir.join("llm_debug.log.1");
            let _ = tokio::fs::remove_file(&rot_path).await;
            let _ = tokio::fs::rename(&log_path, &rot_path).await;
        }
    }
    if let Ok(mut file) = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .await
    {
        use tokio::io::AsyncWriteExt;
        let _ = file.write_all(text.as_bytes()).await;
    }
}

/// Helper to look up a completed subagent's report from SQLite dialog_items.
pub fn find_subagent_report_from_db(
    root: &Path,
    parent_task_id: Option<&str>,
    subagent_id: &str,
) -> Option<String> {
    let p_trim = parent_task_id.map(str::trim).unwrap_or("");
    let s_trim = subagent_id.trim();

    let candidate_keys = [
        if s_trim.contains(":sub:") { Some(s_trim.to_string()) } else { None },
        if !p_trim.is_empty() { Some(format!("{}:sub:{}", p_trim, s_trim)) } else { None },
        if !p_trim.is_empty() && !s_trim.contains(".sub-") {
            Some(format!("{}:sub:{}.sub-{}", p_trim, p_trim, s_trim))
        } else {
            None
        },
        Some(s_trim.to_string()),
    ];

    for key_opt in candidate_keys.into_iter().flatten() {
        if let Ok(safe_id) = crate::commands::dialogs::sanitize_task_id(&key_opt) {
            if let Ok(items) = crate::db::load_dialog_items(root, &safe_id) {
                if let Some(out_item) = items.iter().rev().find(|it| {
                    it.item_type == "output"
                        && it.content.as_ref().map(|c| !c.trim().is_empty()).unwrap_or(false)
                }) {
                    if let Some(ref text) = out_item.content {
                        return Some(format!("[Subagent ID: {}]
{}", s_trim, text.trim()));
                    }
                }
            }
        }
    }

    None
}

/// Helper to remove a receiver from SUBAGENT_RECEIVERS either by exact ID or by `.sub-{id}` suffix.
pub fn remove_subagent_receiver(
    reg: &mut HashMap<String, SubagentReceiver>,
    id: &str,
) -> Option<(String, SubagentReceiver)> {
    let trimmed = id.trim();
    if let Some(rx) = reg.remove(trimmed) {
        return Some((trimmed.to_string(), rx));
    }
    let suffix = if trimmed.starts_with(".sub-") {
        trimmed.to_string()
    } else {
        format!(".sub-{}", trimmed)
    };
    let matching_key = reg.keys().find(|k| k.ends_with(&suffix)).cloned();
    if let Some(key) = matching_key {
        if let Some(rx) = reg.remove(&key) {
            return Some((key, rx));
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct ExecutedSubagentCall {
    pub id: String,
    pub tool_name: String,
    pub role: String,
    pub prompt: String,
    pub result: String,
}


/// Robust subagent prompt similarity check for single-turn deduplication.
pub fn are_subagent_prompts_duplicate(p1: &str, p2: &str) -> bool {
    let t1 = p1.trim();
    let t2 = p2.trim();
    if t1 == t2 {
        return true;
    }
    let ws1: String = t1.split_whitespace().collect::<Vec<_>>().join(" ");
    let ws2: String = t2.split_whitespace().collect::<Vec<_>>().join(" ");
    if ws1 == ws2 {
        return true;
    }
    if ws1.len() >= 20 && ws2.len() >= 20 {
        if ws1.starts_with(&ws2) || ws2.starts_with(&ws1) {
            return true;
        }
        let set1: std::collections::HashSet<&str> = ws1.split(' ').collect();
        let set2: std::collections::HashSet<&str> = ws2.split(' ').collect();
        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();
        if union > 0 && (intersection as f64 / union as f64) >= 0.85 {
            return true;
        }
    }
    false
}

/// Determines if a subagent role is an exploratory/architect/researcher persona
/// subject to single-turn or in-flight deduplication.
#[allow(dead_code)]
pub fn is_exploratory_role(role: &str) -> bool {
    let r = role.trim().to_lowercase();
    r == "explorer"
        || r == "architect"
        || r == "researcher"
        || r == "scout"
        || r == "code_explorer"
        || r == "system_architect"
        || r.contains("explorer")
        || r.contains("architect")
        || r.contains("researcher")
}

/// Canonical tool name normalization for aliases and SWE-bench compatibility.
pub fn canonical_tool_name(tool_name: &str) -> &str {
    crate::config::normalize_tool_name(tool_name)
}

pub async fn get_stage_personas(repo_root: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(boards_cfg) = crate::config::load_boards_config(repo_root).await {
        if let Some(active) = boards_cfg.boards.iter().find(|b| b.id == boards_cfg.active_board_id) {
            for c in &active.columns {
                if let Some(ref p) = c.persona {
                    map.insert(c.id.clone(), p.clone());
                }
            }
        }
        for b in &boards_cfg.boards {
            if b.id != boards_cfg.active_board_id {
                for c in &b.columns {
                    if let Some(ref p) = c.persona {
                        map.entry(c.id.clone()).or_insert_with(|| p.clone());
                    }
                }
            }
        }
    }
    if let Ok(pipeline) = crate::config::load_pipeline_config(repo_root).await {
        for c in pipeline.columns {
            if let Some(p) = c.persona {
                map.entry(c.id).or_insert(p);
            }
        }
    }
    map
}

pub fn resolve_task_role(stage_id: &str, personas: &HashMap<String, String>) -> String {
    personas.get(stage_id).cloned().unwrap_or_else(|| {
        match stage_id {
            "dev" => "developer".to_string(),
            "review" => "reviewer".to_string(),
            "retro" => "retro-verifier".to_string(),
            other => other.to_string(),
        }
    })
}

/// Executes a tool within the safe boundary of `worktree_dir` with zero-trust policy checks
/// (global/stage matrix AND optional per-agent `allowed_tools` intersection).
pub async fn execute_tool(
    ctx: &ToolContext<'_>,
    tool_name: &str,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;
    let worktree_dir = ctx.worktree_dir;
    let stage_id = ctx.stage_id;
    let abort_handle = Arc::clone(&ctx.abort_handle);
    let tool_policy = ctx.tool_policy;
    let task_id = ctx.task_id;
    let agent_allowed_tools = ctx.agent_allowed_tools;

    if abort_handle.load(Ordering::SeqCst) {
        bail!("Operation aborted");
    }

    // SWE-bench signatures & aliasing
    let normalized_tool_name = canonical_tool_name(tool_name);

    // Zero-Trust Iron Barrier (§ 2.16): verify tool permission for current stage & role
    if !is_tool_allowed(tool_policy, stage_id, normalized_tool_name) {
        bail!(
            "PermissionDenied: Инструмент '{}' отключен политикой безопасности для колонки '{}'",
            normalized_tool_name,
            stage_id.unwrap_or("global")
        );
    }
    if let Some(role) = ctx.role {
        if tool_policy.stage_overrides.contains_key(role) && !is_tool_allowed(tool_policy, Some(role), normalized_tool_name) {
            bail!(
                "PermissionDenied: Инструмент '{}' отключен политикой безопасности для агента '{}'",
                normalized_tool_name,
                role
            );
        }
    }
    if let Some(allowed) = agent_allowed_tools {
        if !agent_allows_tool(Some(allowed), normalized_tool_name) {
            bail!(
                "PermissionDenied: Инструмент '{}' отключен конфигурацией агента",
                normalized_tool_name
            );
        }
    }

    // Subagent boundary (§ 2.8, § 2.16 & Invariant 2.32):
    let is_subagent = task_id.map(|t| t.contains(".sub-") || t.contains(":sub:")).unwrap_or(false);
    if is_subagent && agent_allowed_tools.is_some() && !agent_allows_tool(agent_allowed_tools, normalized_tool_name) {
        bail!(
            "PermissionDenied: Инструмент '{}' запрещён для субагента (§ 2.8)",
            normalized_tool_name
        );
    }

    // § 2.8 Invariant: task card manipulation tools ("create_task", "complete_task")
    // are reserved for UI/orchestrators unless explicitly allowed by tool-policy.json (§ 2.5).
    if normalized_tool_name == "create_task" || normalized_tool_name == "create_tasks" {
        let role = ctx.role.unwrap_or("");
        let is_allowed = is_tool_active_for_agent(
            normalized_tool_name,
            tool_policy,
            role,
            ctx.stage_id,
            agent_allowed_tools,
        );
        if !is_allowed {
            bail!(
                "PermissionDenied: Агент '{}' не может использовать '{}': создание карточек задач зарезервировано для оркестраторов (§ 2.8)",
                role,
                normalized_tool_name
            );
        }
    }
    if normalized_tool_name == "complete_task" {
        let role = ctx.role.unwrap_or("");
        let is_allowed = is_tool_active_for_agent(
            normalized_tool_name,
            tool_policy,
            role,
            ctx.stage_id,
            agent_allowed_tools,
        );
        if !is_allowed {
            bail!(
                "PermissionDenied: Агент '{}' не может завершать задачи (§ 2.8). Завершение карточек зарезервировано для оркестратора и пользователя.",
                role
            );
        }
    }

    let raw_result = match normalized_tool_name {
        "read_file" => {
            let rel_path = args
                .get("file_path")
                .or_else(|| args.get("path"))
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'file_path'")?;

            let target = resolve_safe_path(worktree_dir, rel_path)?;
            if !target.exists() {
                bail!("File does not exist: {:?}", rel_path);
            }

            let default_limit = ctx
                .tool_policy
                .tool_settings
                .get("read_file")
                .and_then(|v| v.get("max_lines"))
                .and_then(|v| v.as_u64())
                .unwrap_or(2000) as usize;
            let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(default_limit);

            let content = tokio::fs::read_to_string(&target)
                .await
                .with_context(|| format!("Failed to read file: {:?}", rel_path))?;

            let lines: Vec<&str> = content.lines().collect();
            let start = offset.saturating_sub(1);
            if start >= lines.len() {
                return Ok("(File is empty or offset out of range)\n".to_string());
            }
            let end = (start + limit).min(lines.len());
            if start > end {
                return Ok("(File is empty or offset out of range)\n".to_string());
            }

            let mut output = String::new();
            for (idx, line) in lines[start..end].iter().enumerate() {
                output.push_str(&format!("{}: {}\n", start + idx + 1, line));
            }

            if output.is_empty() {
                output.push_str("(File is empty or offset out of range)\n");
            }

            output
        }

        "write_file" => {
            let rel_path = args
                .get("file_path")
                .or_else(|| args.get("path"))
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'file_path'")?;
            let content = args
                .get("content")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'content'")?;

            let target = resolve_safe_path(worktree_dir, rel_path)?;
            crate::config::atomic_write_file(&target, content).await?;
            emit_file_change_events(ctx, rel_path).await;
            format!("Successfully wrote {} bytes to {:?}", content.len(), rel_path)
        }

        "edit_file" => {
            let rel_path = args
                .get("file_path")
                .or_else(|| args.get("path"))
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'file_path'")?;
            let old_string = args
                .get("old_string")
                .or_else(|| args.get("old_str"))
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'old_string'")?;
            let new_string = args
                .get("new_string")
                .or_else(|| args.get("new_str"))
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'new_string'")?;
            let replace_all = args.get("replace_all").and_then(|v| v.as_bool()).unwrap_or(false);

            let target = resolve_safe_path(worktree_dir, rel_path)?;
            let current = tokio::fs::read_to_string(&target)
                .await
                .with_context(|| format!("Failed to read file for edit: {:?}", rel_path))?;

            let updated = match replace_text_normalized(&current, old_string, new_string, replace_all) {
                Ok(u) => u,
                Err(err) if err == "NOT_FOUND" => {
                    let informative_err = build_edit_mismatch_error(&current, rel_path, old_string);
                    bail!(informative_err);
                }
                Err(err) => {
                    bail!(err);
                }
            };

            crate::config::atomic_write_file(&target, &updated).await?;
            emit_file_change_events(ctx, rel_path).await;
            format!("Successfully updated {:?}", rel_path)
        }

        "glob" => {
            let pattern = args
                .get("pattern")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'pattern'")?;
            let sub_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let search_dir = resolve_safe_path(worktree_dir, sub_path)?;

            let mut results = Vec::new();
            collect_matching_files(&search_dir, worktree_dir, pattern, &mut results).await?;
            if results.is_empty() {
                "No matching files found.".to_string()
            } else {
                results.join("\n")
            }
        }

        "grep" => {
            let pattern = args
                .get("pattern")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'pattern'")?;
            let sub_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let search_target = resolve_safe_path(worktree_dir, sub_path)?;

            let mut matches = Vec::new();
            grep_walk(&search_target, worktree_dir, pattern, &mut matches).await?;

            if matches.is_empty() {
                "No matches found.".to_string()
            } else {
                matches.join("\n")
            }
        }

        "execute_command" => {
            let command_str = args
                .get("command")
                .or_else(|| args.get("cmd"))
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'command'")?;
            let default_timeout = ctx
                .tool_policy
                .tool_settings
                .get("execute_command")
                .and_then(|v| v.get("timeout_sec"))
                .and_then(|v| v.as_u64())
                .unwrap_or(180);
            let timeout_secs = args
                .get("timeout_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(default_timeout);

            let res = run_scoped_command(worktree_dir, command_str, Duration::from_secs(timeout_secs), Arc::clone(&abort_handle)).await?;
            format!(
                "[Exit Code: {}]\n--- STDOUT ---\n{}\n--- STDERR ---\n{}",
                res.exit_code.unwrap_or(-1),
                res.stdout.trim(),
                res.stderr.trim()
            )
        }

        "web_search" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'query'")?;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            execute_web_search(query, limit).await?
        }

        "web_fetch" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'url'")?;
            execute_web_fetch(url).await?
        }

        "subagent" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'title'")?;
            let prompt = args
                .get("prompt")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'prompt'")?;
            let role = args
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("general");
            let requested_tools: Option<Vec<String>> = args.get("tools").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()
            });

            let parent_id = ctx.task_id.unwrap_or("unknown");

            // Deduplication check across sessions:
            // If an identical subagent already completed with report, return it.
            {
                let sessions = SUBAGENT_SESSIONS.lock().await;
                for sess in sessions.values() {
                    let info = sess.retry_info.lock().await;
                    if info.parent_task_id == parent_id {
                        let is_dup = info.role.eq_ignore_ascii_case(role) && are_subagent_prompts_duplicate(&info.prompt, prompt);

                        if is_dup && info.status == "finished" {
                            if let Some(ref rep) = info.report {
                                eprintln!("CVH [agent]: Reusing completed subagent report for request (role: '{}')", info.role);
                                return Ok(rep.clone());
                            }
                        }
                    }
                }
            }

            let subagent_suffix = match ctx.call_id {
                Some(cid) if !cid.trim().is_empty() => cid.trim().to_string(),
                _ => format!(
                    "{}-{}",
                    chrono::Utc::now().timestamp_millis(),
                    uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>()
                ),
            };
            let subagent_id = format!("{}.sub-{}", parent_id, subagent_suffix);
            let tc_id_opt = ctx.call_id.map(|s| s.to_string());

            let sub_abort_handle = Arc::new(AtomicBool::new(false));
            let sub_join_slot = Arc::new(tokio::sync::Mutex::new(None));

            let (tx, rx) = oneshot::channel::<String>();
            let retry_info = SubagentRetryInfo {
                parent_task_id: parent_id.to_string(),
                subagent_id: subagent_id.clone(),
                tool_call_id: tc_id_opt,
                title: title.to_string(),
                role: role.to_string(),
                prompt: prompt.to_string(),
                requested_tools: requested_tools.clone(),
                ctx: ctx.to_owned(),
                status: "running".to_string(),
                last_error: None,
                report: None,
                is_bg: false,
                duration_ms: None,
                steps: 0,
            };

            let session = SubagentSession {
                sender: Arc::new(tokio::sync::Mutex::new(Some(tx))),
                retry_info: Arc::new(tokio::sync::Mutex::new(retry_info)),
                abort_handle: Arc::clone(&sub_abort_handle),
                join_handle: Arc::clone(&sub_join_slot),
            };

            {
                let mut sessions = SUBAGENT_SESSIONS.lock().await;
                sessions.insert(subagent_id.clone(), session);
            }
            prune_subagent_sessions_cache().await;

            let mut owned_ctx = ctx.to_owned();
            owned_ctx.abort_handle = Arc::clone(&sub_abort_handle);
            let sub_id = subagent_id.clone();
            let title_s = title.to_string();
            let role_s = role.to_string();
            let prompt_s = prompt.to_string();

            let join_h = tokio::spawn(async move {
                let _ = run_inprocess_subagent(
                    owned_ctx,
                    title_s,
                    role_s,
                    prompt_s,
                    requested_tools,
                    Some(sub_id),
                ).await;
            });
            *sub_join_slot.lock().await = Some(join_h);

            // Register into state.active_agents so abort_agent can track/abort it
            if let Some(ref app) = ctx.app_handle {
                if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                    if let Ok(mut map) = state_ref.active_agents.lock() {
                        map.insert(
                            subagent_id.clone(),
                            crate::commands::ActiveAgentHandle {
                                abort_flag: Arc::clone(&sub_abort_handle),
                                join_handle: Arc::clone(&sub_join_slot),
                            },
                        );
                    }
                }
            }

            // Parent agent blocks synchronously awaiting clean report from channel (Zero-Pollution Invariant!)
            let mut rx = rx;
            let clean_report = loop {
                tokio::select! {
                    res = &mut rx => {
                        match res {
                            Ok(report) => break report,
                            Err(_) => {
                                if sub_abort_handle.load(Ordering::SeqCst) {
                                    return Ok("[Субагент прерван пользователем / Subagent aborted by user]".to_string());
                                }
                                bail!("Subagent channel closed before completion");
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(200)) => {
                        if abort_handle.load(Ordering::SeqCst) {
                            sub_abort_handle.store(true, Ordering::SeqCst);
                            bail!("Agent execution aborted while waiting for subagent");
                        }
                        if sub_abort_handle.load(Ordering::SeqCst) {
                            return Ok("[Субагент прерван пользователем / Subagent aborted by user]".to_string());
                        }
                    }
                }
            };
            clean_report
        }

        "subagent_bg" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'title'")?;
            let prompt = args
                .get("prompt")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'prompt'")?;
            let role = args
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("general");
            let requested_tools: Option<Vec<String>> = args.get("tools").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()
            });

            let parent_id = ctx.task_id.unwrap_or("unknown");

            // Deduplication check across sessions: if an identical subagent is already actively running or finished, reuse it.
            {
                let sessions = SUBAGENT_SESSIONS.lock().await;
                for (sid, sess) in sessions.iter() {
                    let info = sess.retry_info.lock().await;
                    if info.parent_task_id == parent_id
                        && (info.status == "running" || info.status == "finished")
                    {
                        let is_dup = info.role.eq_ignore_ascii_case(role) && are_subagent_prompts_duplicate(&info.prompt, prompt);

                        if is_dup {
                            eprintln!("CVH [agent]: Reusing existing subagent {} (role: '{}') for duplicate subagent_bg request", sid, info.role);
                            return Ok(serde_json::to_string(&serde_json::json!({
                                "subagent_id": sid,
                                "status": &info.status,
                                "note": format!("Reused existing {} subagent to prevent duplicate subagent_bg execution", info.role)
                            }))?);
                        }
                    }
                }
            }

            let subagent_suffix = match ctx.call_id {
                Some(cid) if !cid.trim().is_empty() => cid.trim().to_string(),
                _ => format!(
                    "{}-{}",
                    chrono::Utc::now().timestamp_millis(),
                    uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>()
                ),
            };
            let subagent_id = format!("{}.sub-{}", parent_id, subagent_suffix);
            let tc_id_opt = ctx.call_id.map(|s| s.to_string());

            let sub_abort_handle = Arc::new(AtomicBool::new(false));
            let sub_join_slot = Arc::new(tokio::sync::Mutex::new(None));

            let (tx, rx) = oneshot::channel::<String>();
            let retry_info = SubagentRetryInfo {
                parent_task_id: parent_id.to_string(),
                subagent_id: subagent_id.clone(),
                tool_call_id: tc_id_opt,
                title: title.to_string(),
                role: role.to_string(),
                prompt: prompt.to_string(),
                requested_tools: requested_tools.clone(),
                ctx: ctx.to_owned(),
                status: "running".to_string(),
                last_error: None,
                report: None,
                is_bg: true,
                duration_ms: None,
                steps: 0,
            };

            let session = SubagentSession {
                sender: Arc::new(tokio::sync::Mutex::new(Some(tx))),
                retry_info: Arc::new(tokio::sync::Mutex::new(retry_info)),
                abort_handle: Arc::clone(&sub_abort_handle),
                join_handle: Arc::clone(&sub_join_slot),
            };

            {
                let mut receivers = SUBAGENT_RECEIVERS.lock().await;
                receivers.insert(subagent_id.clone(), rx);
                let mut sessions = SUBAGENT_SESSIONS.lock().await;
                sessions.insert(subagent_id.clone(), session);
            }
            prune_subagent_sessions_cache().await;

            let mut owned_ctx = ctx.to_owned();
            owned_ctx.abort_handle = Arc::clone(&sub_abort_handle);
            let sub_id = subagent_id.clone();
            let title_s = title.to_string();
            let role_s = role.to_string();
            let prompt_s = prompt.to_string();

            let join_h = tokio::spawn(async move {
                let _ = run_inprocess_subagent(
                    owned_ctx,
                    title_s,
                    role_s,
                    prompt_s,
                    requested_tools,
                    Some(sub_id),
                ).await;
            });
            *sub_join_slot.lock().await = Some(join_h);

            // Register into state.active_agents so abort_agent can track/abort it
            if let Some(ref app) = ctx.app_handle {
                if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                    if let Ok(mut map) = state_ref.active_agents.lock() {
                        map.insert(
                            subagent_id.clone(),
                            crate::commands::ActiveAgentHandle {
                                abort_flag: Arc::clone(&sub_abort_handle),
                                join_handle: Arc::clone(&sub_join_slot),
                            },
                        );
                    }
                }
            }

            serde_json::to_string(&serde_json::json!({
                "subagent_id": subagent_id,
                "status": "running"
            }))?
        }

        "collect_subagent" => {
            let mut ids: Vec<String> = Vec::new();
            if let Some(id) = args.get("subagent_id").and_then(|v| v.as_str()) {
                if !id.trim().is_empty() {
                    ids.push(id.trim().to_string());
                }
            }
            if let Some(arr) = args.get("subagent_ids").and_then(|v| v.as_array()) {
                for v in arr {
                    if let Some(s) = v.as_str() {
                        let trimmed = s.trim();
                        if !trimmed.is_empty() && !ids.iter().any(|existing| existing == trimmed) {
                            ids.push(trimmed.to_string());
                        }
                    }
                }
            }

            if ids.is_empty() {
                bail!("Missing required parameter: provide either 'subagent_id' or 'subagent_ids'");
            }

            let mut collected_reports: Vec<(String, String)> = Vec::new();

            for subagent_id in &ids {
                // 1. Check SUBAGENT_RECEIVERS (in-flight background subagents)
                let rx_opt = {
                    let mut reg = SUBAGENT_RECEIVERS.lock().await;
                    remove_subagent_receiver(&mut reg, subagent_id)
                };

                if let Some((resolved_id, rx)) = rx_opt {
                    match rx.await {
                        Ok(report) => {
                            let mut sessions = SUBAGENT_SESSIONS.lock().await;
                            sessions.remove(&resolved_id);
                            collected_reports.push((resolved_id, report));
                            continue;
                        }
                        Err(_) => {
                            // 1. Check SUBAGENT_SESSIONS for completed report or aborted status
                            if let Some((_, session)) = find_subagent_session(task_id, &resolved_id).await {
                                let info = session.retry_info.lock().await;
                                if let Some(ref rep) = info.report {
                                    let mut sessions = SUBAGENT_SESSIONS.lock().await;
                                    sessions.remove(&resolved_id);
                                    collected_reports.push((resolved_id, rep.clone()));
                                    continue;
                                }
                                if info.status == "aborted" || session.abort_handle.load(Ordering::SeqCst) {
                                    let mut sessions = SUBAGENT_SESSIONS.lock().await;
                                    sessions.remove(&resolved_id);
                                    collected_reports.push((resolved_id, "[Субагент прерван пользователем / Subagent aborted by user]".to_string()));
                                    continue;
                                }
                            }

                            // 2. Check SQLite dialog_items for persisted subagent output report
                            if let Some(db_report) = find_subagent_report_from_db(repo_root, task_id, &resolved_id)
                                .or_else(|| find_subagent_report_from_db(repo_root, task_id, subagent_id))
                            {
                                let mut sessions = SUBAGENT_SESSIONS.lock().await;
                                sessions.remove(&resolved_id);
                                collected_reports.push((resolved_id, db_report));
                                continue;
                            }

                            let is_aborted = if let Some((_, session)) = find_subagent_session(task_id, &resolved_id).await {
                                let info = session.retry_info.lock().await;
                                info.status == "aborted" || session.abort_handle.load(Ordering::SeqCst)
                            } else {
                                false
                            };
                            let mut sessions = SUBAGENT_SESSIONS.lock().await;
                            sessions.remove(&resolved_id);
                            if is_aborted {
                                collected_reports.push((resolved_id, "[Субагент прерван пользователем / Subagent aborted by user]".to_string()));
                                continue;
                            }
                            bail!("Subagent [{}] channel closed before completion", resolved_id);
                        }
                    }
                }

                // 2. Check SUBAGENT_SESSIONS (synchronous or already completed subagents)
                if let Some((resolved_id, session)) = find_subagent_session(task_id, subagent_id).await {
                    let info = session.retry_info.lock().await;
                    if let Some(ref rep) = info.report {
                        collected_reports.push((resolved_id, rep.clone()));
                        continue;
                    }
                    if info.status == "aborted" || session.abort_handle.load(Ordering::SeqCst) {
                        collected_reports.push((resolved_id, "[Субагент прерван пользователем / Subagent aborted by user]".to_string()));
                        continue;
                    }
                    if let Some(db_report) = find_subagent_report_from_db(repo_root, task_id, &resolved_id)
                        .or_else(|| find_subagent_report_from_db(repo_root, task_id, subagent_id))
                    {
                        collected_reports.push((resolved_id, db_report));
                        continue;
                    }
                    if info.status == "error" {
                        let err = info.last_error.as_deref().unwrap_or("Unknown error");
                        bail!("Subagent [{}] failed with error: {}", resolved_id, err);
                    }
                }

                // 3. Check SQLite dialog_items for persisted subagent output report
                if let Some(db_report) = find_subagent_report_from_db(repo_root, task_id, subagent_id) {
                    collected_reports.push((subagent_id.clone(), db_report));
                    continue;
                }

                bail!("Subagent [{}] not found in background registry or session history", subagent_id);
            }

            if collected_reports.len() == 1 {
                collected_reports.remove(0).1
            } else {
                collected_reports
                    .into_iter()
                    .map(|(id, report)| format!("### Subagent [{}]
{}", id, report))
                    .collect::<Vec<_>>()
                    .join("

---

")
            }
        }

        "subagent_list" => {
            let status_filter = args
                .get("status_filter")
                .and_then(|v| v.as_str())
                .unwrap_or("all")
                .trim()
                .to_lowercase();

            let target_task_id = ctx.task_id.unwrap_or("");
            let mut list: Vec<serde_json::Value> = Vec::new();
            let mut seen_ids = std::collections::HashSet::new();

            // 1. Scan in-memory SUBAGENT_SESSIONS
            {
                let sessions = SUBAGENT_SESSIONS.lock().await;
                for (sid, sess) in sessions.iter() {
                    let info = sess.retry_info.lock().await;
                    if !target_task_id.is_empty() {
                        let matches_parent = info.parent_task_id == target_task_id
                            || sid.starts_with(&format!("{}.", target_task_id))
                            || sid.starts_with(target_task_id);
                        if !matches_parent {
                            continue;
                        }
                    }

                    let raw_status = if sess.abort_handle.load(Ordering::SeqCst) || info.status == "aborted" {
                        "aborted"
                    } else if info.status == "error" {
                        "error"
                    } else if info.status == "finished" || info.status == "completed" {
                        "completed"
                    } else if info.status == "running" {
                        "running"
                    } else {
                        info.status.as_str()
                    };

                    let norm_status = match raw_status {
                        "finished" => "completed",
                        s => s,
                    };

                    let matches_filter = match status_filter.as_str() {
                        "all" | "" => true,
                        "running" => norm_status == "running",
                        "completed" => norm_status == "completed",
                        "aborted" => norm_status == "aborted",
                        "error" => norm_status == "error",
                        f => norm_status == f,
                    };

                    if matches_filter {
                        let mut item = serde_json::json!({
                            "subagent_id": &info.subagent_id,
                            "title": &info.title,
                            "role": &info.role,
                            "status": norm_status,
                            "steps": info.steps,
                        });
                        if let Some(dur) = info.duration_ms {
                            item["duration_ms"] = serde_json::json!(dur.round() as u64);
                        }
                        list.push(item);
                    }
                    seen_ids.insert(info.subagent_id.clone());
                    seen_ids.insert(sid.clone());
                }
            }

            // 2. Check SQLite dialog_items for persisted subagents not in active sessions
            if !target_task_id.is_empty() {
                if let Ok(dialog_keys) = crate::db::list_subagent_dialog_keys(ctx.repo_root, target_task_id) {
                    for key in dialog_keys {
                        let sub_id = if key.contains(":sub:") {
                            key.rsplit(":sub:").next().unwrap_or(&key).to_string()
                        } else {
                            key.clone()
                        };

                        if seen_ids.contains(&sub_id) || seen_ids.contains(&key) {
                            continue;
                        }

                        if let Ok(items) = crate::db::load_dialog_items(ctx.repo_root, &key) {
                            if items.is_empty() {
                                continue;
                            }

                            let mut title = "Subagent".to_string();
                            let mut role = "general".to_string();
                            let mut has_error = false;
                            let mut has_abort = false;
                            let mut steps = 0;
                            let mut duration_ms: Option<f64> = None;

                            for it in &items {
                                if it.item_type == "tool_call" {
                                    steps += 1;
                                }
                                if it.item_type == "output" {
                                    if let Some(dur) = it.duration_ms {
                                        duration_ms = Some(dur);
                                    }
                                    if let Some(ref c) = it.content {
                                        if let Some(idx_sub) = c.find("Subagent [") {
                                            let after = &c[idx_sub + 10..];
                                            if let Some(idx_bracket) = after.find("] '") {
                                                role = after[..idx_bracket].to_string();
                                                let after_title = &after[idx_bracket + 3..];
                                                if let Some(idx_quote) = after_title.find("' report:") {
                                                    title = after_title[..idx_quote].to_string();
                                                }
                                            }
                                        }
                                    }
                                }
                                if it.item_type == "prompt" {
                                    if let Some(ref tool_meta) = it.tool {
                                        if let Some(t) = tool_meta.get("title").and_then(|v| v.as_str()) {
                                            title = t.to_string();
                                        }
                                        if let Some(r) = tool_meta.get("role").and_then(|v| v.as_str()) {
                                            role = r.to_string();
                                        }
                                    }
                                }
                                if it.item_type == "error" {
                                    has_error = true;
                                    if it.content.as_ref().map(|c| c.contains("aborted") || c.contains("прерван")).unwrap_or(false) {
                                        has_abort = true;
                                    }
                                }
                            }

                            let norm_status = if has_abort {
                                "aborted"
                            } else if has_error {
                                "error"
                            } else {
                                "completed"
                            };

                            let matches_filter = match status_filter.as_str() {
                                "all" | "" => true,
                                "running" => norm_status == "running",
                                "completed" => norm_status == "completed",
                                "aborted" => norm_status == "aborted",
                                "error" => norm_status == "error",
                                f => norm_status == f,
                            };

                            if matches_filter {
                                let mut item = serde_json::json!({
                                    "subagent_id": &sub_id,
                                    "title": &title,
                                    "role": &role,
                                    "status": norm_status,
                                    "steps": steps,
                                });
                                if let Some(dur) = duration_ms {
                                    item["duration_ms"] = serde_json::json!(dur.round() as u64);
                                }
                                list.push(item);
                            }
                            seen_ids.insert(sub_id);
                            seen_ids.insert(key);
                        }
                    }
                }
            }

            serde_json::to_string_pretty(&list)?
        }

        "todo_write" => {
            let raw_todos = args
                .get("todos")
                .and_then(|v| v.as_array())
                .context("Missing required parameter 'todos' (must be array)")?;

            let total_raw = raw_todos.len();

            let _min_items = ctx
                .tool_policy
                .tool_settings
                .get("todo_write")
                .and_then(|v| v.get("min_items"))
                .and_then(|v| v.as_u64())
                .unwrap_or(3) as usize;

            let max_items = ctx
                .tool_policy
                .tool_settings
                .get("todo_write")
                .and_then(|v| v.get("max_items"))
                .and_then(|v| v.as_u64())
                .unwrap_or(20) as usize;

            let mut parsed_todos: Vec<TodoItem> = Vec::new();
            let mut completed_cnt = 0;
            let mut in_progress_cnt = 0;
            let mut pending_cnt = 0;

            for (idx, item) in raw_todos.iter().enumerate() {
                if parsed_todos.len() >= max_items {
                    break;
                }
                let content = item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if content.is_empty() {
                    continue;
                }
                let raw_status = item
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pending")
                    .trim()
                    .to_lowercase();

                let status = match raw_status.as_str() {
                    "completed" | "done" => {
                        completed_cnt += 1;
                        "completed".to_string()
                    }
                    "in_progress" | "running" => {
                        in_progress_cnt += 1;
                        "in_progress".to_string()
                    }
                    _ => {
                        pending_cnt += 1;
                        "pending".to_string()
                    }
                };

                let id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("todo-{}", idx + 1));

                parsed_todos.push(TodoItem { id, content, status });
            }

            // If task_id provided, atomically persist to .cvh/tasks.json or SQLite dialog
            if let Some(tid) = task_id {
                if tid.contains(".sub-") {
                    let subagent_dialog_id = if tid.contains(":sub:") {
                        tid.to_string()
                    } else if let Some((parent_id, _)) = tid.split_once(".sub-") {
                        format!("{}:sub:{}", parent_id, tid)
                    } else {
                        tid.to_string()
                    };
                    let todo_act = ActivityItem {
                        id: format!("sub-todos-{}", chrono::Utc::now().timestamp_millis()),
                        item_type: "todos".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        content: serde_json::to_string(&parsed_todos).ok(),
                        tool: None,
                        duration_ms: None,
                        images: None,
                    };
                    let _ = crate::db::append_dialog_items(repo_root, &subagent_dialog_id, std::slice::from_ref(&todo_act));
                    if let Some(ref app) = ctx.app_handle {
                        let _ = app.emit("subagent:activity", serde_json::json!({
                            "subagentId": tid,
                            "activity": &todo_act,
                        }));
                    }
                } else {
                    // Atomically save todos to SQLite table task_todos
                    let _ = crate::db::db_save_todos(repo_root, tid, &parsed_todos);

                    // Update in SQLite tasks table if task exists
                    if let Ok(Some(mut db_task)) = crate::db::db_get_task(repo_root, tid) {
                        db_task.todos = Some(parsed_todos.clone());
                        db_task.updated_at = chrono::Utc::now().timestamp_millis();
                        let _ = crate::db::db_upsert_task(repo_root, &db_task);
                    }
                }
            }

            if total_raw > max_items {
                let truncated_count = total_raw - max_items;
                format!(
                    "Updated todo list: {} completed, {} in progress, {} pending. (Note: {} items were truncated to stay within the max limit of {}).",
                    completed_cnt, in_progress_cnt, pending_cnt, truncated_count, max_items
                )
            } else {
                format!(
                    "Updated todo list: {} completed, {} in progress, {} pending.",
                    completed_cnt, in_progress_cnt, pending_cnt
                )
            }
        }

        "advance_task" | "move_task" => {
            let summary = args
                .get("summary")
                .or_else(|| args.get("handover_summary"))
                .or_else(|| args.get("description"))
                .or_else(|| args.get("comment"))
                .or_else(|| args.get("message"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("Задача выполнена и передана на следующий этап");

            let target_stage = args
                .get("stage_id")
                .or_else(|| args.get("target_stage"))
                .or_else(|| args.get("column_id"))
                .or_else(|| args.get("target_stage_id"))
                .or_else(|| args.get("to_stage"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());

            let task_id = args
                .get("task_id")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .or(task_id)
                .context("No active task ID for advance_task / move_task")?;

            crate::commands::internal_advance_task_to_stage(
                repo_root,
                task_id,
                target_stage,
                summary,
                ctx.app_handle.as_ref(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?
        }

        "reject_task" => {
            let reason = args
                .get("reason")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'reason'")?;

            let task_id = args
                .get("task_id")
                .or_else(|| args.get("taskId"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .or(task_id)
                .context("No active task ID for reject_task")?;

            // 1. Determine active board and find current column
            let tasks = crate::commands::load_tasks_from_disk(repo_root).await;
            let current_task = tasks
                .iter()
                .find(|t| crate::commands::task_id_matches(&t.id, task_id))
                .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;

            let boards_cfg = crate::config::load_boards_config(repo_root).await.ok();
            let columns = {
                let from_board = boards_cfg.as_ref().and_then(|bc| {
                    let b = if let Some(ref bid) = current_task.board_id {
                        bc.boards.iter().find(|b| &b.id == bid)
                    } else {
                        bc.boards.iter().find(|b| b.id == bc.active_board_id)
                    };
                    b.map(|b| b.columns.clone())
                });
                match from_board {
                    Some(cols) => cols,
                    None => {
                        let pipeline = crate::config::load_pipeline_config(repo_root).await
                            .context("Failed to load pipeline configuration")?;
                        pipeline.columns
                    }
                }
            };

            let curr_idx = columns
                .iter()
                .position(|c| c.id == current_task.stage_id)
                .ok_or_else(|| anyhow::anyhow!("Current column '{}' not found in pipeline", current_task.stage_id))?;

            if curr_idx == 0 {
                bail!("Задача {} уже находится в начальной колонке ({})", task_id, columns[curr_idx].title);
            }

            let prev_col = &columns[curr_idx - 1];

            // ЖЕЛЕЗНЫЙ ИНВАРИАНТ: Если предыдущая колонка — колонка приёма/бэклог (индекс 0 или is_intake),
            // шаг назад в бэклог СТРОГО ЗАПРЕЩЁН! Возвращай ошибку:
            // "Запрещено возвращать задачу из первой рабочей стадии назад в бэклог. Задача должна быть доработана или закрыта."
            let is_prev_backlog = prev_col.is_intake == Some(true)
                || curr_idx == 1
                || prev_col.id == "backlog"
                || prev_col.title.to_lowercase().contains("backlog")
                || prev_col.title.to_lowercase().contains("бэклог");

            if is_prev_backlog {
                bail!("Запрещено возвращать задачу из первой рабочей стадии назад в бэклог. Задача должна быть доработана или закрыта.");
            }

            let reject_content = format!(
                "### Задача возвращена на этап: {}\n\n**Причина отклонения:**\n{}",
                prev_col.title,
                reason
            );

            // If current task is a subtask, find the previous subtask in prev_col and reactivate it
            let subtask_rejection = if let Some(ref pid) = current_task.parent_id {
                let prev_subtask_opt = tasks
                    .iter()
                    .filter(|t| {
                        t.parent_id.as_deref().is_some_and(|p| crate::commands::task_id_matches(p, pid))
                            && crate::commands::stage_id_matches(&t.stage_id, &prev_col.id)
                            && !crate::commands::task_id_matches(&t.id, task_id)
                    })
                    .max_by_key(|t| (t.subtask_index.unwrap_or(0), t.created_at))
                    .cloned();

                if let Some(prev_sub) = prev_subtask_opt {
                    let mut mut_tasks = tasks.clone();
                    let prev_col_has_autorun = prev_col.persona.is_some() || prev_col.auto_run == Some(true);
                    let prev_status = if prev_col_has_autorun { "running" } else { "idle" };

                    // 1. Reactivate prev_subtask in prev_col
                    if let Some(ps) = mut_tasks.iter_mut().find(|t| crate::commands::task_id_matches(&t.id, &prev_sub.id)) {
                        ps.status = Some(prev_status.to_string());
                        ps.verdict = Some("CHANGES_REQUESTED".to_string());
                        ps.handover_summary = Some(format!("ОТКЛОНЕНО: {}", reason));
                        ps.updated_at = chrono::Utc::now().timestamp_millis();
                        let _ = crate::db::db_upsert_task(repo_root, ps);
                    }

                    // 2. Keep rejecting subtask in its stage with status idle and verdict CHANGES_REQUESTED
                    if let Some(cs) = mut_tasks.iter_mut().find(|t| crate::commands::task_id_matches(&t.id, task_id)) {
                        cs.status = Some("idle".to_string());
                        cs.verdict = Some("CHANGES_REQUESTED".to_string());
                        cs.updated_at = chrono::Utc::now().timestamp_millis();
                        let _ = crate::db::db_upsert_task(repo_root, cs);
                    }

                    // 3. Update parent task waiting_for_subtask_id
                    if let Some(p) = mut_tasks.iter_mut().find(|t| crate::commands::task_id_matches(&t.id, pid)) {
                        p.waiting_for_subtask_id = Some(prev_sub.id.clone());
                        p.status = Some("waiting_subtask".to_string());
                        p.updated_at = chrono::Utc::now().timestamp_millis();
                        let _ = crate::db::db_upsert_task(repo_root, p);
                    }

                    // Save to disk and AppState
                    let _ = crate::commands::save_tasks_to_disk(repo_root, &mut_tasks).await;
                    if let Some(app) = ctx.app_handle.as_ref() {
                        if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                            if let Ok(mut guard) = state_ref.tasks.lock() {
                                *guard = mut_tasks.clone();
                            }
                        }
                    }

                    // Append dialog activities
                    let _ = crate::commands::append_task_dialog_activity(
                        repo_root,
                        task_id,
                        "reject",
                        &reject_content,
                        ctx.app_handle.as_ref(),
                    )
                    .await;

                    let _ = crate::commands::append_task_dialog_activity(
                        repo_root,
                        &prev_sub.id,
                        "steer",
                        &reject_content,
                        ctx.app_handle.as_ref(),
                    )
                    .await;

                    // Emit events
                    if let Some(app) = ctx.app_handle.as_ref() {
                        if let Some(cs) = mut_tasks.iter().find(|t| crate::commands::task_id_matches(&t.id, task_id)) {
                            let _ = app.emit("task:updated", serde_json::json!({ "task": cs }));
                        }
                        if let Some(ps) = mut_tasks.iter().find(|t| crate::commands::task_id_matches(&t.id, &prev_sub.id)) {
                            let _ = app.emit("task:updated", serde_json::json!({ "task": ps }));
                            let _ = app.emit("task:moved", serde_json::json!({
                                "taskId": ps.id,
                                "toStage": prev_col.id,
                                "stageId": prev_col.id,
                                "status": prev_status,
                                "rejected": true,
                                "reason": reason,
                                "verdict": "CHANGES_REQUESTED",
                                "task": ps,
                            }));
                        }
                        if let Some(p) = mut_tasks.iter().find(|t| crate::commands::task_id_matches(&t.id, pid)) {
                            let _ = app.emit("task:updated", serde_json::json!({ "task": p }));
                        }
                        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &mut_tasks }));

                        if prev_col_has_autorun {
                            let _ = app.emit("agent:autorun", serde_json::json!({
                                "taskId": prev_sub.id,
                                "prompt": reject_content,
                            }));
                        }
                    }

                    Some(format!(
                        "ОТКЛОНЕНО: Задача {} возвращена на этап '{}' ({}) к подзадаче {} по причине: {}",
                        task_id,
                        prev_col.title,
                        prev_col.id,
                        prev_sub.id,
                        reason
                    ))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(res_str) = subtask_rejection {
                res_str
            } else {
                // Fallback for single tasks or subtasks without previous subtask
                let rejection_summary = format!("ОТКЛОНЕНО РЕВЬЮ: {}", reason);
                let updated_task = crate::commands::internal_move_task_stage(
                    repo_root,
                    task_id,
                    &prev_col.id,
                    Some("CHANGES_REQUESTED".to_string()),
                    Some("idle".to_string()),
                    Some(rejection_summary),
                    ctx.app_handle.as_ref(),
                    true, // is_autonomous_advance
                    None,
                )
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

                let _ = crate::commands::append_task_dialog_activity(
                    repo_root,
                    task_id,
                    "reject",
                    &reject_content,
                    ctx.app_handle.as_ref(),
                )
                .await;

                // Emit task:moved with reject marker and reason
                if let Some(app) = ctx.app_handle.as_ref() {
                    let _ = app.emit("task:moved", serde_json::json!({
                        "taskId": task_id,
                        "toStage": prev_col.id,
                        "stageId": prev_col.id,
                        "status": "idle",
                        "rejected": true,
                        "reason": reason,
                        "verdict": "CHANGES_REQUESTED",
                        "task": &updated_task,
                    }));
                    let prev_has_autorun = prev_col.persona.is_some() || prev_col.auto_run == Some(true);
                    if prev_has_autorun {
                        let _ = app.emit("agent:autorun", serde_json::json!({
                            "taskId": task_id,
                            "prompt": reject_content,
                        }));
                    }
                }

                format!(
                    "ОТКЛОНЕНО: Задача {} возвращена на этап '{}' ({}) по причине: {}",
                    task_id,
                    prev_col.title,
                    prev_col.id,
                    reason
                )
            }
        }

        "question" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'text'")?;
            let options = args.get("options").cloned();
            let multiple = args.get("multiple").and_then(|v| v.as_bool()).unwrap_or(false);

            let question_id = uuid::Uuid::new_v4().to_string();
            let task_id_str = args
                .get("task_id")
                .or_else(|| args.get("taskId"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .or(task_id)
                .unwrap_or("unknown")
                .to_string();

            // 1. Thread-safe oneshot channel registration (zero-polling)
            let (tx, mut rx) = tokio::sync::oneshot::channel::<String>();
            {
                let mut reg = QUESTION_REGISTRY.lock().await;
                reg.insert(question_id.clone(), tx);
            }

            // 2. Persist activity item in SQLite dialog_items with type: "question"
            let timestamp_str = chrono::Utc::now().to_rfc3339();
            let question_payload = serde_json::json!({
                "taskId": task_id_str,
                "questionId": question_id,
                "text": text,
                "options": options,
                "multiple": multiple,
            });

            let activity_item = crate::db::ActivityItem {
                id: format!("q-{}", question_id),
                item_type: "question".to_string(),
                timestamp: timestamp_str,
                content: Some(text.to_string()),
                tool: Some(serde_json::json!({
                    "id": question_id,
                    "toolName": "question",
                    "status": "running",
                    "params": question_payload,
                })),
                duration_ms: None,
                images: None,
            };

            if let Err(e) = crate::db::append_dialog_items(repo_root, &task_id_str, &[activity_item]) {
                eprintln!("CVH [question]: Failed to persist question activity item: {}", e);
            }

            // 3. Emit Tauri event agent:question with { taskId, questionId, text, options, multiple }
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit(
                    "agent:question",
                    serde_json::json!({
                        "taskId": task_id_str,
                        "questionId": question_id,
                        "text": text,
                        "options": options,
                        "multiple": multiple,
                    }),
                );
            }

            // 4. Zero-Polling Tokio Pause: wait on rx.await (with abort check)
            let answer = loop {
                tokio::select! {
                    res = &mut rx => {
                        match res {
                            Ok(ans) => break ans,
                            Err(_) => bail!("Question channel was closed without an answer"),
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(250)) => {
                        if abort_handle.load(Ordering::SeqCst) {
                            let mut reg = QUESTION_REGISTRY.lock().await;
                            reg.remove(&question_id);
                            bail!("Agent execution was aborted while waiting for answer to question");
                        }
                    }
                }
            };

            // 5. Update question activity item in SQLite dialog_items to locked/answered state
            let answered_item = crate::db::ActivityItem {
                id: format!("q-{}", question_id),
                item_type: "question".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: Some(text.to_string()),
                tool: Some(serde_json::json!({
                    "id": question_id,
                    "toolName": "question",
                    "status": "success",
                    "selectedAnswer": &answer,
                    "params": {
                        "taskId": task_id_str,
                        "questionId": question_id,
                        "text": text,
                        "options": options,
                        "multiple": multiple,
                        "selectedAnswer": &answer,
                    }
                })),
                duration_ms: None,
                images: None,
            };
            let _ = crate::db::append_dialog_items(repo_root, &task_id_str, &[answered_item]);

            answer
        }

        "create_task" => {
            // Check if batch call (via create_tasks alias or tasks array in arguments)
            let is_batch = tool_name == "create_tasks" || args.get("tasks").and_then(|v| v.as_array()).is_some();

            struct TaskInput {
                title: String,
                description: String,
                column_id: Option<String>,
                board_id: Option<String>,
                tags: Option<Vec<String>>,
                priority: Option<String>,
                model: Option<String>,
                is_subtask: Option<bool>,
                parent_id: Option<String>,
            }

            let mut inputs = Vec::new();

            if let Some(tasks_arr) = args.get("tasks").and_then(|v| v.as_array()) {
                if tasks_arr.is_empty() {
                    bail!("'tasks' array must not be empty");
                }
                for (idx, item) in tasks_arr.iter().enumerate() {
                    let title = item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("Missing 'title' in tasks[{}]", idx))?
                        .trim()
                        .to_string();
                    if title.is_empty() {
                        bail!("'title' in tasks[{}] cannot be empty", idx);
                    }
                    let description = item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let column_id = item
                        .get("column_id")
                        .or_else(|| item.get("columnId"))
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                    let board_id = item
                        .get("board_id")
                        .or_else(|| item.get("boardId"))
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                    let tags = item.get("tags").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter().filter_map(|t| t.as_str().map(str::to_string)).collect::<Vec<_>>()
                    });
                    let priority = item
                        .get("priority")
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                    let model = item
                        .get("model")
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                    let is_subtask = item
                        .get("is_subtask")
                        .or_else(|| item.get("isSubtask"))
                        .and_then(|v| v.as_bool());
                    let parent_id = item
                        .get("parent_id")
                        .or_else(|| item.get("parentId"))
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);

                    inputs.push(TaskInput {
                        title,
                        description,
                        column_id,
                        board_id,
                        tags,
                        priority,
                        model,
                        is_subtask,
                        parent_id,
                    });
                }
            } else {
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .context("Missing required parameter 'title'")?
                    .trim()
                    .to_string();
                if title.is_empty() {
                    bail!("Parameter 'title' cannot be empty");
                }
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let column_id = args
                    .get("column_id")
                    .or_else(|| args.get("columnId"))
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                let board_id = args
                    .get("board_id")
                    .or_else(|| args.get("boardId"))
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter().filter_map(|t| t.as_str().map(str::to_string)).collect::<Vec<_>>()
                });
                let priority = args
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                let model = args
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                let is_subtask = args
                    .get("is_subtask")
                    .or_else(|| args.get("isSubtask"))
                    .and_then(|v| v.as_bool());
                let parent_id = args
                    .get("parent_id")
                    .or_else(|| args.get("parentId"))
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);

                inputs.push(TaskInput {
                    title,
                    description,
                    column_id,
                    board_id,
                    tags,
                    priority,
                    model,
                    is_subtask,
                    parent_id,
                });
            }

            let mut tasks = crate::commands::load_tasks_from_disk(repo_root).await;
            let current_task_opt = task_id.and_then(|tid| tasks.iter().find(|t| t.id == tid).cloned());
            let boards_cfg = crate::config::load_boards_config(repo_root).await.ok();

            let mut created_results = Vec::new();
            let mut autorun_task_ids = Vec::new();
            let now = chrono::Utc::now().timestamp_millis();
            let mut next_num = crate::commands::tasks::allocate_next_task_num(repo_root, &tasks).await;

            let pipe_cols = crate::config::load_pipeline_config(repo_root).await.ok().map(|p| p.columns);

            for input in inputs {
                while tasks.iter().any(|t| t.id == format!("CVH-{:03}", next_num)) {
                    next_num = next_num.saturating_add(1);
                }
                let new_id = format!("CVH-{:03}", next_num);
                let _ = crate::commands::tasks::save_task_sequence(repo_root, next_num).await;
                next_num = next_num.saturating_add(1);

                // Resolve target board (defaults to current task's board)
                let effective_board_id = if let Some(ref bid) = input.board_id {
                    bid.clone()
                } else if let Some(ref ct) = current_task_opt {
                    ct.board_id.clone().unwrap_or_else(|| {
                        boards_cfg.as_ref().map(|b| b.active_board_id.clone()).unwrap_or_else(|| "development".to_string())
                    })
                } else {
                    boards_cfg.as_ref().map(|b| b.active_board_id.clone()).unwrap_or_else(|| "development".to_string())
                };

                let target_board_cols = boards_cfg.as_ref().and_then(|bc| {
                    bc.boards.iter().find(|b| b.id == effective_board_id).map(|b| b.columns.clone())
                });

                // Resolve column (defaults to "backlog" or target board's backlog column)
                let effective_column_id = if let Some(ref cid) = input.column_id {
                    cid.clone()
                } else if let Some(ref cols) = target_board_cols {
                    if let Some(backlog_col) = cols.iter().find(|c| crate::commands::tasks::stage_id_matches(&c.id, "backlog")) {
                        backlog_col.id.clone()
                    } else if let Some(first) = cols.first() {
                        first.id.clone()
                    } else {
                        "backlog".to_string()
                    }
                } else {
                    "backlog".to_string()
                };

                let is_subtask = input.is_subtask.unwrap_or(false);
                let parent_id = if is_subtask {
                    input.parent_id.or_else(|| task_id.map(str::to_string))
                } else {
                    input.parent_id
                };

                let target_col = target_board_cols.as_ref().and_then(|cols| {
                    cols.iter().find(|c| c.id == effective_column_id)
                }).or_else(|| {
                    pipe_cols.as_ref().and_then(|cols| cols.iter().find(|c| c.id == effective_column_id))
                });

                let is_backlog = crate::commands::tasks::stage_id_matches(&effective_column_id, "backlog");
                let should_autorun = !is_backlog && target_col.map(|c| {
                    c.auto_run == Some(true) || c.persona.is_some()
                }).unwrap_or(false);

                if should_autorun {
                    autorun_task_ids.push(new_id.clone());
                }

                let new_task = Task {
                    id: new_id.clone(),
                    title: input.title.clone(),
                    description: input.description.clone(),
                    stage_id: effective_column_id.clone(),
                    board_id: Some(effective_board_id.clone()),
                    worktree_path: None,
                    branch: None,
                    verdict: None,
                    custom_flow: None,
                    status: Some("idle".to_string()),
                    model: input.model.or_else(|| current_task_opt.as_ref().and_then(|ct| ct.model.clone())),
                    parent_id,
                    subtask_index: None,
                    waiting_for_subtask_id: None,
                    handover_summary: None,
                    cover_image: None,
                    attachments: None,
                    todos: None,
                    priority: input.priority,
                    tags: input.tags,
                    current_tokens: None,
                    skip_worktree: None,
                    created_at: now,
                    updated_at: now,
                    completed_at: None,
                    position: None,
                    hook_states: None,
                };

                tasks.push(new_task);

                created_results.push(serde_json::json!({
                    "task_id": new_id,
                    "title": input.title,
                    "column_id": effective_column_id,
                    "board_id": effective_board_id,
                    "status": "created"
                }));
            }

            // Sync in-memory AppState
            if let Some(app) = ctx.app_handle.as_ref() {
                if let Some(state_ref) = app.try_state::<crate::commands::state::AppState>() {
                    if let Ok(mut guard) = state_ref.tasks.lock() {
                        *guard = tasks.clone();
                    }
                }
            }

            // Persist to disk
            crate::commands::tasks::save_tasks_to_disk(repo_root, &tasks)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            // Emit tasks:updated via Tauri AppHandle
            if let Some(app) = ctx.app_handle.as_ref() {
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                for auto_tid in autorun_task_ids {
                    let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": &auto_tid }));
                }
            }

            // Record dialog activity in SQLite
            if let Some(parent_tid) = task_id {
                let act_content = if created_results.len() == 1 {
                    format!(
                        "### Оркестратор создал задачу\n- **ID**: {}\n- **Название**: {}\n- **Колонка**: {}\n- **Доска**: {}",
                        created_results[0]["task_id"].as_str().unwrap_or_default(),
                        created_results[0]["title"].as_str().unwrap_or_default(),
                        created_results[0]["column_id"].as_str().unwrap_or_default(),
                        created_results[0]["board_id"].as_str().unwrap_or_default()
                    )
                } else {
                    let mut msg = format!("### Оркестратор пакетно создал {} задач:\n", created_results.len());
                    for item in &created_results {
                        msg.push_str(&format!(
                            "- **{}**: {} ({})\n",
                            item["task_id"].as_str().unwrap_or_default(),
                            item["title"].as_str().unwrap_or_default(),
                            item["column_id"].as_str().unwrap_or_default()
                        ));
                    }
                    msg
                };
                let _ = crate::commands::dialogs::append_task_dialog_activity(
                    repo_root,
                    parent_tid,
                    "orchestration_create",
                    &act_content,
                    ctx.app_handle.as_ref(),
                ).await;
            }

            if !is_batch && created_results.len() == 1 {
                serde_json::to_string_pretty(&created_results[0])?
            } else {
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "created",
                    "count": created_results.len(),
                    "tasks": created_results
                }))?
            }
        }

        "complete_task" => {
            let summary = args
                .get("summary")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'summary'")?;

            let current_task_id = args
                .get("task_id")
                .or_else(|| args.get("taskId"))
                .and_then(|v| v.as_str())
                .or(task_id)
                .context("No active task ID for complete_task")?;

            let mut tasks = crate::commands::load_tasks_from_disk(repo_root).await;
            let current_task_idx = tasks
                .iter()
                .position(|t| crate::commands::task_id_matches(&t.id, current_task_id))
                .ok_or_else(|| anyhow::anyhow!("Task {} not found", current_task_id))?;

            let current_task = &tasks[current_task_idx];

            let boards_cfg = crate::config::load_boards_config(repo_root).await.ok();
            let board_cols = boards_cfg.as_ref().and_then(|bc| {
                let b = if let Some(ref bid) = current_task.board_id {
                    bc.boards.iter().find(|b| &b.id == bid)
                } else {
                    bc.boards.iter().find(|b| b.id == bc.active_board_id)
                };
                b.map(|b| b.columns.clone())
            });

            // Find terminal column (ready / done)
            let columns = match board_cols {
                Some(cols) => cols,
                None => {
                    let pipeline = crate::config::load_pipeline_config(repo_root).await
                        .context("Failed to load pipeline configuration")?;
                    pipeline.columns
                }
            };

            let terminal_col = columns
                .iter()
                .find(|c| {
                    c.terminal == Some(true)
                        || crate::commands::tasks::stage_id_matches(&c.id, "ready")
                        || crate::commands::tasks::stage_id_matches(&c.id, "done")
                })
                .or_else(|| columns.last())
                .ok_or_else(|| anyhow::anyhow!("No terminal column found in board/pipeline"))?;
            let terminal_col_id = terminal_col.id.clone();
            let old_stage = tasks[current_task_idx].stage_id.clone();

            tasks[current_task_idx].stage_id = terminal_col_id.clone();
            tasks[current_task_idx].status = Some("done".to_string());
            tasks[current_task_idx].verdict = Some("APPROVED".to_string());
            tasks[current_task_idx].handover_summary = Some(summary.to_string());
            tasks[current_task_idx].updated_at = chrono::Utc::now().timestamp_millis();
            let updated_task = tasks[current_task_idx].clone();

            // In-memory state sync
            if let Some(app) = ctx.app_handle.as_ref() {
                if let Some(state_ref) = app.try_state::<crate::commands::state::AppState>() {
                    if let Ok(mut guard) = state_ref.tasks.lock() {
                        *guard = tasks.clone();
                    }
                }
            }

            // Persist to disk
            crate::commands::tasks::save_tasks_to_disk(repo_root, &tasks)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            // Emit events
            if let Some(app) = ctx.app_handle.as_ref() {
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                let _ = app.emit(
                    "task:moved",
                    serde_json::json!({
                        "taskId": current_task_id,
                        "fromStage": old_stage,
                        "toStage": &terminal_col_id,
                        "stageId": &terminal_col_id,
                        "status": "done",
                        "verdict": "APPROVED",
                        "task": &updated_task
                    }),
                );
            }

            // Append dialog activity in SQLite
            let _ = crate::commands::dialogs::append_task_dialog_activity(
                repo_root,
                current_task_id,
                "orchestration_complete",
                &format!("### Оркестратор завершил задачу\n\n**Итоговое саммари декомпозиции:**\n{}", summary),
                ctx.app_handle.as_ref(),
            ).await;

            serde_json::to_string_pretty(&serde_json::json!({
                "task_id": current_task_id,
                "title": updated_task.title,
                "column_id": terminal_col_id,
                "status": "done",
                "handover_summary": summary
            }))?
        }

        "list_tasks" => {
            let mut tasks = crate::db::db_get_all_tasks(repo_root)
                .map_err(|e| anyhow::anyhow!("Failed to read tasks from database: {}", e))?;

            if let Some(target_stage) = args.get("stage_id").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
                let target = target_stage.trim();
                tasks.retain(|t| t.stage_id.eq_ignore_ascii_case(target));
            }

            if let Some(q) = args.get("query").and_then(|v| v.as_str()).map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()) {
                tasks.retain(|t| {
                    t.id.to_lowercase().contains(&q)
                        || t.title.to_lowercase().contains(&q)
                        || t.handover_summary.as_ref().map(|h| h.to_lowercase().contains(&q)).unwrap_or(false)
                });
            }

            // Sort by updated_at / created_at (fresh first)
            tasks.sort_by(|a, b| {
                let time_b = if b.updated_at > 0 { b.updated_at } else { b.created_at };
                let time_a = if a.updated_at > 0 { a.updated_at } else { a.created_at };
                time_b.cmp(&time_a)
            });

            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| (v as usize).clamp(1, 50))
                .unwrap_or(20);

            tasks.truncate(limit);

            let personas = get_stage_personas(repo_root).await;

            #[derive(Serialize)]
            struct CompactTaskItem {
                id: String,
                title: String,
                stage_id: String,
                status: String,
                role: String,
                priority: String,
                handover_summary: Option<String>,
                updated_at: i64,
            }

            let compact_list: Vec<CompactTaskItem> = tasks
                .into_iter()
                .map(|t| {
                    let role = resolve_task_role(&t.stage_id, &personas);
                    let status = t.status.unwrap_or_else(|| "idle".to_string());
                    let priority = t.priority.unwrap_or_else(|| "medium".to_string());
                    let updated_at = if t.updated_at > 0 { t.updated_at } else { t.created_at };
                    CompactTaskItem {
                        id: t.id,
                        title: t.title,
                        stage_id: t.stage_id,
                        status,
                        role,
                        priority,
                        handover_summary: t.handover_summary,
                        updated_at,
                    }
                })
                .collect();

            serde_json::to_string_pretty(&compact_list)?
        }

        "get_task_history" => {
            let target_task_id = args
                .get("task_id")
                .or_else(|| args.get("taskId"))
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'task_id'")?;
            let include_tool_outputs = args
                .get("include_tool_outputs")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let maybe_task = crate::db::db_get_task(repo_root, target_task_id)
                .map_err(|e| anyhow::anyhow!("Database error loading task '{}': {}", target_task_id, e))?;

            let task = maybe_task.ok_or_else(|| anyhow::anyhow!("Task '{}' not found", target_task_id))?;
            let items = crate::db::load_dialog_items(repo_root, &task.id)
                .unwrap_or_default();

            let personas = get_stage_personas(repo_root).await;
            let role = resolve_task_role(&task.stage_id, &personas);
            let status = task.status.as_deref().unwrap_or("idle");
            let mut report = String::new();
            report.push_str(&format!(
                "# Task History: {} - {} (Этап: {}, Статус: {}, Роль: {})\n\n",
                task.id, task.title, task.stage_id, status, role
            ));

            report.push_str("## Initial Objective:\n");
            let desc = task.description.trim();
            if desc.is_empty() {
                report.push_str("(No description provided)\n\n");
            } else {
                report.push_str(desc);
                report.push_str("\n\n");
            }

            report.push_str("## Execution Steps:\n");
            let mut step_num = 1;
            let mut last_terminal_text: Option<String> = None;

            for it in &items {
                match it.item_type.as_str() {
                    "steer" | "system" | "prompt" => {
                        let content = it.content.as_deref().unwrap_or("").trim();
                        if !content.is_empty() {
                            report.push_str(&format!("- Шаг {} [User Steer / System]: {}\n", step_num, content));
                            step_num += 1;
                        }
                    }
                    "tool" => {
                        let (tool_name, params_str, status_str, duration, error_snippet, output_snippet) = if let Some(ref t) = it.tool {
                            let name = t.get("toolName").or_else(|| t.get("name")).and_then(|v| v.as_str()).unwrap_or("unknown");
                            let params = t.get("params").or_else(|| t.get("arguments")).cloned().unwrap_or_else(|| serde_json::json!({}));
                            let params_str = serde_json::to_string(&params).unwrap_or_else(|_| "{}".to_string());
                            let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("success");
                            let duration = t.get("durationMs").and_then(|v| v.as_f64()).or(it.duration_ms).unwrap_or(0.0);
                            let err = t.get("stderr").or_else(|| t.get("error")).and_then(|v| v.as_str())
                                .or_else(|| if status == "failed" { it.content.as_deref() } else { None })
                                .unwrap_or("none");
                            let out = t.get("stdout").or_else(|| t.get("result")).and_then(|v| v.as_str())
                                .or(it.content.as_deref());
                            (name, params_str, status, duration, err, out)
                        } else {
                            ("unknown", "{}".to_string(), "unknown", it.duration_ms.unwrap_or(0.0), "none", it.content.as_deref())
                        };

                        report.push_str(&format!(
                            "- Шаг {} [Tool {}]: params: {}, status: {}, duration: {:.0}ms, error: {}.\n",
                            step_num, tool_name, params_str, status_str, duration, error_snippet
                        ));

                        if include_tool_outputs {
                            if let Some(out) = output_snippet {
                                let trimmed = out.trim();
                                if !trimmed.is_empty() {
                                    let compacted = if trimmed.chars().count() > 300 {
                                        format!("{}... [compacted]", trimmed.chars().take(300).collect::<String>())
                                    } else {
                                        trimmed.to_string()
                                    };
                                    report.push_str(&format!("  Output: {}\n", compacted));
                                }
                            }
                        }
                        step_num += 1;
                        if let Some(ref c) = it.content {
                            if !c.trim().is_empty() {
                                last_terminal_text = Some(c.trim().to_string());
                            }
                        }
                    }
                    "output" | "assistant" | "response" => {
                        if let Some(ref c) = it.content {
                            if !c.trim().is_empty() {
                                last_terminal_text = Some(c.trim().to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }

            if step_num == 1 {
                report.push_str("(No execution steps recorded)\n");
            }
            report.push('\n');

            report.push_str("## Handover & Terminal Response:\n");
            let handover = task.handover_summary.as_deref().filter(|s| !s.trim().is_empty()).unwrap_or("None");
            report.push_str(&format!("- Handover Summary: {}\n", handover));

            let verdict_text = match (task.verdict.as_deref(), last_terminal_text.as_deref()) {
                (Some(v), Some(out)) => format!("[{}] {}", v, out),
                (Some(v), None) => format!("[{}]", v),
                (None, Some(out)) => out.to_string(),
                (None, None) => "None".to_string(),
            };
            report.push_str(&format!("- Final Output / Verdict: {}\n", verdict_text));

            report
        }

        "load_rule" => {
            let rule_raw = args
                .get("rule")
                .or_else(|| args.get("name"))
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'rule'")?;

            if rule_raw.contains("..") || rule_raw.contains(':') || rule_raw.starts_with('/') || rule_raw.starts_with('\\') {
                bail!("Invalid rule name '{}': path traversal characters are forbidden", rule_raw);
            }

            let rule_clean = rule_raw.strip_suffix(".md").unwrap_or(rule_raw);
            let role = ctx.role.unwrap_or("general");

            // Look for rule in prioritized order:
            // 1) repo_root/.cvh/agents/<role>/rules/<rule>.md
            // 2) repo_root/.cvh/rules/<rule>.md
            // 3) repo_root/.cvh/agents/<role>/rules/<rule>
            let mut candidates = vec![
                repo_root.join(".cvh").join("agents").join(role).join("rules").join(format!("{}.md", rule_clean)),
                repo_root.join(".cvh").join("rules").join(format!("{}.md", rule_clean)),
                repo_root.join(".cvh").join("agents").join(role).join("rules").join(rule_raw),
            ];

            // If running in an isolated worktree or test dir:
            if worktree_dir != repo_root {
                candidates.push(worktree_dir.join(".cvh").join("agents").join(role).join("rules").join(format!("{}.md", rule_clean)));
                candidates.push(worktree_dir.join(".cvh").join("rules").join(format!("{}.md", rule_clean)));
                candidates.push(worktree_dir.join(".cvh").join("agents").join(role).join("rules").join(rule_raw));
            }

            // Fallback: search across any agents' rules directory
            let agents_base = repo_root.join(".cvh").join("agents");
            if agents_base.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&agents_base) {
                    for entry in entries.flatten() {
                        let path = entry.path().join("rules").join(format!("{}.md", rule_clean));
                        if !candidates.contains(&path) {
                            candidates.push(path);
                        }
                    }
                }
            }

            let mut content = None;
            for cand in candidates {
                if cand.is_file() {
                    match tokio::fs::read_to_string(&cand).await {
                        Ok(text) => {
                            content = Some(text);
                            break;
                        }
                        Err(e) => {
                            bail!("Failed to read rule file at {:?}: {}", cand, e);
                        }
                    }
                }
            }

            if let Some(text) = content {
                text
            } else {
                bail!(
                    "Rule '{}' not found in .cvh/agents/{}/rules or .cvh/rules",
                    rule_raw,
                    role
                )
            }
        }

        custom_tool => {
            let custom_tools = load_custom_tools(repo_root).await;
            if let Some(ct) = custom_tools.into_iter().find(|t| t.name == custom_tool) {
                let cmd_str = ct.command.replace("{{args}}", &args.to_string());
                let res = run_scoped_command(worktree_dir, &cmd_str, Duration::from_secs(60), Arc::clone(&abort_handle)).await?;
                format!(
                    "[Custom Tool: {} Exit: {}]\nSTDOUT: {}\nSTDERR: {}",
                    custom_tool,
                    res.exit_code.unwrap_or(-1),
                    res.stdout.trim(),
                    res.stderr.trim()
                )
            } else {
                bail!("Unknown tool: {}", custom_tool);
            }
        }
    };

    if abort_handle.load(Ordering::SeqCst) {
        bail!("Operation aborted");
    }

    maybe_spill_output(repo_root, normalized_tool_name, raw_result).await
}

async fn emit_file_change_events(ctx: &ToolContext<'_>, rel_path: &str) {
    if let Some(app) = &ctx.app_handle {
        let norm = rel_path.replace('\\', "/");
        if norm.ends_with("boards.json") || norm.ends_with("pipeline.json") {
            if let Ok(bc) = crate::config::load_boards_config(ctx.repo_root).await {
                let _ = app.emit(
                    "boards:updated",
                    serde_json::json!({ "boards": &bc.boards, "activeBoardId": &bc.active_board_id }),
                );
                let mut tasks = crate::commands::tasks::load_tasks_from_disk(ctx.repo_root).await;
                let modified = crate::commands::tasks::heal_tasks(&mut tasks, &bc);
                if modified {
                    let _ = crate::commands::tasks::save_tasks_to_disk(ctx.repo_root, &tasks).await;
                }
                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
            }
            if let Ok(pipe) = crate::config::load_pipeline_config(ctx.repo_root).await {
                let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
            }
        } else if norm.contains(".cvh/agents") || norm.contains("/agents/") {
            let list = crate::commands::list_agent_personas_in(ctx.repo_root).await;
            let _ = app.emit("agent:personas_updated", &list);
        }
    }
}

/// In-Process Subagent Execution Engine (§ 3 Architectural Invariants)
/// ZERO WORKTREES, ZERO HOOKS: Runs strictly in-process inside parent's active worktree_dir.
/// ZERO BOARD MOVEMENT: Subagents do not modify cards on the kanban board or move stages.
pub fn run_inprocess_subagent(
    ctx: OwnedToolContext,
    title: String,
    role: String,
    prompt: String,
    requested_tools: Option<Vec<String>>,
    explicit_subagent_id: Option<String>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> {
    Box::pin(async move {
    let start_instant = std::time::Instant::now();
    let parent_id = ctx.task_id.as_deref().unwrap_or("unknown").to_string();
    let subagent_suffix = match ctx.call_id.as_deref() {
        Some(cid) if !cid.trim().is_empty() => cid.trim().to_string(),
        _ => format!(
            "{}-{}",
            chrono::Utc::now().timestamp_millis(),
            uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>()
        ),
    };
    let is_bg_subagent = explicit_subagent_id.is_some();
    let raw_sub_id = explicit_subagent_id.unwrap_or_else(|| format!("{}.sub-{}", parent_id, subagent_suffix));
    let (subagent_id, subagent_dialog_id) = if raw_sub_id.contains(":sub:") {
        let after = raw_sub_id.rsplit(":sub:").next().unwrap_or(&raw_sub_id).to_string();
        (after, raw_sub_id)
    } else {
        let dialog_id = format!("{}:sub:{}", parent_id, raw_sub_id);
        (raw_sub_id, dialog_id)
    };
    let tool_call_id = ctx
        .call_id
        .clone()
        .or_else(|| subagent_id.rfind(".sub-").map(|pos| subagent_id[(pos + 5)..].to_string()));

    // 1. Tool filtering (Check against subagent role via tool policy and subagent setting)
    let subagent_setting = load_agent_setting(&ctx.repo_root, &role).await;
    let all_tools = get_all_tools_for_role(&ctx.repo_root, Some(&role)).await;
    let base_tools: Vec<String> = match requested_tools {
        Some(t) if !t.is_empty() => t,
        _ => all_tools.iter().map(|t| t.function.name.clone()).collect(),
    };

    let mut effective_sub_tools: Vec<String> = base_tools
        .into_iter()
        .filter(|t| {
            t != "subagent"
                && t != "subagent_bg"
                && t != "collect_subagent"
                && t != "subagent_list"
                && t != "advance_task"
                && t != "move_task"
                && t != "move_task_stage"
                && t != "advance_stage"
                && t != "next_stage"
                && t != "reject_task"
                && t != "create_task"
                && t != "create_tasks"
                && t != "complete_task"
                && is_tool_active_for_agent(
                    t,
                    &ctx.tool_policy,
                    &role,
                    None,
                    subagent_setting.allowed_tools.as_deref(),
                )
        })
        .collect();
    effective_sub_tools.sort();
    effective_sub_tools.dedup();

    // 2. Emit subagent:spawned
    if let Some(ref app) = ctx.app_handle {
        let _ = app.emit(
            "subagent:spawned",
            serde_json::json!({
                "parentTaskId": &parent_id,
                "subagentId": &subagent_id,
                "toolCallId": &tool_call_id,
                "title": &title,
                "role": &role,
                "prompt": &prompt,
                "tools": &effective_sub_tools,
            }),
        );
    }

    // 3. Persist top instruction prompt into SQLite dialog
    let prompt_item = ActivityItem {
        id: format!("sub-p-{}", subagent_suffix),
        item_type: "prompt".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        content: Some(prompt.to_string()),
        tool: Some(serde_json::json!({
            "title": &title,
            "role": &role,
            "is_bg": is_bg_subagent
        })),
        duration_ms: None,
        images: None,
    };
    let _ = crate::db::append_dialog_items(
        &ctx.repo_root,
        &subagent_dialog_id,
        std::slice::from_ref(&prompt_item),
    );

    // 4. Resolve schemas for allowed subagent tools
    let all_tools = get_all_tools_for_role(&ctx.repo_root, Some(&role)).await;
    let mut active_sub_tool_defs: Vec<ToolDefinition> = all_tools
        .into_iter()
        .filter(|t| effective_sub_tools.contains(&t.function.name))
        .collect();
    active_sub_tool_defs.sort_by(|a, b| a.function.name.cmp(&b.function.name));

    // 5. System persona & initial messages:
    // Prompt Cache Prefix Stabilization (§ 2.10): Keep SystemMessage completely static
    // (role persona + invariants + static tool list) with zero dynamic worktree dir, task title,
    // or timestamp. Dynamic metadata is placed in initial_user_prompt.
    let persona_content = {
        let persona_path = crate::config::agent_persona_path(&ctx.repo_root, &role);
        if persona_path.is_file() {
            tokio::fs::read_to_string(&persona_path).await.ok()
        } else {
            None
        }
    };

    let system_prompt = format!(
        "# Role Persona: {}
{}

# Scoped Subagent Mandate
You are an in-process subagent executing an atomic subtask.
Allowed Tools: {:?}
Invariants:
- Do not attempt to move kanban stages or spawn nested subagents.
- Single-Action Collection: When invoking 'collect_subagent', you MUST invoke it as the ONLY tool call in that turn. Do NOT batch file reads or other tools alongside collect_subagent; wait to consume and analyze the received reports before planning next steps.
- Execute the task thoroughly and verify results.
- When done, output your comprehensive final report.",
        role,
        persona_content.as_deref().unwrap_or("You are a specialized autonomous subagent."),
        effective_sub_tools
    );

    let initial_user_prompt = format!(
        "# Context
Working directory: {}
Task Title: {}

# Objective: {}

{}",
        ctx.worktree_dir.to_string_lossy(),
        title,
        title,
        prompt
    );

    let system_tokens = compactor::estimate_text_tokens(&system_prompt) as u64;
    let mut sub_messages = vec![
        ChatMessage::system(system_prompt).with_cache_control(crate::llm::CacheControl::ephemeral()),
        ChatMessage::user(initial_user_prompt).with_cache_control(crate::llm::CacheControl::ephemeral()),
    ];

    let base_url = ctx.base_url.as_deref().unwrap_or("http://127.0.0.1:8045/v1");
    let resolved_model = match ctx
        .model
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
    {
        Some(m) => m.to_string(),
        None => {
            let global_settings = crate::config::load_llm_settings(&ctx.repo_root)
                .await
                .unwrap_or_default();
            let m = global_settings.model.trim();
            if !m.is_empty() {
                m.to_string()
            } else {
                "gemini-3.8-flash-high".to_string()
            }
        }
    };
    let model = resolved_model.as_str();
    let api_key = ctx.api_key.as_deref();
    let temperature = ctx.temperature.or(Some(0.2));
    let reasoning_effort = ctx.reasoning_effort.clone();

    let llm_client = LlmClient::new();
    let mut sub_turn = 0;
    let mut final_sub_response = String::new();
    let mut sub_error: Option<String> = None;

    loop {
        sub_turn += 1;
        if let Some((_, session)) = find_subagent_session(Some(&parent_id), &subagent_id).await {
            let mut info = session.retry_info.lock().await;
            info.steps = sub_turn;
        }
        if ctx.abort_handle.load(Ordering::SeqCst) {
            break;
        }

        let req_tools = if active_sub_tool_defs.is_empty() {
            None
        } else {
            Some(active_sub_tool_defs.clone())
        };
        let tool_tokens = compactor::estimate_tools_tokens(&req_tools) as u64;

        let req = OpenAIChatRequest {
            model: model.to_string(),
            messages: sub_messages.clone(),
            temperature,
            stream: true,
            tools: req_tools.clone(),
            reasoning_effort: reasoning_effort.clone(),
            stream_options: Some(crate::llm::StreamOptions {
                include_usage: true,
            }),
        };

        // Log raw request for complete transparency & debugging (§ llm_debug.log)
        {
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            let mut req_log = format!(
                "[{}] [{}] Subagent Turn {} Request:
Model: {}
Messages count: {}
",
                ts, subagent_id, sub_turn, model, sub_messages.len()
            );
            if let Some(first) = sub_messages.first() {
                if first.role == Role::System {
                    req_log.push_str(&format!("System prompt:
{}
", first.content.as_deref().unwrap_or("")));
                }
            }
            req_log.push_str(&format!("Tool calls requested ({}): {:?}

", effective_sub_tools.len(), effective_sub_tools));
            append_llm_debug_log(&ctx.repo_root, &req_log).await;
        }

        let (tx, mut rx) = mpsc::channel::<StreamChunk>(100);
        let cl = llm_client.clone();
        let b_url = base_url.to_string();
        let a_key = api_key.map(|s| s.to_string());

        tokio::spawn(async move {
            if let Err(e) = cl.stream_openai(&b_url, a_key.as_deref(), req, tx.clone()).await {
                let _ = tx.send(StreamChunk::Error(format!("Subagent stream failed: {}", e))).await;
            }
        });

        let mut turn_content = String::new();
        let mut turn_thoughts = String::new();
        let turn_thought_id = format!("sub-th-{}-turn-{}", subagent_suffix, sub_turn);
        let mut thought_start_instant: Option<std::time::Instant> = None;
        let mut thought_end_instant: Option<std::time::Instant> = None;
        let mut turn_tool_calls: Vec<ToolCall> = Vec::new();
        let mut thinking_parser = ThinkingStreamParser::new();
        let turn_start_instant = std::time::Instant::now();
        let mut first_token_instant: Option<std::time::Instant> = None;
        let mut last_token_instant: Option<std::time::Instant> = None;
        let mut turn_streamed_chars: usize = 0;
        let mut received_usage = false;

        while let Some(chunk) = rx.recv().await {
            if ctx.abort_handle.load(Ordering::SeqCst) {
                break;
            }
            match chunk {
                StreamChunk::FirstChunk => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                }
                StreamChunk::Token(token) => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += token.len();

                    let parsed = thinking_parser.push(&token);
                    for (is_thought, text) in parsed {
                        if is_thought {
                            if thought_start_instant.is_none() {
                                thought_start_instant = Some(std::time::Instant::now());
                            }
                            thought_end_instant = Some(std::time::Instant::now());
                            turn_thoughts.push_str(&text);

                            let th_item = ActivityItem {
                                id: turn_thought_id.clone(),
                                item_type: "thought".to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                content: Some(turn_thoughts.clone()),
                                tool: None,
                                duration_ms: None,
                                images: None,
                            };
                            let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&th_item));
                            if let Some(ref app) = ctx.app_handle {
                                let _ = app.emit("subagent:activity", serde_json::json!({
                                    "parentTaskId": parent_id,
                                    "subagentId": &subagent_id,
                                    "activity": &th_item,
                                }));
                            }
                        } else {
                            turn_content.push_str(&text);
                        }
                    }
                }
                StreamChunk::Output(output) => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += output.len();
                    turn_content.push_str(&output);
                }
                StreamChunk::Thought(thought) => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += thought.len();
                    if thought_start_instant.is_none() {
                        thought_start_instant = Some(std::time::Instant::now());
                    }
                    thought_end_instant = Some(std::time::Instant::now());
                    turn_thoughts.push_str(&thought);

                    let th_item = ActivityItem {
                        id: turn_thought_id.clone(),
                        item_type: "thought".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        content: Some(turn_thoughts.clone()),
                        tool: None,
                        duration_ms: None,
                        images: None,
                    };
                    let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&th_item));
                    if let Some(ref app) = ctx.app_handle {
                        let _ = app.emit("subagent:activity", serde_json::json!({
                            "parentTaskId": parent_id,
                            "subagentId": &subagent_id,
                            "activity": &th_item,
                        }));
                    }
                }
                StreamChunk::ToolCall { id, name, arguments } => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += name.len() + arguments.len();

                    turn_tool_calls.push(ToolCall {
                        id,
                        call_type: "function".to_string(),
                        function: crate::llm::FunctionCall { name, arguments },
                    });
                }
                StreamChunk::Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens,
                    cache_creation_tokens,
                } => {
                    received_usage = true;
                    let total_input = input_tokens + cache_read_tokens;
                    let cache_hit_rate = if total_input > 0 {
                        (cache_read_tokens as f32 / total_input as f32) * 100.0
                    } else {
                        0.0
                    };
                    let start_time = first_token_instant.unwrap_or(turn_start_instant);
                    let end_time = last_token_instant.unwrap_or_else(std::time::Instant::now);
                    let duration_secs = end_time.duration_since(start_time).as_secs_f64();
                    let duration_ms = duration_secs * 1000.0;
                    let ttft_ms = first_token_instant
                        .map(|t| t.duration_since(turn_start_instant).as_secs_f64() * 1000.0)
                        .unwrap_or(0.0);
                    let tps = if duration_secs > 0.05 && output_tokens > 0 {
                        (output_tokens as f64 / duration_secs) as f32
                    } else {
                        0.0
                    };
                    eprintln!(
                        "CVH [subagent telemetry] turn={} subagent={} input={} output={} cache_read={} hit_rate={:.1}% duration={:.2}s tps={:.1} ttft={:.1}ms",
                        sub_turn, subagent_id, input_tokens, output_tokens, cache_read_tokens, cache_hit_rate, duration_secs, tps, ttft_ms
                    );
                    if let Some(ref app) = ctx.app_handle {
                        let _ = app.emit(
                            "agent:telemetry",
                            serde_json::json!({
                                "taskId": &subagent_id,
                                "inputTokens": input_tokens,
                                "outputTokens": output_tokens,
                                "cacheReadTokens": cache_read_tokens,
                                "cacheCreationTokens": cache_creation_tokens,
                                "cacheHitRate": cache_hit_rate,
                                "durationMs": duration_ms,
                                "tps": tps,
                                "ttftMs": ttft_ms,
                                "systemTokens": system_tokens,
                                "toolTokens": tool_tokens,
                            }),
                        );
                        if let Some(ref cid) = tool_call_id {
                            if cid != &subagent_id {
                                let _ = app.emit(
                                    "agent:telemetry",
                                    serde_json::json!({
                                        "taskId": cid,
                                        "inputTokens": input_tokens,
                                        "outputTokens": output_tokens,
                                        "cacheReadTokens": cache_read_tokens,
                                        "cacheCreationTokens": cache_creation_tokens,
                                        "cacheHitRate": cache_hit_rate,
                                        "durationMs": duration_ms,
                                        "tps": tps,
                                        "ttftMs": ttft_ms,
                                        "systemTokens": system_tokens,
                                        "toolTokens": tool_tokens,
                                    }),
                                );
                            }
                        }
                    }
                    let telem_record = crate::db::TelemetryRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        task_id: subagent_id.clone(),
                        model: Some(model.to_string()),
                        input_tokens: input_tokens as i64,
                        output_tokens: output_tokens as i64,
                        cached_tokens: cache_read_tokens as i64,
                        tps: tps as f64,
                        ttft_ms,
                        duration_ms,
                        created_at: chrono::Utc::now().timestamp_millis(),
                    };
                    let _ = crate::db::db_record_telemetry(&ctx.repo_root, &telem_record);
                }
                StreamChunk::Done => break,
                StreamChunk::Error(e) => {
                    eprintln!("CVH [subagent]: error: {}", e);
                    sub_error = Some(e);
                    break;
                }
            }
        }

        // Flush thinking parser
        for (is_thought, text) in thinking_parser.flush() {
            if is_thought {
                if thought_start_instant.is_none() {
                    thought_start_instant = Some(std::time::Instant::now());
                }
                thought_end_instant = Some(std::time::Instant::now());
                turn_thoughts.push_str(&text);

                let th_item = ActivityItem {
                    id: turn_thought_id.clone(),
                    item_type: "thought".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    content: Some(turn_thoughts.clone()),
                    tool: None,
                    duration_ms: None,
                    images: None,
                };
                let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&th_item));
                if let Some(ref app) = ctx.app_handle {
                    let _ = app.emit("subagent:activity", serde_json::json!({
                        "parentTaskId": parent_id,
                        "subagentId": &subagent_id,
                        "activity": &th_item,
                    }));
                }
            } else {
                turn_content.push_str(&text);
            }
        }

        // Finalize duration for the turn thought card
        if !turn_thoughts.is_empty() {
            let thought_duration_ms = thought_start_instant
                .zip(thought_end_instant)
                .map(|(s, e)| e.duration_since(s).as_secs_f64() * 1000.0);
            let th_item = ActivityItem {
                id: turn_thought_id.clone(),
                item_type: "thought".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: Some(turn_thoughts.clone()),
                tool: None,
                duration_ms: thought_duration_ms,
                images: None,
            };
            let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&th_item));
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": parent_id,
                    "subagentId": &subagent_id,
                    "activity": &th_item,
                }));
            }
        }

        // Fallback telemetry if provider did not emit a usage chunk
        if !received_usage && !ctx.abort_handle.load(Ordering::SeqCst) {
            let start_time = first_token_instant.unwrap_or(turn_start_instant);
            let end_time = last_token_instant.unwrap_or_else(std::time::Instant::now);
            let duration_secs = end_time.duration_since(start_time).as_secs_f64();
            let duration_ms = duration_secs * 1000.0;
            let ttft_ms = first_token_instant
                .map(|t| t.duration_since(turn_start_instant).as_secs_f64() * 1000.0)
                .unwrap_or(0.0);
            let output_tokens = (compactor::estimate_text_tokens(&turn_content) as u64)
                .max((turn_streamed_chars as f64 / 3.8).ceil() as u64);
            let input_tokens = compactor::estimate_request_tokens(&sub_messages, req_tools.as_deref()) as u64;
            let tps = if duration_secs > 0.05 && output_tokens > 0 {
                (output_tokens as f64 / duration_secs) as f32
            } else {
                0.0
            };
            eprintln!(
                "CVH [subagent telemetry:fallback] turn={} subagent={} input={} output={} duration={:.2}s tps={:.1} ttft={:.1}ms",
                sub_turn, subagent_id, input_tokens, output_tokens, duration_secs, tps, ttft_ms
            );
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit(
                    "agent:telemetry",
                    serde_json::json!({
                        "taskId": &subagent_id,
                        "inputTokens": input_tokens,
                        "outputTokens": output_tokens,
                        "cacheReadTokens": 0,
                        "cacheCreationTokens": 0,
                        "cacheHitRate": 0.0,
                        "durationMs": duration_ms,
                        "tps": tps,
                        "ttftMs": ttft_ms,
                        "systemTokens": system_tokens,
                        "toolTokens": tool_tokens,
                    }),
                );
                if let Some(ref cid) = tool_call_id {
                    if cid != &subagent_id {
                        let _ = app.emit(
                            "agent:telemetry",
                            serde_json::json!({
                                "taskId": cid,
                                "inputTokens": input_tokens,
                                "outputTokens": output_tokens,
                                "cacheReadTokens": 0,
                                "cacheCreationTokens": 0,
                                "cacheHitRate": 0.0,
                                "durationMs": duration_ms,
                                "tps": tps,
                                "ttftMs": ttft_ms,
                                "systemTokens": system_tokens,
                                "toolTokens": tool_tokens,
                            }),
                        );
                    }
                }
            }
            let telem_record = crate::db::TelemetryRecord {
                id: uuid::Uuid::new_v4().to_string(),
                task_id: subagent_id.clone(),
                model: Some(model.to_string()),
                input_tokens: input_tokens as i64,
                output_tokens: output_tokens as i64,
                cached_tokens: 0,
                tps: tps as f64,
                ttft_ms,
                duration_ms,
                created_at: chrono::Utc::now().timestamp_millis(),
            };
            let _ = crate::db::db_record_telemetry(&ctx.repo_root, &telem_record);
        }

        // Log raw response for complete transparency & debugging (§ llm_debug.log)
        {
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            let mut resp_log = format!(
                "[{}] [{}] Subagent Turn {} Response:
",
                ts, subagent_id, sub_turn
            );
            if !turn_thoughts.trim().is_empty() {
                resp_log.push_str(&format!("Thoughts:
{}
", turn_thoughts.trim()));
            }
            if !turn_content.trim().is_empty() {
                resp_log.push_str(&format!("Content:
{}
", turn_content.trim()));
            }
            if !turn_tool_calls.is_empty() {
                resp_log.push_str(&format!("Tool calls received ({}):
", turn_tool_calls.len()));
                for tc in &turn_tool_calls {
                    resp_log.push_str(&format!("  - {} (id: {}): {}
", tc.function.name, tc.id, tc.function.arguments));
                }
            } else {
                resp_log.push_str("Tool calls received: None (terminal response)
");
            }
            resp_log.push_str("
--------------------------------------------------------------------------------

");
            append_llm_debug_log(&ctx.repo_root, &resp_log).await;
        }

        if turn_tool_calls.is_empty() {
            final_sub_response = turn_content.clone();
            if !final_sub_response.is_empty() {
                let out_item = ActivityItem {
                    id: format!("sub-out-{}", subagent_suffix),
                    item_type: "output".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    content: Some(final_sub_response.clone()),
                    tool: None,
                    duration_ms: None,
                    images: None,
                };
                let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&out_item));
                if let Some(ref app) = ctx.app_handle {
                    let _ = app.emit("subagent:activity", serde_json::json!({
                        "parentTaskId": parent_id,
                        "subagentId": &subagent_id,
                        "activity": &out_item,
                    }));
                }
            }
            break;
        }

        sub_messages.push(ChatMessage::assistant(
            if turn_content.is_empty() { None } else { Some(turn_content.clone()) },
            Some(turn_tool_calls.clone()),
        ));

        if !turn_content.is_empty() {
            let out_item = ActivityItem {
                id: format!("sub-msg-{}-{}", subagent_suffix, sub_turn),
                item_type: "output".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: Some(turn_content),
                tool: None,
                duration_ms: None,
                images: None,
            };
            let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&out_item));
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": parent_id,
                    "subagentId": &subagent_id,
                    "activity": &out_item,
                }));
            }
        }

        // Execute subagent tool calls
        for tc in turn_tool_calls {
            if ctx.abort_handle.load(Ordering::SeqCst) {
                break;
            }
            let parsed_args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| serde_json::json!({}));

            let tool_item_id = format!("sub-tool-{}", tc.id);
            let start_time = chrono::Utc::now().timestamp_millis();
            let tool_exec_running = serde_json::json!({
                "id": &tc.id,
                "toolName": &tc.function.name,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "durationMs": 0,
                "status": "running",
                "params": &parsed_args,
            });
            let tool_act_running = ActivityItem {
                id: tool_item_id.clone(),
                item_type: "tool".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: None,
                tool: Some(tool_exec_running),
                duration_ms: None,
                images: None,
            };
            let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&tool_act_running));
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": parent_id,
                    "subagentId": &subagent_id,
                    "activity": &tool_act_running,
                }));
            }

            let sub_tool_ctx = ToolContext {
                repo_root: &ctx.repo_root,
                worktree_dir: &ctx.worktree_dir,
                stage_id: None,
                role: Some(&role),
                abort_handle: Arc::clone(&ctx.abort_handle),
                tool_policy: &ctx.tool_policy,
                task_id: Some(&subagent_id),
                agent_allowed_tools: Some(&effective_sub_tools),
                app_handle: ctx.app_handle.clone(),
                base_url: ctx.base_url.as_deref(),
                api_key: ctx.api_key.as_deref(),
                model: Some(model),
                temperature: ctx.temperature,
                reasoning_effort: ctx.reasoning_effort.as_deref(),
                call_id: Some(&tc.id),
            };

            let exec_res = execute_tool(&sub_tool_ctx, &tc.function.name, &parsed_args).await;
            if ctx.abort_handle.load(Ordering::SeqCst) {
                let duration_ms = (chrono::Utc::now().timestamp_millis() - start_time) as f64;
                let tool_exec_aborted = serde_json::json!({
                    "id": &tc.id,
                    "toolName": &tc.function.name,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "durationMs": duration_ms,
                    "status": "failed",
                    "params": &parsed_args,
                    "stderr": "Прервано пользователем",
                    "stdout": "[Прервано пользователем / Aborted by user]",
                });
                let tool_act_aborted = ActivityItem {
                    id: tool_item_id,
                    item_type: "tool".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    content: Some("Прервано пользователем".to_string()),
                    tool: Some(tool_exec_aborted),
                    duration_ms: Some(duration_ms),
                    images: None,
                };
                let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&tool_act_aborted));
                if let Some(ref app) = ctx.app_handle {
                    let _ = app.emit("subagent:activity", serde_json::json!({
                        "parentTaskId": &parent_id,
                        "subagentId": &subagent_id,
                        "activity": &tool_act_aborted,
                    }));
                }
                break;
            }
            let duration_ms = (chrono::Utc::now().timestamp_millis() - start_time) as f64;

            let (res_str, is_success) = match exec_res {
                Ok(s) => (s, true),
                Err(e) => (format!("Error executing {}: {:#}", tc.function.name, e), false),
            };

            let tool_exec_done = serde_json::json!({
                "id": &tc.id,
                "toolName": &tc.function.name,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "durationMs": duration_ms,
                "status": if is_success { "success" } else { "failed" },
                "params": &parsed_args,
                "stdout": &res_str,
            });
            let tool_act_done = ActivityItem {
                id: tool_item_id,
                item_type: "tool".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: None,
                tool: Some(tool_exec_done),
                duration_ms: Some(duration_ms),
                images: None,
            };
            let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&tool_act_done));
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": &parent_id,
                    "subagentId": &subagent_id,
                    "activity": &tool_act_done,
                }));
            }

            sub_messages.push(ChatMessage::tool_response(tc.id, res_str));
        }
    }

    // Zero-Pollution of Parent Context on Subagent Failure (§ Directive 1):
    // If the subagent encountered an error, record error, emit subagent:error,
    // and keep parent channel pending so parent context is never polluted!
    if let Some(err_msg) = sub_error {
        let duration_ms = start_instant.elapsed().as_millis() as f64;
        let err_item = ActivityItem {
            id: format!("sub-err-{}", uuid::Uuid::new_v4()),
            item_type: "error".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            content: Some(err_msg.clone()),
            tool: None,
            duration_ms: Some(duration_ms),
            images: None,
        };
        let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&err_item));
        if let Some(ref app) = ctx.app_handle {
            let _ = app.emit("subagent:error", serde_json::json!({
                "parentTaskId": &parent_id,
                "subagentId": &subagent_id,
                "error": &err_msg,
                "durationMs": duration_ms,
            }));
        }

        if let Some((_, session)) = find_subagent_session(Some(&parent_id), &subagent_id).await {
            let mut info = session.retry_info.lock().await;
            info.status = "error".to_string();
            info.last_error = Some(err_msg.clone());
            info.duration_ms = Some(duration_ms);
            if let Some(sender) = session.sender.lock().await.take() {
                let _ = sender.send(format!("[Subagent error: {}]", err_msg));
            }
        }

        bail!("Subagent [{}] error: {}", subagent_id, err_msg);
    }

    // If the subagent completed its steps without emitting a final response, perform a synthesis turn
    if sub_error.is_none() && final_sub_response.trim().is_empty() && !ctx.abort_handle.load(Ordering::SeqCst) {
        sub_messages.push(ChatMessage::user(
            "Предоставь краткий и исчерпывающий финальный отчет о результатах выполненной работы и найденных данных (Provide a concise, comprehensive final report summarizing your findings and work done)."
        ));

        let synth_system_tokens = system_tokens;
        let req = OpenAIChatRequest {
            model: model.to_string(),
            messages: sub_messages.clone(),
            temperature,
            stream: true,
            tools: None,
            reasoning_effort: reasoning_effort.clone(),
            stream_options: Some(crate::llm::StreamOptions {
                include_usage: true,
            }),
        };

        let (tx, mut rx) = mpsc::channel::<StreamChunk>(100);
        let cl = llm_client.clone();
        let b_url = base_url.to_string();
        let a_key = api_key.map(|s| s.to_string());

        tokio::spawn(async move {
            if let Err(e) = cl.stream_openai(&b_url, a_key.as_deref(), req, tx.clone()).await {
                let _ = tx.send(StreamChunk::Error(format!("Subagent synthesis stream failed: {}", e))).await;
            }
        });

        let mut synth_content = String::new();
        let mut thinking_parser = ThinkingStreamParser::new();
        let synth_start_instant = std::time::Instant::now();
        let mut synth_first_token_instant: Option<std::time::Instant> = None;
        let mut synth_last_token_instant: Option<std::time::Instant> = None;
        let mut synth_streamed_chars: usize = 0;
        let mut synth_received_usage = false;

        while let Some(chunk) = rx.recv().await {
            if ctx.abort_handle.load(Ordering::SeqCst) {
                break;
            }
            match chunk {
                StreamChunk::FirstChunk => {
                    if synth_first_token_instant.is_none() {
                        synth_first_token_instant = Some(std::time::Instant::now());
                    }
                    synth_last_token_instant = Some(std::time::Instant::now());
                }
                StreamChunk::Token(token) => {
                    if synth_first_token_instant.is_none() {
                        synth_first_token_instant = Some(std::time::Instant::now());
                    }
                    synth_last_token_instant = Some(std::time::Instant::now());
                    synth_streamed_chars += token.len();

                    let parsed = thinking_parser.push(&token);
                    for (is_thought, text) in parsed {
                        if !is_thought {
                            synth_content.push_str(&text);
                        }
                    }
                }
                StreamChunk::Output(output) => {
                    if synth_first_token_instant.is_none() {
                        synth_first_token_instant = Some(std::time::Instant::now());
                    }
                    synth_last_token_instant = Some(std::time::Instant::now());
                    synth_streamed_chars += output.len();
                    synth_content.push_str(&output);
                }
                StreamChunk::Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens,
                    cache_creation_tokens,
                } => {
                    synth_received_usage = true;
                    let total_input = input_tokens + cache_read_tokens;
                    let cache_hit_rate = if total_input > 0 {
                        (cache_read_tokens as f32 / total_input as f32) * 100.0
                    } else {
                        0.0
                    };
                    let start_time = synth_first_token_instant.unwrap_or(synth_start_instant);
                    let end_time = synth_last_token_instant.unwrap_or_else(std::time::Instant::now);
                    let duration_secs = end_time.duration_since(start_time).as_secs_f64();
                    let duration_ms = duration_secs * 1000.0;
                    let ttft_ms = synth_first_token_instant
                        .map(|t| t.duration_since(synth_start_instant).as_secs_f64() * 1000.0)
                        .unwrap_or(0.0);
                    let tps = if duration_secs > 0.05 && output_tokens > 0 {
                        (output_tokens as f64 / duration_secs) as f32
                    } else {
                        0.0
                    };
                    if let Some(ref app) = ctx.app_handle {
                        let _ = app.emit(
                            "agent:telemetry",
                            serde_json::json!({
                                "taskId": &subagent_id,
                                "inputTokens": input_tokens,
                                "outputTokens": output_tokens,
                                "cacheReadTokens": cache_read_tokens,
                                "cacheCreationTokens": cache_creation_tokens,
                                "cacheHitRate": cache_hit_rate,
                                "durationMs": duration_ms,
                                "tps": tps,
                                "ttftMs": ttft_ms,
                                "systemTokens": synth_system_tokens,
                                "toolTokens": 0,
                            }),
                        );
                        if let Some(ref cid) = tool_call_id {
                            if cid != &subagent_id {
                                let _ = app.emit(
                                    "agent:telemetry",
                                    serde_json::json!({
                                        "taskId": cid,
                                        "inputTokens": input_tokens,
                                        "outputTokens": output_tokens,
                                        "cacheReadTokens": cache_read_tokens,
                                        "cacheCreationTokens": cache_creation_tokens,
                                        "cacheHitRate": cache_hit_rate,
                                        "durationMs": duration_ms,
                                        "tps": tps,
                                        "ttftMs": ttft_ms,
                                        "systemTokens": synth_system_tokens,
                                        "toolTokens": 0,
                                    }),
                                );
                            }
                        }
                    }
                }
                StreamChunk::Done => break,
                StreamChunk::Error(e) => {
                    eprintln!("CVH [subagent synthesis]: error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        for (is_thought, text) in thinking_parser.flush() {
            if !is_thought {
                synth_content.push_str(&text);
            }
        }

        if !synth_received_usage && !ctx.abort_handle.load(Ordering::SeqCst) {
            let start_time = synth_first_token_instant.unwrap_or(synth_start_instant);
            let end_time = synth_last_token_instant.unwrap_or_else(std::time::Instant::now);
            let duration_secs = end_time.duration_since(start_time).as_secs_f64();
            let duration_ms = duration_secs * 1000.0;
            let ttft_ms = synth_first_token_instant
                .map(|t| t.duration_since(synth_start_instant).as_secs_f64() * 1000.0)
                .unwrap_or(0.0);
            let output_tokens = (compactor::estimate_text_tokens(&synth_content) as u64)
                .max((synth_streamed_chars as f64 / 3.8).ceil() as u64);
            let input_tokens = compactor::estimate_request_tokens(&sub_messages, None) as u64;
            let tps = if duration_secs > 0.05 && output_tokens > 0 {
                (output_tokens as f64 / duration_secs) as f32
            } else {
                0.0
            };
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit(
                    "agent:telemetry",
                    serde_json::json!({
                        "taskId": &subagent_id,
                        "inputTokens": input_tokens,
                        "outputTokens": output_tokens,
                        "cacheReadTokens": 0,
                        "cacheCreationTokens": 0,
                        "cacheHitRate": 0.0,
                        "durationMs": duration_ms,
                        "tps": tps,
                        "ttftMs": ttft_ms,
                        "systemTokens": synth_system_tokens,
                        "toolTokens": 0,
                    }),
                );
                if let Some(ref cid) = tool_call_id {
                    if cid != &subagent_id {
                        let _ = app.emit(
                            "agent:telemetry",
                            serde_json::json!({
                                "taskId": cid,
                                "inputTokens": input_tokens,
                                "outputTokens": output_tokens,
                                "cacheReadTokens": 0,
                                "cacheCreationTokens": 0,
                                "cacheHitRate": 0.0,
                                "durationMs": duration_ms,
                                "tps": tps,
                                "ttftMs": ttft_ms,
                                "systemTokens": synth_system_tokens,
                                "toolTokens": 0,
                            }),
                        );
                    }
                }
            }
        }

        if !synth_content.trim().is_empty() {
            final_sub_response = synth_content.trim().to_string();
            let out_item = ActivityItem {
                id: format!("sub-out-{}", subagent_suffix),
                item_type: "output".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: Some(final_sub_response.clone()),
                tool: None,
                duration_ms: None,
                images: None,
            };
            let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&out_item));
            if let Some(ref app) = ctx.app_handle {
                let _ = app.emit("subagent:activity", serde_json::json!({
                    "parentTaskId": &parent_id,
                    "subagentId": &subagent_id,
                    "activity": &out_item,
                }));
            }
        }
    }

    // Fallback: if still empty, collect last tool results and assistant messages
    if final_sub_response.trim().is_empty() {
        let mut collected = Vec::new();
        for msg in sub_messages.iter().rev() {
            if msg.role == Role::Tool || msg.role == Role::Assistant {
                if let Some(ref c) = msg.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        collected.push(trimmed.to_string());
                        if collected.len() >= 3 {
                            break;
                        }
                    }
                }
            }
        }
        if !collected.is_empty() {
            collected.reverse();
            final_sub_response = collected.join("

");
        }
    }

    let report = format!(
        "[Subagent ID: {}]
Subagent [{}] '{}' report:

{}",
        subagent_id,
        role,
        title,
        if final_sub_response.trim().is_empty() {
            "Subagent completed work."
        } else {
            final_sub_response.trim()
        }
    );

    if ctx.abort_handle.load(Ordering::SeqCst) {
        let duration_ms = start_instant.elapsed().as_millis() as f64;
        if let Some((_, session)) = find_subagent_session(Some(&parent_id), &subagent_id).await {
            let mut info = session.retry_info.lock().await;
            info.status = "aborted".to_string();
            info.last_error = Some("Прервано пользователем".to_string());
            info.duration_ms = Some(duration_ms);
            if let Some(sender) = session.sender.lock().await.take() {
                let _ = sender.send("[Субагент прерван пользователем / Subagent aborted by user]".to_string());
            }
        }
        if let Some(ref app) = ctx.app_handle {
            let _ = app.emit(
                "subagent:aborted",
                serde_json::json!({
                    "parentTaskId": &parent_id,
                    "subagentId": &subagent_id,
                    "durationMs": duration_ms,
                }),
            );
        }
        bail!("Subagent [{}] was aborted by user", subagent_id);
    }

    let duration_ms = start_instant.elapsed().as_millis() as f64;

    // Subagent completed cleanly: deliver report to waiting parent channel!
    if let Some((_, session)) = find_subagent_session(Some(&parent_id), &subagent_id).await {
        let mut info = session.retry_info.lock().await;
        info.status = "finished".to_string();
        info.last_error = None;
        info.report = Some(report.clone());
        info.duration_ms = Some(duration_ms);
        if let Some(sender) = session.sender.lock().await.take() {
            let _ = sender.send(report.clone());
        }
    }
    prune_subagent_sessions_cache().await;

    if let Some(ref app) = ctx.app_handle {
        let _ = app.emit(
            "subagent:finished",
            serde_json::json!({
                "parentTaskId": &parent_id,
                "subagentId": &subagent_id,
                "response": &final_sub_response,
                "durationMs": duration_ms,
            }),
        );
    }

    Ok(report)
    })
}

pub async fn retry_subagent_execution(
    parent_task_id: Option<&str>,
    subagent_id: &str,
    additional_prompt: Option<String>,
) -> Result<()> {
    let (resolved_id, session) = find_subagent_session(parent_task_id, subagent_id)
        .await
        .context(format!(
            "Subagent session not found in registry (parent: {:?}, id: '{}')",
            parent_task_id, subagent_id
        ))?;

    // Keep existing sender channel so parent agent awaiting rx is NEVER dropped!
    {
        let mut tx_guard = session.sender.lock().await;
        if tx_guard.is_none() {
            let (new_tx, new_rx) = oneshot::channel::<String>();
            *tx_guard = Some(new_tx);
            let mut receivers = SUBAGENT_RECEIVERS.lock().await;
            receivers.insert(resolved_id.clone(), new_rx);
        }
    }

    let mut info = session.retry_info.lock().await.clone();

    if let Some(msg) = additional_prompt {
        let trimmed = msg.trim();
        if !trimmed.is_empty() {
            info.prompt = format!("{}

[USER GUIDANCE]: {}", info.prompt, trimmed);
            let p_id = parent_task_id.unwrap_or(info.parent_task_id.as_str());
            let sub_dialog_id = if resolved_id.contains(":sub:") {
                resolved_id.clone()
            } else if !p_id.is_empty() {
                format!("{}:sub:{}", p_id, resolved_id)
            } else {
                resolved_id.clone()
            };
            let user_item = ActivityItem {
                id: format!(
                    "sub-user-{}-{}",
                    sub_dialog_id.replace(':', "-"),
                    chrono::Utc::now().timestamp_millis()
                ),
                item_type: "user".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                content: Some(trimmed.to_string()),
                tool: None,
                duration_ms: None,
                images: None,
            };
            let _ = crate::db::append_dialog_items(
                &info.ctx.repo_root,
                &sub_dialog_id,
                std::slice::from_ref(&user_item),
            );
            if let Some(ref app) = info.ctx.app_handle {
                let _ = app.emit(
                    "subagent:activity",
                    serde_json::json!({
                        "parentTaskId": p_id,
                        "subagentId": &resolved_id,
                        "activity": &user_item,
                    }),
                );
            }
        }
    }
    info.status = "running".to_string();
    info.last_error = None;
    info.report = None;

    {
        let mut session_info = session.retry_info.lock().await;
        *session_info = info.clone();
    }

    let owned_ctx = info.ctx.clone();
    let title_s = info.title.clone();
    let role_s = info.role.clone();
    let prompt_s = info.prompt.clone();
    let requested_tools = info.requested_tools.clone();
    let sub_id = resolved_id.clone();

    tokio::spawn(async move {
        let _ = run_inprocess_subagent(
            owned_ctx,
            title_s,
            role_s,
            prompt_s,
            requested_tools,
            Some(sub_id),
        ).await;
    });

    Ok(())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::Prefix(p) => components.push(std::path::Component::Prefix(p)),
            std::path::Component::RootDir => components.push(std::path::Component::RootDir),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::Normal(c) => components.push(std::path::Component::Normal(c)),
        }
    }
    components.into_iter().collect()
}

fn resolve_safe_path(worktree_dir: &Path, rel_path: &str) -> Result<PathBuf> {
    let cleaned = rel_path.trim_start_matches(['/', '\\']);
    if cleaned.contains(':') {
        bail!("Absolute path with drive letter denied: {:?}", rel_path);
    }

    let candidate = worktree_dir.join(cleaned);
    let norm_worktree = normalize_path(worktree_dir);
    let norm_candidate = normalize_path(&candidate);

    if !norm_candidate.starts_with(&norm_worktree) {
        bail!(
            "Path traversal denied: {:?} attempts to escape worktree boundary {:?}",
            rel_path,
            worktree_dir
        );
    }

    Ok(norm_candidate)
}

async fn run_scoped_command(
    cwd: &Path,
    cmd_str: &str,
    timeout: Duration,
    abort_flag: Arc<AtomicBool>,
) -> Result<crate::hooks::HookExecutionResult> {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd.exe");
        c.raw_arg("/D");
        c.raw_arg("/S");
        c.raw_arg("/C");
        c.raw_arg(format!("\"chcp 65001 >nul && {}\"", cmd_str));
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.args(["-c", cmd_str]);
        c
    };

    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd.spawn().with_context(|| format!("Failed to spawn command: {}", cmd_str))?;
    let pid = child.id();

    let mut stdout_pipe = child.stdout.take().context("Failed to take stdout")?;
    let mut stderr_pipe = child.stderr.take().context("Failed to take stderr")?;

    let run_task = async {
        let stdout_drain = async {
            let mut buf = Vec::new();
            let _ = stdout_pipe.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).to_string()
        };

        let stderr_drain = async {
            let mut buf = Vec::new();
            let _ = stderr_pipe.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).to_string()
        };

        let (stdout_str, stderr_str) = tokio::join!(stdout_drain, stderr_drain);
        let status = child.wait().await?;
        Ok::<crate::hooks::HookExecutionResult, anyhow::Error>(crate::hooks::HookExecutionResult {
            success: status.success(),
            exit_code: status.code(),
            stdout: stdout_str,
            stderr: stderr_str,
        })
    };

    let check_interval = async {
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            if abort_flag.load(Ordering::SeqCst) {
                if let Some(p) = pid {
                    kill_process_tree(p).await;
                }
                return;
            }
        }
    };

    tokio::select! {
        res = run_task => res,
        _ = tokio::time::sleep(timeout) => {
            if let Some(p) = pid {
                kill_process_tree(p).await;
            }
            bail!("Command '{}' timed out after {:?}", cmd_str, timeout)
        }
        _ = check_interval => {
            bail!("Command cancelled by user")
        }
    }
}

async fn collect_matching_files(
    start_dir: &Path,
    worktree_root: &Path,
    pattern: &str,
    out: &mut Vec<String>,
) -> Result<()> {
    if !start_dir.exists() {
        return Ok(());
    }

    let mut dir_stack = vec![start_dir.to_path_buf()];

    while let Some(current_dir) = dir_stack.pop() {
        let mut read_dir = match tokio::fs::read_dir(&current_dir).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            let file_type = match entry.file_type().await {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if file_type.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == ".git" || name == "target" || name == "node_modules" {
                    continue;
                }
                dir_stack.push(path);
            } else if file_type.is_file() {
                let rel = path
                    .strip_prefix(worktree_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");

                if glob_match(pattern, &rel) {
                    out.push(rel);
                }
            }
        }
    }

    Ok(())
}

fn glob_match(pattern: &str, candidate: &str) -> bool {
    let p = pattern.replace('\\', "/");
    let c = candidate.replace('\\', "/");
    glob_match_slices(p.as_bytes(), c.as_bytes())
}

fn glob_match_slices(pattern: &[u8], candidate: &[u8]) -> bool {
    if pattern.is_empty() {
        return candidate.is_empty();
    }
    if pattern.starts_with(b"**") {
        let rest_pat = if pattern.starts_with(b"**/*") {
            &pattern[4..]
        } else if pattern.starts_with(b"**/") {
            &pattern[3..]
        } else {
            &pattern[2..]
        };
        for i in 0..=candidate.len() {
            if glob_match_slices(rest_pat, &candidate[i..]) {
                return true;
            }
        }
        return false;
    }
    if pattern[0] == b'*' {
        let rest_pat = &pattern[1..];
        for i in 0..=candidate.len() {
            if i > 0 && candidate[i - 1] == b'/' {
                break;
            }
            if glob_match_slices(rest_pat, &candidate[i..]) {
                return true;
            }
        }
        return false;
    }
    if !candidate.is_empty() && (pattern[0] == b'?' || pattern[0] == candidate[0]) {
        return glob_match_slices(&pattern[1..], &candidate[1..]);
    }
    false
}

async fn grep_walk(
    target: &Path,
    worktree_root: &Path,
    needle: &str,
    out: &mut Vec<String>,
) -> Result<()> {
    if target.is_file() {
        if let Ok(content) = tokio::fs::read_to_string(target).await {
            let rel = target
                .strip_prefix(worktree_root)
                .unwrap_or(target)
                .to_string_lossy()
                .replace('\\', "/");

            for (idx, line) in content.lines().enumerate() {
                if line.contains(needle) {
                    out.push(format!("{}:{}: {}", rel, idx + 1, line.trim()));
                    if out.len() >= 200 {
                        return Ok(());
                    }
                }
            }
        }
        return Ok(());
    }

    let mut dir_stack = vec![target.to_path_buf()];

    while let Some(current_dir) = dir_stack.pop() {
        let mut read_dir = match tokio::fs::read_dir(&current_dir).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            let file_type = match entry.file_type().await {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" || name == "target" || name == "node_modules" {
                continue;
            }

            if file_type.is_dir() {
                dir_stack.push(path);
            } else if file_type.is_file() {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    let rel = path
                        .strip_prefix(worktree_root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");

                    for (idx, line) in content.lines().enumerate() {
                        if line.contains(needle) {
                            out.push(format!("{}:{}: {}", rel, idx + 1, line.trim()));
                            if out.len() >= 200 {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// Web Tools Implementation (DuckDuckGo Search & Markdown Fetch)
// ============================================================================

pub async fn execute_web_search(query: &str, limit: usize) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    // 1. DuckDuckGo HTML Search
    let url = format!("https://html.duckduckgo.com/html/?q={}", simple_url_encode(query));
    if let Ok(resp) = client.get(&url).send().await {
        if resp.status().is_success() {
            if let Ok(html) = resp.text().await {
                let results = parse_ddg_results(&html, limit);
                if !results.is_empty() {
                    return Ok(results.join("\n\n"));
                }
            }
        }
    }

    // 2. Fallback: DuckDuckGo Instant Answer API
    let api_url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        simple_url_encode(query)
    );
    if let Ok(resp) = client.get(&api_url).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            let mut out = Vec::new();
            if let Some(abs) = json.get("AbstractText").and_then(|v| v.as_str()) {
                if !abs.is_empty() {
                    let source = json.get("AbstractURL").and_then(|v| v.as_str()).unwrap_or("");
                    out.push(format!("### Abstract\n{}\nSource: {}", abs, source));
                }
            }
            if let Some(topics) = json.get("RelatedTopics").and_then(|v| v.as_array()) {
                for t in topics.iter().take(limit) {
                    if let Some(txt) = t.get("Text").and_then(|v| v.as_str()) {
                        let u = t.get("FirstURL").and_then(|v| v.as_str()).unwrap_or("");
                        out.push(format!("- **{}**: {}", txt, u));
                    }
                }
            }
            if !out.is_empty() {
                return Ok(out.join("\n\n"));
            }
        }
    }

    Ok(format!("Search completed for query '{}'. No web records matched.", query))
}

fn simple_url_encode(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}

fn parse_ddg_results(html: &str, limit: usize) -> Vec<String> {
    let mut results = Vec::new();
    let mut pos = 0;
    while let Some(start_res) = html[pos..].find("class=\"result__body\"") {
        let abs_start = pos + start_res;
        let end_res = html[abs_start..].find("</div>\n</div>").unwrap_or(1500);
        let block = &html[abs_start..abs_start + end_res];

        let title = extract_snippet_tag(block, "result__a");
        let snippet = extract_snippet_tag(block, "result__snippet");
        let url = extract_snippet_href(block, "result__a");

        if !title.is_empty() {
            results.push(format!(
                "### [{}]({})\n{}",
                clean_tags(&title),
                url,
                clean_tags(&snippet)
            ));
        }

        pos = abs_start + end_res;
        if results.len() >= limit {
            break;
        }
    }
    results
}

fn extract_snippet_tag(block: &str, class_name: &str) -> String {
    if let Some(idx) = block.find(class_name) {
        if let Some(open) = block[idx..].find('>') {
            let start = idx + open + 1;
            if let Some(close) = block[start..].find('<') {
                return block[start..start + close].trim().to_string();
            }
        }
    }
    String::new()
}

fn extract_snippet_href(block: &str, class_name: &str) -> String {
    if let Some(idx) = block.find(class_name) {
        let search_range = &block[idx.saturating_sub(60)..idx.min(block.len())];
        if let Some(href_idx) = search_range.find("href=\"") {
            let start = href_idx + 6;
            if let Some(end) = search_range[start..].find('"') {
                return search_range[start..start + end].to_string();
            }
        }
    }
    String::new()
}

fn clean_tags(s: &str) -> String {
    let mut clean = String::new();
    let mut inside = false;
    for c in s.chars() {
        if c == '<' {
            inside = true;
        } else if c == '>' {
            inside = false;
        } else if !inside {
            clean.push(c);
        }
    }
    clean
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}

pub async fn execute_web_fetch(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(18))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch URL: {}", url))?;

    if !response.status().is_success() {
        bail!("HTTP request failed with status: {}", response.status());
    }

    let html = response.text().await.context("Failed to read HTML body")?;
    let markdown = convert_html_to_clean_markdown(&html);
    Ok(markdown)
}

fn convert_html_to_clean_markdown(html: &str) -> String {
    let mut clean = html.to_string();
    for tag in &["script", "style", "head", "nav", "footer", "svg"] {
        clean = strip_html_block(&clean, tag);
    }

    let mut md = String::new();
    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let stripped = clean_tags(trimmed);
        if !stripped.is_empty() {
            md.push_str(&stripped);
            md.push('\n');
        }
    }

    if md.is_empty() {
        "(Empty web page content)".to_string()
    } else {
        md
    }
}

fn strip_html_block(html: &str, tag: &str) -> String {
    let open_pat = format!("<{}", tag);
    let close_pat = format!("</{}>", tag);
    let mut result = String::new();
    let mut pos = 0;

    while let Some(open_idx) = html[pos..].to_lowercase().find(&open_pat) {
        let abs_open = pos + open_idx;
        result.push_str(&html[pos..abs_open]);

        if let Some(close_idx) = html[abs_open..].to_lowercase().find(&close_pat) {
            pos = abs_open + close_idx + close_pat.len();
        } else {
            pos = html.len();
            break;
        }
    }
    result.push_str(&html[pos..]);
    result
}

// ============================================================================
// Agent Loop Execution
// ============================================================================

pub struct AgentSessionConfig {
    pub repo_root: PathBuf,
    pub worktree_path: PathBuf,
    pub role: String,
    pub stage_id: String,
    pub task_id: String,
    pub task_title: String,
    pub task_description: String,
    pub handover_summary: Option<String>,
    pub task_attachments: Option<Vec<String>>,
    pub steer_prompt: Option<String>,
    pub steer_images: Option<Vec<String>>,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<String>,
    pub max_turns: usize,
    pub app_handle: Option<AppHandle>,
    pub context_window: Option<u64>,
}

/// Checks if an opening tag candidate (<thinking> or <think>) at `pos` is a genuine model
/// control tag rather than markdown inline code (`<thinking>`), code fence, or word substring.
pub fn is_valid_opening_tag(buffer: &str, pos: usize, tag_len: usize) -> bool {
    if !buffer.is_char_boundary(pos) || !buffer.is_char_boundary(pos + tag_len) || pos + tag_len > buffer.len() {
        return false;
    }

    // 1. Check character immediately preceding the tag
    if pos > 0 {
        if let Some(prev_char) = buffer.get(..pos).and_then(|s| s.chars().next_back()) {
            // If preceded by a backtick (e.g. `<thinking>` or ```<thinking>), it's markdown inline code or fence
            if prev_char == '`' {
                return false;
            }
            // If preceded by an alphanumeric char or underscore, it's inside an identifier/word (e.g. var<thinking>)
            if prev_char.is_alphanumeric() || prev_char == '_' {
                return false;
            }
        }
    }

    // 2. Check character immediately following the tag
    if pos + tag_len < buffer.len() {
        if let Some(next_char) = buffer.get(pos + tag_len..).and_then(|s| s.chars().next()) {
            // If immediately followed by a backtick (e.g. <thinking>`), it's markdown
            if next_char == '`' {
                return false;
            }
        }
    }

    // 3. Check if inside unclosed inline backtick code on the current line
    if let Some(before) = buffer.get(..pos) {
        let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        if let Some(line_before) = before.get(line_start..) {
            let backticks_before = line_before.chars().filter(|&c| c == '`').count();
            if backticks_before % 2 == 1 {
                return false;
            }
        }
    }

    true
}

/// Checks if a closing tag candidate (</thinking> or </think>) at `pos` is a genuine model
/// control tag rather than markdown inline code (`</thinking>`).
pub fn is_valid_closing_tag(buffer: &str, pos: usize, tag_len: usize) -> bool {
    if !buffer.is_char_boundary(pos) || !buffer.is_char_boundary(pos + tag_len) || pos + tag_len > buffer.len() {
        return false;
    }

    if pos > 0 {
        if let Some(prev_char) = buffer.get(..pos).and_then(|s| s.chars().next_back()) {
            if prev_char == '`' {
                return false;
            }
        }
    }

    if pos + tag_len < buffer.len() {
        if let Some(next_char) = buffer.get(pos + tag_len..).and_then(|s| s.chars().next()) {
            if next_char == '`' {
                return false;
            }
        }
    }

    if let Some(before) = buffer.get(..pos) {
        let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        if let Some(line_before) = before.get(line_start..) {
            let backticks_before = line_before.chars().filter(|&c| c == '`').count();
            if backticks_before % 2 == 1 {
                return false;
            }
        }
    }

    true
}

/// Helper to check if a potential partial tag prefix at `pos` could be a valid opening tag
pub fn is_valid_partial_opening_tag(buffer: &str, pos: usize) -> bool {
    if !buffer.is_char_boundary(pos) || pos > buffer.len() {
        return false;
    }
    if pos > 0 {
        if let Some(prev_char) = buffer.get(..pos).and_then(|s| s.chars().next_back()) {
            if prev_char == '`' || prev_char.is_alphanumeric() || prev_char == '_' {
                return false;
            }
        }
    }
    if let Some(before) = buffer.get(..pos) {
        let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        if let Some(line_before) = before.get(line_start..) {
            let backticks_before = line_before.chars().filter(|&c| c == '`').count();
            if backticks_before % 2 == 1 {
                return false;
            }
        }
    }
    true
}

#[derive(Debug, Default)]
pub struct ThinkingStreamParser {
    in_thinking: bool,
    in_backtick: bool,
    buffer: String,
}

impl ThinkingStreamParser {
    pub fn new() -> Self {
        Self {
            in_thinking: false,
            in_backtick: false,
            buffer: String::new(),
        }
    }

    /// Process an incoming token chunk. Returns a list of (is_thought, text) chunks.
    pub fn push(&mut self, chunk: &str) -> Vec<(bool, String)> {
        self.buffer.push_str(chunk);
        let mut outputs = Vec::new();

        loop {
            if !self.in_thinking {
                // Look for opening tag <thinking> or <think>
                let mut earliest_open: Option<(usize, usize)> = None;
                let mut search_pos = 0;
                while search_pos < self.buffer.len() {
                    let sub = match self.buffer.get(search_pos..) {
                        Some(s) => s,
                        None => break,
                    };
                    let open_thinking = sub.find("<thinking>").map(|p| search_pos + p);
                    let open_think = sub.find("<think>").map(|p| search_pos + p);

                    let next_cand = match (open_thinking, open_think) {
                        (Some(pos1), Some(pos2)) => {
                            if pos1 <= pos2 {
                                Some((pos1, "<thinking>".len()))
                            } else {
                                Some((pos2, "<think>".len()))
                            }
                        }
                        (Some(pos), None) => Some((pos, "<thinking>".len())),
                        (None, Some(pos)) => Some((pos, "<think>".len())),
                        (None, None) => None,
                    };

                    match next_cand {
                        Some((pos, tag_len)) => {
                            let backticks_before = self.buffer.get(..pos).map(|s| s.chars().filter(|&c| c == '`').count()).unwrap_or(0);
                            let effective_in_backtick = (self.in_backtick as usize + backticks_before) % 2 == 1;

                            if !effective_in_backtick && is_valid_opening_tag(&self.buffer, pos, tag_len) {
                                earliest_open = Some((pos, tag_len));
                                break;
                            } else {
                                let advance = self.buffer.get(pos..).and_then(|s| s.chars().next()).map(|c| c.len_utf8()).unwrap_or(1);
                                search_pos = pos + advance;
                            }
                        }
                        None => break,
                    }
                }

                if let Some((pos, tag_len)) = earliest_open {
                    if pos > 0 {
                        let text = self.buffer.get(..pos).unwrap_or("").to_string();
                        for c in text.chars() {
                            if c == '`' {
                                self.in_backtick = !self.in_backtick;
                            }
                        }
                        outputs.push((false, text));
                    }
                    self.in_thinking = true;
                    self.in_backtick = false;
                    self.buffer = self.buffer.get(pos + tag_len..).unwrap_or("").to_string();
                    continue;
                }

                // If not found, check if end of buffer could be a partial opening tag
                let mut partial_len = 0;
                if !self.in_backtick {
                    for tag in &["<thinking>", "<think>"] {
                        for len in 1..tag.len() {
                            if self.buffer.ends_with(&tag[..len]) && len > partial_len {
                                let pos = self.buffer.len() - len;
                                let backticks_before = self.buffer.get(..pos).map(|s| s.chars().filter(|&c| c == '`').count()).unwrap_or(0);
                                let effective_in_backtick = (self.in_backtick as usize + backticks_before) % 2 == 1;
                                if !effective_in_backtick && is_valid_partial_opening_tag(&self.buffer, pos) {
                                    partial_len = len;
                                }
                            }
                        }
                    }
                }

                if partial_len > 0 {
                    let safe_len = self.buffer.len() - partial_len;
                    if safe_len > 0 && self.buffer.is_char_boundary(safe_len) {
                        let text = self.buffer.get(..safe_len).unwrap_or("").to_string();
                        for c in text.chars() {
                            if c == '`' {
                                self.in_backtick = !self.in_backtick;
                            }
                        }
                        outputs.push((false, text));
                        self.buffer = self.buffer.get(safe_len..).unwrap_or("").to_string();
                    }
                    break;
                } else {
                    if !self.buffer.is_empty() {
                        let text = std::mem::take(&mut self.buffer);
                        for c in text.chars() {
                            if c == '`' {
                                self.in_backtick = !self.in_backtick;
                            }
                        }
                        outputs.push((false, text));
                    }
                    break;
                }
            } else {
                // In thinking: look for closing tag </thinking> or </think>
                let mut earliest_close: Option<(usize, usize)> = None;
                let mut search_pos = 0;
                while search_pos < self.buffer.len() {
                    let sub = match self.buffer.get(search_pos..) {
                        Some(s) => s,
                        None => break,
                    };
                    let close_thinking = sub.find("</thinking>").map(|p| search_pos + p);
                    let close_think = sub.find("</think>").map(|p| search_pos + p);

                    let next_cand = match (close_thinking, close_think) {
                        (Some(pos1), Some(pos2)) => {
                            if pos1 <= pos2 {
                                Some((pos1, "</thinking>".len()))
                            } else {
                                Some((pos2, "</think>".len()))
                            }
                        }
                        (Some(pos), None) => Some((pos, "</thinking>".len())),
                        (None, Some(pos)) => Some((pos, "</think>".len())),
                        (None, None) => None,
                    };

                    match next_cand {
                        Some((pos, tag_len)) => {
                            if is_valid_closing_tag(&self.buffer, pos, tag_len) {
                                earliest_close = Some((pos, tag_len));
                                break;
                            } else {
                                let advance = self.buffer.get(pos..).and_then(|s| s.chars().next()).map(|c| c.len_utf8()).unwrap_or(1);
                                search_pos = pos + advance;
                            }
                        }
                        None => break,
                    }
                }

                if let Some((pos, tag_len)) = earliest_close {
                    if pos > 0 {
                        let text = self.buffer.get(..pos).unwrap_or("").to_string();
                        outputs.push((true, text));
                    }
                    self.in_thinking = false;
                    self.in_backtick = false;
                    self.buffer = self.buffer.get(pos + tag_len..).unwrap_or("").to_string();
                    continue;
                }

                // If not found, check if end of buffer could be a partial closing tag
                let mut partial_len = 0;
                for tag in &["</thinking>", "</think>"] {
                    for len in 1..tag.len() {
                        if self.buffer.ends_with(&tag[..len]) && len > partial_len {
                            let pos = self.buffer.len() - len;
                            if is_valid_partial_opening_tag(&self.buffer, pos) {
                                partial_len = len;
                            }
                        }
                    }
                }

                if partial_len > 0 {
                    let safe_len = self.buffer.len() - partial_len;
                    if safe_len > 0 && self.buffer.is_char_boundary(safe_len) {
                        let text = self.buffer.get(..safe_len).unwrap_or("").to_string();
                        outputs.push((true, text));
                        self.buffer = self.buffer.get(safe_len..).unwrap_or("").to_string();
                    }
                    break;
                } else {
                    if !self.buffer.is_empty() {
                        outputs.push((true, std::mem::take(&mut self.buffer)));
                    }
                    break;
                }
            }
        }

        outputs
    }

    /// Flush any remaining buffered characters at the end of the stream
    pub fn flush(&mut self) -> Vec<(bool, String)> {
        if self.buffer.is_empty() {
            Vec::new()
        } else {
            let item = (self.in_thinking, std::mem::take(&mut self.buffer));
            vec![item]
        }
    }
}

/// Scans the conversation for any assistant messages with unclosed tool calls
/// (e.g. when execution was aborted mid-tool) and appends synthetic tool responses
/// so that the transcript remains strictly valid according to LLM API schemas.
pub fn sanitize_unclosed_tool_calls(messages: &mut Vec<ChatMessage>) {
    let mut i = 0;
    while i < messages.len() {
        if messages[i].role == Role::Assistant {
            if let Some(tool_calls) = messages[i].tool_calls.clone() {
                if !tool_calls.is_empty() {
                    // Collect tool_call_ids answered immediately following this assistant message
                    let mut answered_ids = std::collections::HashSet::new();
                    let mut j = i + 1;
                    while j < messages.len() && messages[j].role == Role::Tool {
                        if let Some(ref tid) = messages[j].tool_call_id {
                            answered_ids.insert(tid.clone());
                        }
                        j += 1;
                    }

                    // For any tool call that was not answered, synthesize a tool response
                    for tc in tool_calls {
                        if !answered_ids.contains(&tc.id) {
                            eprintln!(
                                "CVH [agent:sanitize]: Synthesizing tool response for unclosed tool call {} ({})",
                                tc.id, tc.function.name
                            );
                            messages.insert(
                                j,
                                ChatMessage::tool_response(
                                    tc.id.clone(),
                                    "[Инструмент прерван указанием пользователя / Tool execution interrupted by user steer]",
                                ),
                            );
                            j += 1;
                        }
                    }
                    i = j;
                    continue;
                }
            }
        }
        i += 1;
    }
}

/// Reconstructs a list of ChatMessages from persisted SQLite dialog items
/// when in-memory context has been lost (e.g. after process restart).
pub fn reconstruct_messages_from_dialog(
    system_prompt: &str,
    initial_user_prompt: &str,
    items: &[crate::db::ActivityItem],
) -> Vec<ChatMessage> {
    let mut messages = vec![
        ChatMessage::system(system_prompt).with_cache_control(crate::llm::CacheControl::ephemeral()),
        ChatMessage::user(initial_user_prompt).with_cache_control(crate::llm::CacheControl::ephemeral()),
    ];

    for item in items {
        match item.item_type.as_str() {
            "summary" => {
                if let Some(ref c) = item.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        messages.push(ChatMessage::user(trimmed));
                    }
                }
            }
            "steer" => {
                if let Some(ref c) = item.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        let formatted = if trimmed.starts_with("### [УКАЗАНИЕ ПОЛЬЗОВАТЕЛЯ") {
                            trimmed.to_string()
                        } else {
                            format!(
                                "### [УКАЗАНИЕ ПОЛЬЗОВАТЕЛЯ / USER STEER]\n{}",
                                trimmed
                            )
                        };
                        if let Some(ref imgs) = item.images {
                            if !imgs.is_empty() {
                                messages.push(ChatMessage::user_with_images(formatted, imgs.clone()));
                            } else {
                                messages.push(ChatMessage::user(formatted));
                            }
                        } else {
                            messages.push(ChatMessage::user(formatted));
                        }
                    }
                }
            }
            "user" => {
                if let Some(ref c) = item.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        if trimmed == initial_user_prompt.trim() {
                            if let Some(ref imgs) = item.images {
                                if !imgs.is_empty() && messages.len() >= 2 && messages[1].role == Role::User {
                                    messages[1] = ChatMessage::user_with_images(initial_user_prompt, imgs.clone())
                                        .with_cache_control(crate::llm::CacheControl::ephemeral());
                                }
                            }
                        } else {
                            if let Some(ref imgs) = item.images {
                                if !imgs.is_empty() {
                                    messages.push(ChatMessage::user_with_images(trimmed, imgs.clone()));
                                } else {
                                    messages.push(ChatMessage::user(trimmed));
                                }
                            } else {
                                messages.push(ChatMessage::user(trimmed));
                            }
                        }
                    }
                }
            }
            "output" | "assistant" => {
                if let Some(ref c) = item.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        messages.push(ChatMessage::assistant(Some(trimmed.to_string()), None));
                    }
                }
            }
            "tool" => {
                if let Some(ref tool_val) = item.tool {
                    let call_id = tool_val.get("id").and_then(|v| v.as_str()).unwrap_or(&item.id);

                    // 1. Имя инструмента: проверяем 'name', затем 'toolName', затем алиасы
                    let name = tool_val
                        .get("name")
                        .or_else(|| tool_val.get("toolName"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown_tool");

                    // 2. Аргументы: поддерживаем и строку 'arguments', и объект 'params'
                    let args = if let Some(args_str) = tool_val.get("arguments").and_then(|v| v.as_str()) {
                        args_str.to_string()
                    } else if let Some(params_val) = tool_val.get("params") {
                        if params_val.is_string() {
                            params_val.as_str().unwrap().to_string()
                        } else {
                            serde_json::to_string(params_val).unwrap_or_else(|_| "{}".to_string())
                        }
                    } else {
                        "{}".to_string()
                    };

                    // 3. Результат: проверяем 'result', 'stdout', 'output' или item.content
                    let result = tool_val
                        .get("result")
                        .or_else(|| tool_val.get("stdout"))
                        .or_else(|| tool_val.get("output"))
                        .and_then(|v| v.as_str())
                        .or(item.content.as_deref())
                        .unwrap_or("");

                    let tc = crate::llm::ToolCall {
                        id: call_id.to_string(),
                        call_type: "function".to_string(),
                        function: crate::llm::FunctionCall {
                            name: name.to_string(),
                            arguments: args,
                        },
                    };
                    messages.push(ChatMessage::assistant(None, Some(vec![tc])));
                    messages.push(ChatMessage::tool_response(call_id, result));
                }
            }
            "compacted" => {
                if let Some(ref c) = item.content {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        let formatted = if trimmed.starts_with("# Compacted Context Summary") {
                            trimmed.to_string()
                        } else {
                            format!("# Compacted Context Summary\n\n{}", trimmed)
                        };
                        messages.push(ChatMessage::system(formatted));
                    }
                }
            }
            _ => {}
        }
    }

    sanitize_unclosed_tool_calls(&mut messages);
    messages
}

fn sync_context_state(
    app_handle: &Option<AppHandle>,
    task_id: &str,
    turn: usize,
    messages: &[ChatMessage],
) {
    if let Some(ref app) = app_handle {
        if let Some(state) = app.try_state::<crate::commands::AppState>() {
            if let Ok(map) = state.agent_contexts.lock() {
                if let Some(cs) = map.get(task_id) {
                    if let Ok(mut guard) = cs.lock() {
                        guard.current_turn = turn;
                        guard.messages = messages.to_vec();
                    }
                }
            }
        }
    }
}

/// Executes the full ReAct loop for an agent session with deterministic prefix caching,
/// zero-trust tool policy filtering, and state-preserving context compaction.
pub async fn run_agent_loop<F>(
    config: AgentSessionConfig,
    abort_flag: Arc<AtomicBool>,
    event_sink: F,
) -> Result<String>
where
    F: Fn(AgentEvent) + Send + Sync + 'static,
{
    // Tool Policy Matrix (§ 2.16 & § 2.21):
    // tool-policy.json is the single authoritative source of truth for tool permissions.
    let tool_policy = load_tool_policies(&config.repo_root)
        .await
        .unwrap_or_default();

    // Tool permissions are dynamically evaluated from ToolPolicyConfig without touching persona.md.

    // Best-effort per-agent overrides (missing/broken → defaults).
    let agent_setting = load_agent_setting(&config.repo_root, &config.role).await;
    let agent_allowed = agent_setting.allowed_tools.clone();

    let all_tools = get_all_tools_for_role(&config.repo_root, Some(&config.role)).await;

    let mut active_tools: Vec<ToolDefinition> = all_tools
        .into_iter()
        .filter(|t| {
            is_tool_active_for_agent(
                &t.function.name,
                &tool_policy,
                &config.role,
                Some(&config.stage_id),
                agent_allowed.as_deref(),
            )
        })
        .collect();

    // Deterministic sorting of tool definitions for prefix caching
    active_tools.sort_by(|a, b| a.function.name.cmp(&b.function.name));
    let active_tool_names: Vec<String> = active_tools.iter().map(|t| t.function.name.clone()).collect();

    // Deterministic static system prompt with sanitized frontmatter and dynamic tool mandates
    let system_prompt = compile_primed_system_prompt_with_tools(
        &config.repo_root,
        &config.role,
        Some(&active_tool_names),
    ).await?;

    // Zero-Turn Memory Priming Relay Handoff (§ 2.9 AGENTS.md):
    // If task has handover_summary, inject at the beginning of the prompt.
    let mut initial_user_prompt = String::new();
    if let Some(ref handover) = config.handover_summary {
        let trimmed = handover.trim();
        if !trimmed.is_empty() {
            initial_user_prompt.push_str(&format!(
                "### [ЭСТАФЕТА ПАЙПЛАЙНА / RELAY HANDOFF]\nПредыдущий этап пайплайна передал следующее саммари:\n{}\n\n",
                trimmed
            ));
        }
    }
    initial_user_prompt.push_str(&format!(
        "# Objective: {}\n\n{}",
        config.task_title,
        config.task_description.trim()
    ));

    if let Some(ref atts) = config.task_attachments {
        if !atts.is_empty() {
            initial_user_prompt.push_str(&format!(
                "\n\n[ВЛОЖЕНИЯ К ЗАДАЧЕ / ATTACHMENTS]: Прикреплено {} изображений/скриншотов. Внимательно изучи их при выполнении задачи.",
                atts.len()
            ));
        }
    }

    // 1. Check if an existing in-memory context already exists for this task_id
    let in_memory_context = if let Some(ref app) = config.app_handle {
        if let Some(state) = app.try_state::<crate::commands::AppState>() {
            state.agent_contexts.lock().ok().and_then(|map| {
                map.get(&config.task_id).and_then(|cs| {
                    let guard = cs.lock().ok()?;
                    if guard.messages.len() > 2 {
                        Some((guard.current_turn, guard.messages.clone()))
                    } else {
                        None
                    }
                })
            })
        } else {
            None
        }
    } else {
        None
    };

    // 2. Check SQLite history if in-memory context is absent or <= 2 messages
    let sqlite_context = if in_memory_context.is_none() {
        if let Ok(dialog_items) = crate::db::load_dialog_items(&config.repo_root, &config.task_id) {
            let reconstructed = reconstruct_messages_from_dialog(&system_prompt, &initial_user_prompt, &dialog_items);
            if reconstructed.len() > 2 {
                Some((0, reconstructed))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // 3. Adopt existing messages or initialize fresh 2-message array
    let (starting_turn, mut messages) = if let Some((cur_turn, mut msgs)) = in_memory_context {
        eprintln!(
            "CVH [agent]: Adopting existing in-memory context ({} messages, turn {}) for task {}",
            msgs.len(), cur_turn, config.task_id
        );
        if let Some(ref atts) = config.task_attachments {
            if !atts.is_empty() && msgs.len() >= 2 && msgs[1].role == Role::User && !msgs[1].has_images() {
                let text = msgs[1].text_content().unwrap_or(&initial_user_prompt).to_string();
                msgs[1] = ChatMessage::user_with_images(text, atts.clone())
                    .with_cache_control(crate::llm::CacheControl::ephemeral());
            }
        }
        (cur_turn, msgs)
    } else if let Some((cur_turn, mut msgs)) = sqlite_context {
        eprintln!(
            "CVH [agent]: Adopting reconstructed SQLite dialogue context ({} messages) for task {}",
            msgs.len(), config.task_id
        );
        if let Some(ref atts) = config.task_attachments {
            if !atts.is_empty() && msgs.len() >= 2 && msgs[1].role == Role::User && !msgs[1].has_images() {
                let text = msgs[1].text_content().unwrap_or(&initial_user_prompt).to_string();
                msgs[1] = ChatMessage::user_with_images(text, atts.clone())
                    .with_cache_control(crate::llm::CacheControl::ephemeral());
            }
        }
        (cur_turn, msgs)
    } else {
        let initial_user_msg = if let Some(ref atts) = config.task_attachments {
            if !atts.is_empty() {
                ChatMessage::user_with_images(initial_user_prompt, atts.clone())
            } else {
                ChatMessage::user(initial_user_prompt)
            }
        } else {
            ChatMessage::user(initial_user_prompt)
        };
        (
            0,
            vec![
                ChatMessage::system(system_prompt.clone()).with_cache_control(crate::llm::CacheControl::ephemeral()),
                initial_user_msg.with_cache_control(crate::llm::CacheControl::ephemeral()),
            ],
        )
    };

    // 4. Sanitize any unclosed tool calls (e.g. from an aborted turn mid-tool execution)
    sanitize_unclosed_tool_calls(&mut messages);

    // 5. Inject steer message if present
    if let Some(ref steer) = config.steer_prompt {
        let steer_trimmed = steer.trim();
        if !steer_trimmed.is_empty() {
            let formatted_steer = format!(
                "### [УКАЗАНИЕ ПОЛЬЗОВАТЕЛЯ / USER STEER]\n{}",
                steer_trimmed
            );

            // Avoid duplicate identical steer if already at the very end of messages
            let already_appended = messages.last().map(|m| {
                m.role == Role::User && m.content.as_deref().unwrap_or("").contains(steer_trimmed)
            }).unwrap_or(false);

            if !already_appended {
                if let Some(ref imgs) = config.steer_images {
                    if !imgs.is_empty() {
                        messages.push(ChatMessage::user_with_images(formatted_steer, imgs.clone()));
                    } else {
                        messages.push(ChatMessage::user(formatted_steer));
                    }
                } else {
                    messages.push(ChatMessage::user(formatted_steer));
                }
            } else if let Some(ref imgs) = config.steer_images {
                if !imgs.is_empty() {
                    if let Some(last_msg) = messages.last_mut() {
                        if last_msg.role == Role::User {
                            let text = last_msg.text_content().unwrap_or(&formatted_steer).to_string();
                            *last_msg = ChatMessage::user_with_images(text, imgs.clone());
                        }
                    }
                }
            }

            // Persist steer message to SQLite dialog_items ("type": "steer") so load_task_dialog also retains it
            let already_in_db = if let Ok(items) = crate::db::load_dialog_items(&config.repo_root, &config.task_id) {
                items.iter().rev().take(3).any(|it| {
                    (it.item_type == "steer" || it.item_type == "user")
                        && it.content.as_deref().map(str::trim) == Some(steer_trimmed)
                })
            } else {
                false
            };

            if !already_in_db {
                let new_item = crate::db::ActivityItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    item_type: "steer".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    content: Some(steer_trimmed.to_string()),
                    tool: None,
                    duration_ms: None,
                    images: config.steer_images.clone(),
                };
                let _ = crate::db::append_dialog_items(&config.repo_root, &config.task_id, std::slice::from_ref(&new_item));
                if let Some(app) = config.app_handle.as_ref() {
                    let _ = app.emit("task:dialog", serde_json::json!({
                        "taskId": config.task_id,
                        "item": new_item,
                    }));
                }
            }
        }
    }

    // 5b. Guard against Gemini HTTP 400 "Requests ending with a model turn are not supported"
    // when resuming a task without a steer prompt.
    if messages.last().map(|m| m.role == Role::Assistant).unwrap_or(false) {
        let stage_name = if let Ok(pipe) = crate::config::load_pipeline_config(&config.repo_root).await {
            pipe.columns
                .into_iter()
                .find(|c| c.id == config.stage_id)
                .map(|c| c.title)
                .unwrap_or_else(|| config.stage_id.clone())
        } else {
            config.stage_id.clone()
        };

        let resume_msg = format!(
            "### [ВОЗОБНОВЛЕНИЕ ЗАДАЧИ / TASK RESUMED]
Задача перемещена на этап '{}' (роль: {}). Жди указа Человека.",
            stage_name, config.role
        );
        messages.push(ChatMessage::user(resume_msg.clone()));

        // Persist to SQLite dialog_items so history retains it
        let resume_item = crate::db::ActivityItem {
            id: uuid::Uuid::new_v4().to_string(),
            item_type: "steer".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            content: Some(resume_msg),
            tool: None,
            duration_ms: None,
            images: None,
        };
        let _ = crate::db::append_dialog_items(&config.repo_root, &config.task_id, std::slice::from_ref(&resume_item));
        if let Some(app) = config.app_handle.as_ref() {
            let _ = app.emit("task:dialog", serde_json::json!({
                "taskId": config.task_id,
                "item": resume_item,
            }));
        }
    }

    // 6. Synchronize active context state with AppState
    if let Some(ref app) = config.app_handle {
        if let Some(state) = app.try_state::<crate::commands::AppState>() {
            if let Ok(mut map) = state.agent_contexts.lock() {
                map.insert(
                    config.task_id.clone(),
                    std::sync::Arc::new(std::sync::Mutex::new(compactor::AgentContextState {
                        current_turn: starting_turn,
                        messages: messages.clone(),
                    })),
                );
            }
        }
    }

    let llm_client = LlmClient::new();

    // Безлимитный ReAct-цикл (лимит шагов убран по требованию): выход только
    // через терминальный ответ модели (текст без тул-коллов), abort_flag
    // (кнопка «Остановить») или ошибку LLM-стрима.
    let mut turn: usize = starting_turn;
    loop {
        turn += 1;
        if abort_flag.load(Ordering::SeqCst) {
            sync_context_state(&config.app_handle, &config.task_id, turn, &messages);
            event_sink(AgentEvent::Aborted);
            bail!("Agent execution aborted by user");
        }

        event_sink(AgentEvent::TurnStart { turn });

        // Synchronize in-memory agent context state with AppState
        if let Some(ref app) = config.app_handle {
            if let Some(state) = app.try_state::<crate::commands::AppState>() {
                if let Ok(map) = state.agent_contexts.lock() {
                    if let Some(cs) = map.get(&config.task_id) {
                        if let Ok(guard) = cs.lock() {
                            // If external manual compaction condensed messages, adopt them
                            if guard.messages.len() < messages.len() && !guard.messages.is_empty() {
                                eprintln!(
                                    "CVH [compactor]: Adopting externally compacted messages (from {} to {}) for task {}",
                                    messages.len(), guard.messages.len(), config.task_id
                                );
                                messages = guard.messages.clone();
                            }
                        }
                    }
                }
            }
        }

        // State-Preserving Dynamic Context Compactor:
        // Reads active agent persona's context_window (strictly resolved from agent setting, default 128,000).
        // Triggers compaction strictly when current estimated tokens reach or exceed context_window (100%).
        let context_window = config.context_window.unwrap_or(128_000) as usize;
        let compaction_threshold = context_window;
        let current_tokens = compactor::estimate_tokens(&messages);

        let should_compact = current_tokens >= compaction_threshold;
        if should_compact {
            let target_window = (context_window * 3) / 4;
            let compaction_res = compactor::compact_conversation_to_target(&mut messages, target_window, None);
            if compaction_res.saved_chars > 0 || compactor::estimate_tokens(&messages) < current_tokens {
                let tokens_after = compactor::estimate_tokens(&messages);
                let saved_tokens = compaction_res.estimated_saved_tokens;
                let stdout_msg = format!(
                    "Контекст успешно сжат. Сохранено {} токенов. Контекст сокращен с {} до {} токенов.",
                    saved_tokens, current_tokens, tokens_after
                );
                let tool_item_id = format!("compact_context-{}-{}", config.task_id, chrono::Utc::now().timestamp_millis());
                let tool_act_item = crate::db::ActivityItem {
                    id: tool_item_id.clone(),
                    item_type: "tool".to_string(),
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    content: None,
                    tool: Some(serde_json::json!({
                        "id": &tool_item_id,
                        "toolName": "compact_context",
                        "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
                        "durationMs": 0,
                        "status": "success",
                        "params": { "taskId": &config.task_id },
                        "stdout": &stdout_msg,
                    })),
                    duration_ms: Some(0.0),
                    images: None,
                };

                // If app_handle is present, persist to SQLite dialog_items and emit tool events
                if let Some(ref app) = config.app_handle {
                    let summary_text = if !compaction_res.preserved_state_summary.is_empty() {
                        compaction_res.preserved_state_summary.clone()
                    } else {
                        "Dynamic context compaction: older turns condensed to preserve context window.".to_string()
                    };
                    let compacted_item = crate::db::ActivityItem {
                        id: format!("compacted-{}-{}", config.task_id, chrono::Utc::now().timestamp_millis()),
                        item_type: "compacted".to_string(),
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        content: Some(summary_text),
                        tool: Some(serde_json::json!({
                            "compactedCount": compaction_res.evaporated_tools_count,
                            "savedTokens": saved_tokens,
                            "tokensBefore": current_tokens,
                            "tokensAfter": tokens_after,
                            "autoCompacted": true,
                            "contextWindow": context_window,
                        })),
                        duration_ms: None,
                        images: None,
                    };

                    let existing_items = crate::db::load_dialog_items(&config.repo_root, &config.task_id).unwrap_or_default();
                    let has_summary = messages.iter().any(|m| compactor::is_compacted_summary(&m.role, m.content.as_deref().unwrap_or("")));
                    let non_recent_overhead = if has_summary { 3 } else { 2 };
                    let recent_count = messages.len().saturating_sub(non_recent_overhead);
                    let archive_count = existing_items.len().saturating_sub(recent_count);
                    let ids_to_archive: Vec<String> = existing_items[..archive_count].iter().map(|it| it.id.clone()).collect();

                    let _ = crate::db::persist_compacted_dialog(&config.repo_root, &config.task_id, &compacted_item, Some(&tool_act_item), &ids_to_archive);
                    let _ = app.emit(
                        "agent:tool_call",
                        serde_json::json!({
                            "taskId": &config.task_id,
                            "id": &tool_item_id,
                            "name": "compact_context",
                            "arguments": serde_json::json!({ "taskId": &config.task_id }).to_string(),
                        }),
                    );
                    let _ = app.emit(
                        "agent:tool_result",
                        serde_json::json!({
                            "taskId": &config.task_id,
                            "id": &tool_item_id,
                            "name": "compact_context",
                            "result": &stdout_msg,
                            "success": true,
                        }),
                    );
                    let _ = app.emit("dialog:updated", serde_json::json!({ "taskId": config.task_id }));

                    if let Some(state) = app.try_state::<crate::commands::AppState>() {
                        if let Ok(map) = state.agent_contexts.lock() {
                            if let Some(cs) = map.get(&config.task_id) {
                                if let Ok(mut guard) = cs.lock() {
                                    guard.messages = messages.clone();
                                }
                            }
                        }
                    }
                }

                event_sink(AgentEvent::Compacted {
                    saved_chars: compaction_res.saved_chars,
                    estimated_saved_tokens: compaction_res.estimated_saved_tokens,
                    evaporated_count: compaction_res.evaporated_tools_count,
                    tokens_before: current_tokens,
                    tokens_after,
                });
            }
        }

        // Prompt-cache breakpoints: frozen on stable anchors (system + initial user,
        // set once at init). Do NOT retag the last message each turn: for the
        // OpenAI-compatible Gemini path the proxy strips `cache_control` before
        // upstream (no cache effect, only request-byte churn), and unbounded
        // breakpoints exceed the 4-breakpoint budget. New suffix messages stay
        // untagged so already-sent prefix bytes never change.

        // Cache-debug: hash/len of the exact serialized prefix about to be sent.
        // No prompt content is logged, only lengths + hash.
        let (prefix_len, prefix_hash) = {
            let msg_json = serde_json::to_string(&messages).unwrap_or_default();
            let tools_json = serde_json::to_string(&active_tools).unwrap_or_default();
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(&msg_json, &mut hasher);
            std::hash::Hash::hash(&tools_json, &mut hasher);
            (
                msg_json.len() + tools_json.len(),
                std::hash::Hasher::finish(&hasher),
            )
        };
        eprintln!(
            "CVH [cache] turn={} model={} msgs={} tools={} prefix_len={} prefix_hash={:016x}",
            turn,
            config.model,
            messages.len(),
            active_tools.len(),
            prefix_len,
            prefix_hash
        );

        // Safety guard: guarantee request never ends with an Assistant turn (Gemini HTTP 400).
        if messages.last().map(|m| m.role == Role::Assistant).unwrap_or(false) {
            let stage_name = if let Ok(pipe) = crate::config::load_pipeline_config(&config.repo_root).await {
                pipe.columns
                    .into_iter()
                    .find(|c| c.id == config.stage_id)
                    .map(|c| c.title)
                    .unwrap_or_else(|| config.stage_id.clone())
            } else {
                config.stage_id.clone()
            };
            let resume_msg = format!(
                "### [ВОЗОБНОВЛЕНИЕ ЗАДАЧИ / TASK RESUMED]
Задача перемещена на этап '{}' (роль: {}). Жди указа Человека.",
                stage_name, config.role
            );
            messages.push(ChatMessage::user(resume_msg));
        }

        let tools: Option<Vec<crate::llm::ToolDefinition>> = if active_tools.is_empty() {
            None
        } else {
            Some(active_tools.clone())
        };

        // Pre-Flight Hard Ceiling Guard:
        // Guarantees that request tokens strictly do not exceed the upstream model ceiling
        // (default 1,000,000 or context_window), preventing unrecoverable HTTP 400 Bad Request.
        let mut req_tokens = compactor::estimate_request_tokens(&messages, tools.as_deref());
        let model_ceiling = 1_000_000usize.min(context_window);

        if req_tokens >= model_ceiling {
            eprintln!(
                "CVH [agent]: Pre-flight token guard: req_tokens ({}) >= model_ceiling ({}). Triggering emergency multi-tier compaction.",
                req_tokens, model_ceiling
            );
            let emergency_target = (model_ceiling * 7) / 10;
            compactor::compact_conversation_to_target(&mut messages, emergency_target, None);
            req_tokens = compactor::estimate_request_tokens(&messages, tools.as_deref());

            if req_tokens >= model_ceiling {
                let err_msg = format!(
                    "Pre-flight token guard: context size ({} tokens) exceeds model ceiling ({} tokens). Request aborted to prevent HTTP 400.",
                    req_tokens, model_ceiling
                );
                eprintln!("CVH [agent]: {}", err_msg);
                event_sink(AgentEvent::Error(err_msg.clone()));
                bail!("{}", err_msg);
            }
        }

        let system_tokens = compactor::estimate_text_tokens(&system_prompt) as u64;
        let tool_tokens = compactor::estimate_tools_tokens(&tools) as u64;

        let req = OpenAIChatRequest {
            model: config.model.clone(),
            messages: messages.clone(),
            temperature: config.temperature,
            stream: true,
            // Empty active_tools (everything disabled) is a valid case: send
            // tools=None so the field is omitted from the OpenAI-compatible
            // payload (stream_openai only sorts Some) and the model answers
            // with plain text. Sending `"tools":[]` is rejected by some proxies.
            tools: tools.clone(),
            reasoning_effort: config.reasoning_effort.clone(),
            stream_options: Some(crate::llm::StreamOptions { include_usage: true }),
        };

        // Log raw request for complete transparency & debugging (§ llm_debug.log)
        {
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            let mut req_log = format!(
                "[{}] [{}] Turn {} Request:
Model: {}
Messages count: {}
",
                ts, config.task_id, turn, config.model, messages.len()
            );
            if let Some(first) = messages.first() {
                if first.role == Role::System {
                    req_log.push_str(&format!("System prompt:
{}
", first.content.as_deref().unwrap_or("")));
                }
            }
            let tool_names: Vec<String> = active_tools.iter().map(|t| t.function.name.clone()).collect();
            req_log.push_str(&format!("Tool calls requested ({}): {:?}

", tool_names.len(), tool_names));
            append_llm_debug_log(&config.repo_root, &req_log).await;
        }

        let (tx, mut rx) = mpsc::channel::<StreamChunk>(100);
        let client = llm_client.clone();
        let base_url = config.base_url.clone();
        let api_key = config.api_key.clone();

        tokio::spawn(async move {
            if let Err(e) = client.stream_openai(&base_url, api_key.as_deref(), req, tx.clone()).await {
                let err_msg = format!("Failed to dispatch LLM request to {}: {}", base_url, e);
                eprintln!("CVH [agent]: {}", err_msg);
                let _ = tx.send(StreamChunk::Error(err_msg)).await;
            }
        });

        let mut turn_content = String::new();
        let mut turn_thoughts = String::new();
        let mut turn_tool_calls: Vec<ToolCall> = Vec::new();
        let mut thinking_parser = ThinkingStreamParser::new();
        let mut received_any_chunk = false;
        let mut received_usage = false;
        let turn_start_instant = std::time::Instant::now();
        let mut first_token_instant: Option<std::time::Instant> = None;
        let mut last_token_instant: Option<std::time::Instant> = None;
        let mut turn_streamed_chars: usize = 0;

        while let Some(chunk) = rx.recv().await {
            received_any_chunk = true;
            if abort_flag.load(Ordering::SeqCst) {
                sync_context_state(&config.app_handle, &config.task_id, turn, &messages);
                event_sink(AgentEvent::Aborted);
                bail!("Agent execution aborted by user");
            }

            match chunk {
                StreamChunk::FirstChunk => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                }
                StreamChunk::Token(token) => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += token.len();
                    let parsed = thinking_parser.push(&token);
                    for (is_thought, text) in parsed {
                        if is_thought {
                            turn_thoughts.push_str(&text);
                            event_sink(AgentEvent::Thought(text));
                        } else {
                            turn_content.push_str(&text);
                            event_sink(AgentEvent::Token(text));
                        }
                    }
                }
                StreamChunk::Output(output) => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += output.len();
                    event_sink(AgentEvent::Output(output));
                }
                StreamChunk::Thought(thought) => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += thought.len();
                    turn_thoughts.push_str(&thought);
                    event_sink(AgentEvent::Thought(thought));
                }
                StreamChunk::ToolCall { id, name, arguments } => {
                    if first_token_instant.is_none() {
                        first_token_instant = Some(std::time::Instant::now());
                    }
                    last_token_instant = Some(std::time::Instant::now());
                    turn_streamed_chars += name.len() + arguments.len();
                    event_sink(AgentEvent::ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    });
                    turn_tool_calls.push(ToolCall {
                        id,
                        call_type: "function".to_string(),
                        function: crate::llm::FunctionCall { name, arguments },
                    });
                }
                StreamChunk::Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens,
                    cache_creation_tokens,
                } => {
                    received_usage = true;
                    let total_input = input_tokens + cache_read_tokens;
                    let cache_hit_rate = if total_input > 0 {
                        (cache_read_tokens as f32 / total_input as f32) * 100.0
                    } else {
                        0.0
                    };
                    let start_time = first_token_instant.unwrap_or(turn_start_instant);
                    let end_time = last_token_instant.unwrap_or_else(std::time::Instant::now);
                    let duration_secs = end_time.duration_since(start_time).as_secs_f64();
                    let duration_ms = duration_secs * 1000.0;
                    let ttft_ms = first_token_instant
                        .map(|t| t.duration_since(turn_start_instant).as_secs_f64() * 1000.0)
                        .unwrap_or(0.0);
                    let tps = if duration_secs > 0.05 && output_tokens > 0 {
                        (output_tokens as f64 / duration_secs) as f32
                    } else {
                        0.0
                    };
                    eprintln!(
                        "CVH [telemetry] turn={} model={} input={} output={} cache_read={} cache_creation={} hit_rate={:.1}% duration={:.2}s tps={:.1} ttft={:.1}ms prefix_hash={:016x}",
                        turn,
                        config.model,
                        input_tokens,
                        output_tokens,
                        cache_read_tokens,
                        cache_creation_tokens,
                        cache_hit_rate,
                        duration_secs,
                        tps,
                        ttft_ms,
                        prefix_hash
                    );
                    event_sink(AgentEvent::Telemetry {
                        input_tokens,
                        output_tokens,
                        cache_read_tokens,
                        cache_creation_tokens,
                        cache_hit_rate,
                        duration_ms,
                        tps,
                        ttft_ms,
                        system_tokens,
                        tool_tokens,
                    });
                }
                StreamChunk::Done => break,
                StreamChunk::Error(err) => {
                    eprintln!("CVH [agent]: LLM stream error: {}", err);
                    event_sink(AgentEvent::Error(err.clone()));
                    bail!("LLM streaming error: {}", err);
                }
            }
        }

        // Fallback telemetry if provider did not emit a usage chunk:
        if !received_usage && received_any_chunk {
            let start_time = first_token_instant.unwrap_or(turn_start_instant);
            let end_time = last_token_instant.unwrap_or_else(std::time::Instant::now);
            let duration_secs = end_time.duration_since(start_time).as_secs_f64();
            let duration_ms = duration_secs * 1000.0;
            let ttft_ms = first_token_instant
                .map(|t| t.duration_since(turn_start_instant).as_secs_f64() * 1000.0)
                .unwrap_or(0.0);
            let output_tokens = (compactor::estimate_text_tokens(&turn_content) as u64)
                .max((turn_streamed_chars as f64 / 3.8).ceil() as u64);
            let input_tokens = compactor::estimate_request_tokens(&messages, tools.as_deref()) as u64;
            let tps = if duration_secs > 0.05 && output_tokens > 0 {
                (output_tokens as f64 / duration_secs) as f32
            } else {
                0.0
            };
            eprintln!(
                "CVH [telemetry:fallback] turn={} model={} input={} output={} duration={:.2}s tps={:.1} ttft={:.1}ms",
                turn,
                config.model,
                input_tokens,
                output_tokens,
                duration_secs,
                tps,
                ttft_ms
            );
            event_sink(AgentEvent::Telemetry {
                input_tokens,
                output_tokens,
                cache_read_tokens: 0,
                cache_creation_tokens: 0,
                cache_hit_rate: 0.0,
                duration_ms,
                tps,
                ttft_ms,
                system_tokens,
                tool_tokens,
            });
        }

        if !received_any_chunk && !abort_flag.load(Ordering::SeqCst) {
            let err_msg = format!(
                "No stream chunks received from LLM endpoint ({}) - check proxy connection or port",
                config.base_url
            );
            eprintln!("CVH [agent]: {}", err_msg);
            event_sink(AgentEvent::Error(err_msg.clone()));
            bail!("{}", err_msg);
        }

        // Flush any remaining characters from the thinking parser
        let flushed = thinking_parser.flush();
        for (is_thought, text) in flushed {
            if is_thought {
                turn_thoughts.push_str(&text);
                event_sink(AgentEvent::Thought(text));
            } else {
                turn_content.push_str(&text);
                event_sink(AgentEvent::Token(text));
            }
        }

        // Log raw response for complete transparency & debugging (§ llm_debug.log)
        {
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            let mut resp_log = format!(
                "[{}] [{}] Turn {} Response:
",
                ts, config.task_id, turn
            );
            if !turn_thoughts.trim().is_empty() {
                resp_log.push_str(&format!("Thoughts:
{}
", turn_thoughts.trim()));
            }
            if !turn_content.trim().is_empty() {
                resp_log.push_str(&format!("Content:
{}
", turn_content.trim()));
            }
            if !turn_tool_calls.is_empty() {
                resp_log.push_str(&format!("Tool calls received ({}):
", turn_tool_calls.len()));
                for tc in &turn_tool_calls {
                    resp_log.push_str(&format!("  - {} (id: {}): {}
", tc.function.name, tc.id, tc.function.arguments));
                }
            } else {
                resp_log.push_str("Tool calls received: None (terminal turn)
");
            }
            resp_log.push_str("
--------------------------------------------------------------------------------

");
            append_llm_debug_log(&config.repo_root, &resp_log).await;
        }

        // If no tool calls, terminal response reached
        if turn_tool_calls.is_empty() {
            if !turn_content.is_empty() {
                messages.push(ChatMessage::assistant(Some(turn_content.clone()), None));
            }
            sync_context_state(&config.app_handle, &config.task_id, turn, &messages);
            event_sink(AgentEvent::Finished {
                final_response: turn_content.clone(),
            });
            return Ok(turn_content);
        }

        // Append assistant turn message
        messages.push(ChatMessage::assistant(
            if turn_content.is_empty() { None } else { Some(turn_content) },
            Some(turn_tool_calls.clone()),
        ));

        let tool_ctx = ToolContext {
            repo_root: &config.repo_root,
            worktree_dir: &config.worktree_path,
            stage_id: Some(&config.stage_id),
            role: Some(&config.role),
            abort_handle: Arc::clone(&abort_flag),
            tool_policy: &tool_policy,
            task_id: Some(&config.task_id),
            agent_allowed_tools: agent_allowed.as_deref(),
            app_handle: config.app_handle.clone(),
            base_url: Some(&config.base_url),
            api_key: config.api_key.as_deref(),
            model: Some(&config.model),
            temperature: config.temperature,
            reasoning_effort: config.reasoning_effort.as_deref(),
            call_id: None,
        };

        let mut executed_subagents_in_turn: Vec<ExecutedSubagentCall> = Vec::new();

        // Execute each tool call and record response
        for tc in turn_tool_calls {
            if abort_flag.load(Ordering::SeqCst) {
                sanitize_unclosed_tool_calls(&mut messages);
                sync_context_state(&config.app_handle, &config.task_id, turn, &messages);
                event_sink(AgentEvent::Aborted);
                bail!("Agent execution aborted by user");
            }

            let parsed_args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| serde_json::json!({}));

            let norm_name = canonical_tool_name(&tc.function.name);
            let is_sub = norm_name == "subagent" || norm_name == "subagent_bg";

            // In-turn Subagent Deduplication (§ Directive 4):
            // If the model in a single turn emits multiple subagent calls with identical/virtually identical
            // prompt and role, return the result of the already started/completed agent instead of spawning duplicate work.
            if is_sub {
                let role = parsed_args
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("general")
                    .trim();
                let prompt = parsed_args
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();

                if let Some(prev) = executed_subagents_in_turn.iter().find(|p| {
                    p.role.eq_ignore_ascii_case(role) && are_subagent_prompts_duplicate(&p.prompt, prompt)
                }) {
                    eprintln!(
                        "CVH [agent]: Deduplicated subagent call '{}' (role: '{}') in turn {} (matches earlier call '{}' with role '{}')",
                        tc.id, role, turn, prev.id, prev.role
                    );
                    event_sink(AgentEvent::ToolResult {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        result: prev.result.clone(),
                        success: true,
                    });
                    messages.push(ChatMessage::tool_response(tc.id, prev.result.clone()));
                    continue;
                }
            }

            let mut call_tool_ctx = tool_ctx.clone();
            call_tool_ctx.call_id = Some(&tc.id);

            let exec_result = execute_tool(
                &call_tool_ctx,
                &tc.function.name,
                &parsed_args,
            )
            .await;

            if abort_flag.load(Ordering::SeqCst) {
                event_sink(AgentEvent::ToolResult {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    result: "Прервано пользователем".to_string(),
                    success: false,
                });
                sanitize_unclosed_tool_calls(&mut messages);
                sync_context_state(&config.app_handle, &config.task_id, turn, &messages);
                event_sink(AgentEvent::Aborted);
                bail!("Agent execution aborted by user");
            }

            if tc.function.name == "todo_write" && exec_result.is_ok() {
                if let Some(raw_todos) = parsed_args.get("todos").and_then(|v| v.as_array()) {
                    let mut todos_vec = Vec::new();
                    for (idx, item) in raw_todos.iter().enumerate() {
                        let content = item
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        if content.is_empty() {
                            continue;
                        }
                        let raw_status = item
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("pending")
                            .trim()
                            .to_lowercase();
                        let status = match raw_status.as_str() {
                            "completed" | "done" => "completed",
                            "in_progress" | "running" => "in_progress",
                            _ => "pending",
                        }
                        .to_string();
                        let id = item
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("todo-{}", idx + 1));
                        todos_vec.push(TodoItem { id, content, status });
                    }
                    event_sink(AgentEvent::TaskUpdated {
                        task_id: config.task_id.clone(),
                        todos: todos_vec,
                    });
                }
            }

            let (result_str, success) = match exec_result {
                Ok(out) => (out, true),
                Err(e) => (format!("Error executing {}: {:#}", tc.function.name, e), false),
            };

            if is_sub && success {
                let role = parsed_args
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("general")
                    .trim()
                    .to_string();
                let prompt = parsed_args
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                executed_subagents_in_turn.push(ExecutedSubagentCall {
                    id: tc.id.clone(),
                    tool_name: tc.function.name.clone(),
                    role,
                    prompt,
                    result: result_str.clone(),
                });
            }

            event_sink(AgentEvent::ToolResult {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                result: result_str.clone(),
                success,
            });

            messages.push(ChatMessage::tool_response(tc.id, result_str.clone()));

            // Stage transition termination (§ 2.27 & § 1):
            // When any executed tool call is "advance_task", "move_task", or "reject_task" (or aliases):
            // If the transition SUCCEEDED (success == true), the agent has handed over or finalized the task.
            // Trip abort_flag, emit AgentEvent::Finished, break the ReAct loop, and return Ok(result_str).
            // Under NO circumstances send the result back to the LLM for an extra turn or allow duplicate advance_task calls.
            // However, if the transition FAILED (!success, e.g. target stage not found), do NOT terminate the loop.
            // Let the error tool_response return to the LLM so the model can inspect available stages and self-correct!
            let norm_name = tc.function.name.trim().to_lowercase();
            let is_stage_transition = matches!(
                norm_name.as_str(),
                "advance_task" | "move_task" | "reject_task" | "move_task_stage" | "advance_stage" | "next_stage"
            ) || canonical_tool_name(&norm_name) == "advance_task";

            if is_stage_transition && success {
                // Immediately trip abort_flag to prevent any subsequent ticks or parallel operations
                abort_flag.store(true, Ordering::SeqCst);

                // Synchronize messages to in-memory context state before exiting
                sync_context_state(&config.app_handle, &config.task_id, turn, &messages);

                let display_summary = serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                    .ok()
                    .and_then(|val| {
                        val.get("summary")
                            .or_else(|| val.get("reason"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.trim().to_string())
                    })
                    .filter(|s| !s.is_empty());

                let final_resp = display_summary.unwrap_or_else(|| result_str.clone());

                event_sink(AgentEvent::Finished {
                    final_response: final_resp,
                });
                return Ok(result_str);
            }
        }

        // Sync messages to in-memory context state
        sync_context_state(&config.app_handle, &config.task_id, turn, &messages);
    }

    // Недостижимо: цикл выше бесконечен и выходит только через
    // return (терминальный ответ), bail! (abort/ошибка) внутри.
}

#[cfg(test)]
mod tests;
