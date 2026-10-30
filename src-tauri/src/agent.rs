pub mod compactor;
pub use compactor::*;
pub mod tool_schemas;
pub use tool_schemas::*;
pub mod text_match;
pub use text_match::*;
pub mod telemetry;
pub use telemetry::*;
pub mod sanitizer;
pub use sanitizer::*;
pub mod tools;
pub use tools::*;
pub mod react_engine;
pub use react_engine::*;
pub mod session_registry;
pub use session_registry::*;

use crate::config::{
    agent_allows_tool, is_tool_allowed, load_agent_setting, load_tool_policies,
    ToolPolicyConfig,
};
use crate::db::ActivityItem;
use crate::hooks::kill_process_tree;
use crate::llm::{ChatMessage, LlmClient, OpenAIChatRequest, Role, StreamChunk, ToolDefinition};
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    #[serde(rename = "stageEnteredAt", alias = "stage_entered_at", default)]
    pub stage_entered_at: Option<i64>,
    #[serde(rename = "completedAt", alias = "completed_at", default)]
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub position: Option<f64>,
    #[serde(rename = "hookStates", alias = "hook_states", default)]
    pub hook_states: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Idle,
    Running,
    Paused,
    WaitingReview,
    WaitingSubtask,
    Blocked,
    Done,
    #[serde(untagged)]
    Custom(String),
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Idle => "idle",
            TaskStatus::Running => "running",
            TaskStatus::Paused => "paused",
            TaskStatus::WaitingReview => "waiting_review",
            TaskStatus::WaitingSubtask => "waiting_subtask",
            TaskStatus::Blocked => "blocked",
            TaskStatus::Done => "done",
            TaskStatus::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for TaskStatus {
    fn from(s: &str) -> Self {
        match s {
            "idle" => TaskStatus::Idle,
            "running" => TaskStatus::Running,
            "paused" => TaskStatus::Paused,
            "waiting_review" => TaskStatus::WaitingReview,
            "waiting_subtask" => TaskStatus::WaitingSubtask,
            "blocked" => TaskStatus::Blocked,
            "done" => TaskStatus::Done,
            other => TaskStatus::Custom(other.to_string()),
        }
    }
}

impl From<String> for TaskStatus {
    fn from(s: String) -> Self {
        TaskStatus::from(s.as_str())
    }
}

impl Task {
    pub fn get_status(&self) -> TaskStatus {
        self.status.as_deref().map(TaskStatus::from).unwrap_or(TaskStatus::Idle)
    }

    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = Some(status.as_str().to_string());
    }

    pub fn is_done(&self) -> bool {
        self.get_status() == TaskStatus::Done
    }

    pub fn is_running(&self) -> bool {
        self.get_status() == TaskStatus::Running
    }

    pub fn is_waiting_subtask(&self) -> bool {
        self.get_status() == TaskStatus::WaitingSubtask
    }

    /// Returns the strongly-typed domain `TaskState` representation of this task.
    pub fn state(&self) -> crate::domain::TaskState {
        crate::domain::TaskState::from_task(self)
    }

    /// Sets the task state using the domain `TaskState` state machine,
    /// synchronizing status, model, subtask reference, and timestamp.
    pub fn set_state(&mut self, state: crate::domain::TaskState) {
        state.apply_to_task(self);
    }
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
- Direct Coordinate Navigation: use exact target lines and files from planner summary; call `load_rule("backend_map")` or `load_rule("frontend_map")` at Turn 1 to locate exact files. Never read files sequentially from line 1.
- Anti-Micro-Reading Mandate: never call read_file in 20-50 line increments to scroll files. Use grep for search, read targeted chunks of 100-300 lines.
- Mandatory Commit Pre-Flight: always commit working tree changes (`git add -A && git commit`) before calling advance_task.

Workflow (Explore -> Plan -> Code -> Verify -> Commit -> Handover):
1. Explore: call `load_rule("backend_map")` or `load_rule("frontend_map")` at Turn 1 to locate exact subsystems and files, or use planner coordinates.
2. Plan: formulate targeted checklist with `path:line` targets via `todo_write` (batched updates).
3. Code: execute surgical edits inside `worktrees/<task-slug>`.
4. Commit: commit changes to the feature branch before verification.
5. Verify with Ground Truth Sensors:
   - Backend/Rust: `cargo check` and `cargo clippy -- -D warnings` must exit 0 (if backend touched).
   - Frontend/TS: `npx tsc --noEmit` / `npm run build` must exit 0 (if frontend touched).
   - Mergeability: `git merge-tree --write-tree main HEAD` must exit 0.
6. Handover: advance to review with proof of passing sensors.

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

    // 1b. Per-agent rules: `.cvh/agents/<role>/rules/*.md` and global `.cvh/rules/*.md`, alphabetical.
    // Modular Rules (JIT On-Demand): If `load_rule` is active, decouple rule bodies from prompt to optimize tokens.
    let has_load_rule = resolved_tools.iter().any(|t| t == "load_rule");
    if has_load_rule {
        let available = crate::agent::tool_schemas::get_available_rules_with_descriptions(repo_root, Some(role));
        let mut rule_bullets = Vec::new();
        for (r_name, r_desc) in available {
            if let Some(desc) = r_desc {
                rule_bullets.push(format!("- `{}`: {}", r_name, desc));
            } else {
                rule_bullets.push(format!("- `{}`", r_name));
            }
        }
        let list_str = if rule_bullets.is_empty() {
            String::new()
        } else {
            format!("\n\nAvailable rules for dynamic loading:\n{}", rule_bullets.join("\n"))
        };
        sections.push(
            format!("# Modular Rules (JIT On-Demand)\nArchitectural standards, domain invariants, and tech guidelines are decoupled from the prompt to optimize tokens.\nUse the `load_rule` tool to dynamically load rules when working on specific topics or technologies.{list_str}"),
        );
    } else {
        let mut rules: Vec<(String, String)> = Vec::new();
        let mut rule_dirs = vec![crate::config::agent_rules_dir(repo_root, role)];
        let global_rules = crate::config::global_rules_dir(repo_root);
        if global_rules.is_dir() {
            rule_dirs.push(global_rules);
        }
        for rules_dir in rule_dirs {
            if rules_dir.is_dir() {
                if let Ok(mut entries) = tokio::fs::read_dir(&rules_dir).await {
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
                }
            }
        }
        rules.sort_by(|a, b| a.0.cmp(&b.0));
        rules.dedup_by(|a, b| a.0 == b.0);
        for (fname, content) in rules {
            sections.push(format!("## Rule: {}\n\n{}", fname, content.trim()));
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

// Standard tool definitions and modular rule schemas extracted to crate::agent::tool_schemas


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

// Text matching, CRLF normalization, and relaxed replacement routines extracted to crate::agent::text_match


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
    let _worktree_dir = ctx.worktree_dir;
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
        "read_file" => execute_read_file(ctx, args).await?,
        "write_file" => execute_write_file(ctx, args).await?,
        "edit_file" => execute_edit_file(ctx, args).await?,
        "glob" => execute_glob(ctx, args).await?,
        "grep" => execute_grep(ctx, args).await?,
        "execute_command" => execute_command(ctx, args).await?,
        "web_search" => execute_web_search(ctx, args).await?,
        "web_fetch" => execute_web_fetch(ctx, args).await?,
        "subagent" => execute_subagent(ctx, args).await?,
        "subagent_bg" => execute_subagent_bg(ctx, args).await?,
        "collect_subagent" => execute_collect_subagent(ctx, args).await?,
        "subagent_list" => execute_subagent_list(ctx, args).await?,
        "todo_write" => execute_todo_write(ctx, args).await?,
        "advance_task" | "move_task" | "move_task_stage" | "advance_stage" | "next_stage" => {
            execute_advance_task(ctx, args).await?
        }
        "reject_task" => execute_reject_task(ctx, args).await?,
        "question" => execute_question(ctx, args).await?,
        "create_task" => execute_create_task(ctx, args, false).await?,
        "create_tasks" => execute_create_task(ctx, args, true).await?,
        "complete_task" => execute_complete_task(ctx, args).await?,
        "list_tasks" => execute_list_tasks(ctx, args).await?,
        "get_task_history" => execute_get_task_history(ctx, args).await?,
        "load_rule" => execute_load_rule(ctx, args).await?,
        custom_tool => execute_custom_tool(ctx, custom_tool, args).await?,
    };

    if abort_handle.load(Ordering::SeqCst) {
        bail!("Operation aborted");
    }

    maybe_spill_output(repo_root, normalized_tool_name, raw_result).await
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

    let subagent_driver = SubagentDriver::new(SubagentDriverConfig {
        parent_task_id: parent_id.clone(),
        subagent_id: subagent_id.clone(),
        subagent_dialog_id: subagent_dialog_id.clone(),
        tool_call_id: tool_call_id.clone(),
        repo_root: ctx.repo_root.to_path_buf(),
        worktree_dir: ctx.worktree_dir.to_path_buf(),
        model: model.to_string(),
        app_handle: ctx.app_handle.clone(),
    });
    let subagent_engine = ReActEngine::new(subagent_driver);

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

        let turn_thought_id = format!("sub-th-{}-turn-{}", subagent_suffix, sub_turn);
        subagent_engine.driver.reset_turn_thoughts(turn_thought_id);

        let stream_opts = StreamConsumerOptions {
            model,
            turn: sub_turn,
            system_tokens,
            tool_tokens,
            fallback_input_tokens: compactor::estimate_request_tokens(&sub_messages, req_tools.as_deref()) as u64,
            prefix_hash: None,
            log_prefix: "subagent telemetry",
            subagent_id: Some(&subagent_id),
            emit_thoughts: true,
        };

        let stream_res = subagent_engine.consume_turn_stream(&mut rx, &ctx.abort_handle, &stream_opts).await;

        subagent_engine.driver.finalize_turn_thoughts();

        if let Some(err) = stream_res.error {
            sub_error = Some(err);
            break;
        }

        if stream_res.aborted || ctx.abort_handle.load(Ordering::SeqCst) {
            break;
        }

        let turn_content = stream_res.content;
        let turn_thoughts = stream_res.thoughts;
        let turn_tool_calls = stream_res.tool_calls;

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

        if let Some(ref app) = ctx.app_handle {
            if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                state_ref.session_registry.update_status(&subagent_id, crate::agent::session_registry::SessionStatus::Finished);
                if let Some(summary) = state_ref.session_registry.get_session(&subagent_id) {
                    let _ = app.emit("agent:session_status", &summary);
                }
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

        let synth_opts = StreamConsumerOptions {
            model,
            turn: 999,
            system_tokens: synth_system_tokens,
            tool_tokens: 0,
            fallback_input_tokens: compactor::estimate_request_tokens(&sub_messages, None) as u64,
            prefix_hash: None,
            log_prefix: "subagent synthesis telemetry",
            subagent_id: Some(&subagent_id),
            emit_thoughts: false,
        };

        let synth_res = subagent_engine.consume_turn_stream(&mut rx, &ctx.abort_handle, &synth_opts).await;
        let synth_content = synth_res.content;

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
            if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
                state_ref.session_registry.update_status(&subagent_id, crate::agent::session_registry::SessionStatus::Finished);
                if let Some(summary) = state_ref.session_registry.get_session(&subagent_id) {
                    let _ = app.emit("agent:session_status", &summary);
                }
            }
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
        if let Some(state_ref) = app.try_state::<crate::commands::AppState>() {
            state_ref.session_registry.update_status(&subagent_id, crate::agent::session_registry::SessionStatus::Finished);
            if let Some(summary) = state_ref.session_registry.get_session(&subagent_id) {
                let _ = app.emit("agent:session_status", &summary);
            }
        }
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

/// Mapping table for CP866 (IBM866 OEM Cyrillic) bytes 0x80..=0xFF to Unicode char.
pub const CP866_TABLE: [char; 128] = [
    'А', 'Б', 'В', 'Г', 'Д', 'Е', 'Ж', 'З', 'И', 'Й', 'К', 'Л', 'М', 'Н', 'О', 'П',
    'Р', 'С', 'Т', 'У', 'Ф', 'Х', 'Ц', 'Ч', 'Ш', 'Щ', 'Ъ', 'Ы', 'Ь', 'Э', 'Ю', 'Я',
    'а', 'б', 'в', 'г', 'д', 'е', 'ж', 'з', 'и', 'й', 'к', 'л', 'м', 'н', 'о', 'п',
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐',
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀',
    'р', 'с', 'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'ъ', 'ы', 'ь', 'э', 'ю', 'я',
    'Ё', 'ё', 'Є', 'є', 'Ї', 'ї', 'Ў', 'ў', '°', '∙', '·', '√', '№', '¤', '■', '\u{00A0}',
];

/// Decodes CP866 (OEM code page 866 used by Russian Windows cmd.exe) bytes into valid UTF-8.
pub fn decode_cp866(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        if b < 0x80 {
            out.push(b as char);
        } else {
            out.push(CP866_TABLE[(b - 0x80) as usize]);
        }
    }
    out
}

/// Decodes command output bytes (stdout/stderr) with resilient fallback for CP866 on Russian Windows cmd.exe.
/// If `String::from_utf8(buf)` fails or contains replacement characters (`\u{FFFD}`), decodes using CP866.
pub fn decode_output_bytes(buf: &[u8]) -> String {
    match String::from_utf8(buf.to_vec()) {
        Ok(s) => {
            if s.contains('\u{FFFD}') {
                decode_cp866(buf)
            } else {
                s
            }
        }
        Err(_) => decode_cp866(buf),
    }
}

/// Resolves standard runtime locations and enriches Windows PATH if missing.
pub fn build_enriched_windows_paths(current_path: &std::ffi::OsStr) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::env::split_paths(current_path).collect();
    let mut candidates = Vec::new();

    if let Ok(appdata) = std::env::var("APPDATA") {
        if !appdata.trim().is_empty() {
            candidates.push(PathBuf::from(appdata).join("npm"));
        }
    }
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        if !localappdata.trim().is_empty() {
            candidates.push(PathBuf::from(localappdata).join("pnpm"));
        }
    }
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        if !userprofile.trim().is_empty() {
            candidates.push(PathBuf::from(userprofile).join(".cargo").join("bin"));
        }
    }
    let program_files = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    if !program_files.trim().is_empty() {
        candidates.push(PathBuf::from(program_files).join("nodejs"));
    }

    for candidate in candidates {
        let candidate_str = candidate.to_string_lossy();
        let already_present = paths.iter().any(|p| {
            p.to_string_lossy().eq_ignore_ascii_case(&candidate_str)
        });
        if !already_present {
            paths.push(candidate);
        }
    }
    paths
}

#[cfg(target_os = "windows")]
pub fn enrich_windows_path(cmd: &mut Command) {
    let current_path = std::env::var_os("PATH").unwrap_or_default();
    let paths = build_enriched_windows_paths(&current_path);
    if let Ok(new_path) = std::env::join_paths(paths) {
        cmd.env("PATH", new_path);
    }
}

pub async fn run_scoped_command(
    cwd: &Path,
    cmd_str: &str,
    timeout: Duration,
    abort_flag: Arc<AtomicBool>,
    repo_root: Option<&Path>,
) -> Result<crate::hooks::HookExecutionResult> {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd.exe");
        c.raw_arg("/D");
        c.raw_arg("/S");
        c.raw_arg("/C");
        c.raw_arg(format!("\"chcp 65001 >nul && {}\"", cmd_str));
        enrich_windows_path(&mut c);
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

    // Inject shared CARGO_TARGET_DIR to prevent redundant cold compilations in worktrees
    if let Ok(existing_target) = std::env::var("CARGO_TARGET_DIR") {
        if !existing_target.trim().is_empty() {
            cmd.env("CARGO_TARGET_DIR", existing_target);
        }
    } else if let Some(root) = repo_root {
        cmd.env("CARGO_TARGET_DIR", root.join("src-tauri").join("target"));
    } else {
        let abs_cwd = if cwd.is_absolute() {
            cwd.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(cwd)
        };
        for ancestor in abs_cwd.ancestors() {
            if ancestor.file_name().and_then(|n| n.to_str()) == Some("worktrees") {
                if let Some(root) = ancestor.parent() {
                    cmd.env("CARGO_TARGET_DIR", root.join("src-tauri").join("target"));
                    break;
                }
            }
        }
    }

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
            decode_output_bytes(&buf)
        };

        let stderr_drain = async {
            let mut buf = Vec::new();
            let _ = stderr_pipe.read_to_end(&mut buf).await;
            decode_output_bytes(&buf)
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

pub fn sync_context_state(
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
    let driver = MainAgentDriver::new(
        config.task_id.clone(),
        config.worktree_path.clone(),
        config.repo_root.clone(),
        config.app_handle.clone(),
        event_sink,
    );
    let engine = ReActEngine::new(driver);
    let event_sink = |ev: AgentEvent| engine.driver.emit_event(ev);
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
            "### [ВОЗОБНОВЛЕНИЕ ЗАДАЧИ / TASK RESUMED]\nЗадача на этапе '{}' (роль: {}). Продолжай автономное выполнение задачи. Используй доступные инструменты.",
            stage_name, config.role
        );
        messages.push(ChatMessage::user(resume_msg));
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
                "### [ВОЗОБНОВЛЕНИЕ ЗАДАЧИ / TASK RESUMED]\nЗадача на этапе '{}' (роль: {}). Продолжай автономное выполнение задачи. Используй доступные инструменты.",
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

        let stream_opts = StreamConsumerOptions {
            model: &config.model,
            turn,
            system_tokens,
            tool_tokens,
            fallback_input_tokens: compactor::estimate_request_tokens(&messages, tools.as_deref()) as u64,
            prefix_hash: Some(prefix_hash),
            log_prefix: "telemetry",
            subagent_id: None,
            emit_thoughts: true,
        };

        let stream_res = engine.consume_turn_stream(&mut rx, &abort_flag, &stream_opts).await;

        if stream_res.aborted {
            sync_context_state(&config.app_handle, &config.task_id, turn, &messages);
            event_sink(AgentEvent::Aborted);
            bail!("Agent execution aborted by user");
        }

        if let Some(err) = stream_res.error {
            bail!("LLM streaming error: {}", err);
        }

        if !stream_res.received_any_chunk && !abort_flag.load(Ordering::SeqCst) {
            let err_msg = format!(
                "No stream chunks received from LLM endpoint ({}) - check proxy connection or port",
                config.base_url
            );
            eprintln!("CVH [agent]: {}", err_msg);
            event_sink(AgentEvent::Error(err_msg.clone()));
            bail!("{}", err_msg);
        }

        let turn_content = stream_res.content;
        let turn_thoughts = stream_res.thoughts;
        let turn_tool_calls = stream_res.tool_calls;

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
