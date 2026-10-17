pub mod compactor;
pub use compactor::*;

use crate::config::{
    agent_allows_tool, is_tool_allowed, load_agent_setting, load_tool_policies,
    ToolPolicyConfig,
};
use crate::db::ActivityItem;
use crate::hooks::{build_windows_cmd_args, kill_process_tree};
use crate::llm::{ChatMessage, LlmClient, OpenAIChatRequest, StreamChunk, ToolCall, ToolDefinition};
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
    #[serde(rename = "createdAt", alias = "created_at", default)]
    pub created_at: i64,
    #[serde(rename = "updatedAt", alias = "updated_at", default)]
    pub updated_at: i64,
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
    },
    Compacted {
        saved_chars: usize,
        estimated_saved_tokens: usize,
        evaporated_count: usize,
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
pub const BUILTIN_CVH_DEV_PROMPT: &str = r#"# Role Persona: cvh-dev (Harness Router)
Use PROACTIVELY when: empty/new repo, missing .cvh/, broken boards/pipeline/personas/hooks, or new stack to onboard. Goal: one runnable vertical slice.
Identity:
- Columns = Agents: board IS the control plane; column `persona: <role>` on-enter runs `run_agent(taskId)`; flow `backlog` -> ... -> `ready`.
- Router-only: you may only search/inspect/load/delegate via glob/read_file/grep/execute_command/subagent; full text loads lazy by slug from rules/.
- 1 ticket = 1 vertical slice = 1 commit; tiers T0-spec / T1-scaffold / T2-verify / T3-commit; redact secrets at ingest, never log keys.
Overview: bootstrap or repair CVH harness (boards, pipeline, personas, policy, hooks) then stop. No monolith prompt, no docs dump.
When NOT: not a coder for feature logic; not a reviewer (separate clean context); not a docs writer; if task needs code, delegate to `developer`, if audit to `reviewer`.
Paths (repo-root relative, single source of truth):
- `.cvh/boards.json {activeBoardId, boards[{id,name,columns}]}` defines `development` + `kanban` boards; `activeBoardId` selects one.
- `.cvh/pipeline.json {id,title,columns[{id,title,persona,contextWindow,gatekeeperSensor,allowedTools,terminal}]}` mirrors active board columns.
- `.cvh/settings/tool-policy.json {globalEnabled, stageOverrides}` is the only policy file; `.cvh/agents/<role>/persona.md` (directory structure; legacy `.cvh/agents/<role>.md` replaced) + `<role>/rules/*.md` hold personas; `.cvh/tools/<tool>.json` custom tools; `.cvh/hooks/` transition scripts; task work only in `worktrees/<slug>`.
Frontmatter template for every generated `.cvh/agents/<role>/persona.md` (keep ≤30 lines, link detail, never copypaste):
---
name: <role>
description: Use PROACTIVELY when <trigger condition for this role>
model: inherit
tools: [read_file, glob, grep, execute_command]
scope: files-only | read-only | gated-writes
activation: on-enter <column-id> runs run_agent(<task-id>)
---
Identity: 3 bullets max (who, does, refuses).
Load: rules/<topic>.md (progressive disclosure, each rule file ≤8KB).
- If asked to configure boards: read rules/boards.md (.cvh/agents/cvh-dev/rules/boards.md).
- If asked to configure agents/personas: read rules/agents.md (.cvh/agents/cvh-dev/rules/agents.md).
- If asked to configure hooks: read rules/hooks.md (.cvh/agents/cvh-dev/rules/hooks.md).
Boards skeleton (2 boards, minimal):
{"activeBoardId":"development","boards":[{"id":"development","name":"Development","columns":[]},{"id":"kanban","name":"Kanban","columns":[]}]}
Pipeline skeleton (ids/order must match board; backlog first, ready last):
{"id":"development","title":"Development","columns":[{"id":"dev","title":"Dev","persona":"developer","contextWindow":400000},{"id":"review","title":"Review","persona":"reviewer","contextWindow":200000,"gatekeeperSensor":"cargo check"},{"id":"ready","title":"Ready","terminal":true}]}
Tool-policy skeleton (AND-semantics: effective = globalEnabled && stageOverrides; global false cannot be re-enabled):
{"globalEnabled":{"read_file":true,"glob":true,"grep":true,"execute_command":true,"edit_file":true,"write_file":true,"web_search":true,"web_fetch":true,"subagent":true},"stageOverrides":{"dev":{"web_search":false},"review":{"edit_file":false,"write_file":false}}}
Rules (apply to every step):
- Board Reconfiguration Invariant: When creating, renaming, or modifying boards in .cvh/boards.json or .cvh/pipeline.json, NEVER delete the board or column where your current task is running. If you create new boards, ensure all existing tasks in .cvh/tasks.json have their boardId cleanly preserved or mapped to the intended boards.
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
Process (Explore->Plan->Code->Verify->Commit): Explore (glob/grep), Plan (todos + file:line checklist), Code (edit in worktree), Verify (gate sensors: `cargo check`, `cargo clippy -- -D warnings`, `tsc`, `vite build`), Commit (1 slice = 1 commit).
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
    let mut sections = Vec::new();

    // 1. Role Persona: directory path (.cvh/agents/<role>/persona.md) → built-in fallback.
    let persona_path = crate::config::agent_persona_path(repo_root, role);
    if persona_path.is_file() {
        if let Ok(persona_text) = tokio::fs::read_to_string(&persona_path).await {
            sections.push(format!("# Role Persona: {}\n\n{}", role, persona_text.trim()));
        } else if role == "cvh-dev" {
            sections.push(BUILTIN_CVH_DEV_PROMPT.to_string());
        } else if role == "developer" {
            sections.push(BUILTIN_DEVELOPER_PROMPT.to_string());
        } else {
            sections.push(format!("# Role Persona: {}\nYou are an expert AI agent in {}.", role, role));
        }
    } else if role == "cvh-dev" {
        sections.push(BUILTIN_CVH_DEV_PROMPT.to_string());
    } else if role == "developer" {
        sections.push(BUILTIN_DEVELOPER_PROMPT.to_string());
    } else {
        sections.push(format!("# Role Persona: {}\nYou are an expert AI agent in {}.", role, role));
    }

    // 1b. Per-agent rules: `.cvh/agents/<role>/rules/*.md`, alphabetical.
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

    // 2. Tool Instructions (SWE-bench Industrial Suite)
    sections.push(
        "# Scoped Tool Execution Mandate\n\
        You have direct tool access scoped to your isolated worktree. \
        Use `read_file`, `write_file`, `edit_file`, `glob`, `grep`, and `execute_command` to inspect, modify, and verify code. \
        Always use Ground Truth Sensors (e.g. `cargo check`, `npm run build`) via `execute_command` to verify changes.\n\
        \n\
        ## Subagent Delegation Mandate\n\
        - When delegating work to a subagent, prefer `subagent` (oneshot sync) which blocks and waits for the full report before proceeding.\n\
        - If you use `subagent_bg` for background execution, you MUST call `collect_subagent` with the returned `subagent_id` to wait for and consume the subagent's report before taking subsequent actions or finishing the task. Never fire-and-forget background subagents without collecting them.\n\
        \n\
        When finished, output your comprehensive final answer."
            .to_string(),
    );

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
            "todo_write",
            "Record and update task plan list to show progress. Add todos with status pending | in_progress | completed.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
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
            "Передать задачу на следующий этап пайплайна (шаг вперед). Обязательно передайте подробное саммари проделанной работы для следующего агента.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "summary": { "type": "string", "description": "Подробное саммари изменений, результатов и проверок" }
                },
                "required": ["summary"]
            }),
        ),
        ToolDefinition::function(
            "reject_task",
            "Вернуть задачу на предыдущий этап пайплайна (шаг назад с замечаниями). Обязательно укажите причину отклонения и конкретные замечания.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "reason": { "type": "string", "description": "Причина отклонения: упавшие проверки, баги, необходимые исправления" }
                },
                "required": ["reason"]
            }),
        ),
        ToolDefinition::function(
            "question",
            "Задать интерактивный уточняющий вопрос пользователю и приостановить выполнение агента до получения ответа (Human-in-the-Loop). Алиас: 'ask_user'.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "The specific question to ask the user." },
                    "options": {
                        "type": "array",
                        "description": "Predefined choices for the user. If recommended, put '(Рекомендуется)' in label.",
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
            "Декомпозировать требования и создать новую карточку задачи на Канбан-доске. Привязывает созданную задачу к текущей родительской карточке оркестратора (parent_id = current_task_id).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Краткое и емкое название задачи" },
                    "description": { "type": "string", "description": "Полные технические требования, критерии приёмки и файлы" },
                    "column_id": { "type": "string", "description": "Целевая колонка (по умолчанию 'dev' или первая рабочая колонка)" },
                    "board_id": { "type": "string", "description": "Целевая доска (по умолчанию доска текущей задачи)" },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Теги задачи"
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["low", "medium", "high"],
                        "description": "Приоритет задачи: low, medium, high"
                    }
                },
                "required": ["title", "description"]
            }),
        ),
        ToolDefinition::function(
            "create_tasks",
            "Пакетно декомпозировать требования и создать несколько карточек задач на Канбан-доске для оркестратора.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "tasks": {
                        "type": "array",
                        "description": "Список задач для создания",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string", "description": "Краткое и емкое название задачи" },
                                "description": { "type": "string", "description": "Полные технические требования, критерии приёмки и файлы" },
                                "column_id": { "type": "string", "description": "Целевая колонка (по умолчанию 'dev' или первая рабочая колонка)" },
                                "board_id": { "type": "string", "description": "Целевая доска (по умолчанию доска текущей задачи)" },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Теги задачи"
                                },
                                "priority": {
                                    "type": "string",
                                    "enum": ["low", "medium", "high"],
                                    "description": "Приоритет задачи: low, medium, high"
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
            "Завершить текущую задачу оркестратора и перевести её в финальную колонку Готово (ready/done). Доступно только оркестратору, создавшему подзадачи.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "summary": { "type": "string", "description": "Исчерпывающее саммари плана декомпозиции и созданных подзадач" }
                },
                "required": ["summary"]
            }),
        ),
    ]
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
    let mut tools = get_standard_tools();
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
    tools
}

/// Tool Output Spilling: if stdout/output exceeds 100 lines or 8KB, save full log to `.cvh/spill/<timestamp>-<tool>.log`,
/// return head (first 20 lines) and tail (last 20 lines) + reference to spilled file.
pub async fn maybe_spill_output(
    repo_root: &Path,
    tool_name: &str,
    output: String,
) -> Result<String> {
    let lines_count = output.lines().count();
    let bytes_len = output.len();

    if lines_count > 100 || bytes_len > 8192 {
        let spill_dir = repo_root.join(".cvh").join("spill");
        tokio::fs::create_dir_all(&spill_dir).await?;
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f");
        let safe_name = tool_name.replace(['/', '\\', ':', '*'], "_");
        let filename = format!("{}-{}.log", timestamp, safe_name);
        let spill_file = spill_dir.join(&filename);

        tokio::fs::write(&spill_file, &output).await?;

        let all_lines: Vec<&str> = output.lines().collect();
        let head = all_lines[..20.min(all_lines.len())].join("\n");
        let tail = if all_lines.len() > 40 {
            all_lines[all_lines.len() - 20..].join("\n")
        } else {
            String::new()
        };

        let rel_spill = format!(".cvh/spill/{}", filename);
        let spilled_msg = format!(
            "[Output truncated: {} lines ({} bytes) spilled to {}]\n--- HEAD (first 20 lines) ---\n{}\n\n--- TAIL (last 20 lines) ---\n{}\n[Full log available at {}]",
            lines_count, bytes_len, rel_spill, head, tail, rel_spill
        );
        Ok(spilled_msg)
    } else {
        Ok(output)
    }
}

/// Calculates informative error message when `old_string` is not found in file content.
pub fn build_edit_mismatch_error(content: &str, file_path: &str, old_string: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if lines.is_empty() {
        return format!("Edit failed: File '{}' is empty (0 lines). Cannot locate 'old_string'.", file_path);
    }

    let search_trimmed = old_string.trim();
    let search_first_line = search_trimmed.lines().next().unwrap_or("").trim();

    let mut best_score = 0.0f32;
    let mut best_idx = 0;

    for (idx, line) in lines.iter().enumerate() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() {
            continue;
        }
        let score = string_similarity(line_trimmed, search_first_line);
        if score > best_score {
            best_score = score;
            best_idx = idx;
        }
    }

    let mut msg = format!(
        "Target 'old_string' not found in '{}' (total {} lines).\n",
        file_path, total_lines
    );

    if best_score >= 0.25 {
        let start = best_idx.saturating_sub(4);
        let end = (best_idx + 5).min(total_lines);
        msg.push_str(&format!(
            "Closest matching candidate line identified at line {} (similarity: {:.0}%):\n",
            best_idx + 1,
            best_score * 100.0
        ));
        msg.push_str("--- Surrounding Context (use exact line content) ---\n");
        for (i, line) in lines[start..end].iter().enumerate() {
            let line_no = start + i + 1;
            let marker = if line_no == best_idx + 1 { ">>" } else { "  " };
            msg.push_str(&format!("{} {:4} | {}\n", marker, line_no, line));
        }
    } else {
        msg.push_str("No close match found. First 10 lines of file for context:\n");
        let end = 10.min(total_lines);
        for (i, line) in lines[..end].iter().enumerate() {
            msg.push_str(&format!("   {:4} | {}\n", i + 1, line));
        }
    }

    msg.push_str("Recommendation: Use read_file to inspect exact lines and retry edit_file with precise context.");
    msg
}

fn string_similarity(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }
    if a.contains(b) || b.contains(a) {
        return 0.85;
    }
    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if a_words.is_empty() || b_words.is_empty() {
        return 0.0;
    }
    let inter = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();
    inter as f32 / union as f32
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
}

#[derive(Clone)]
pub struct SubagentSession {
    pub sender: Arc<tokio::sync::Mutex<Option<oneshot::Sender<String>>>>,
    pub retry_info: Arc<tokio::sync::Mutex<SubagentRetryInfo>>,
}

pub static SUBAGENT_RECEIVERS: LazyLock<tokio::sync::Mutex<HashMap<String, SubagentReceiver>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));
pub static SUBAGENT_SESSIONS: LazyLock<tokio::sync::Mutex<HashMap<String, SubagentSession>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

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
    let normalized_tool_name = match tool_name {
        "bash" => "execute_command",
        "str_replace" => "edit_file",
        "ask_user" => "question",
        "create_tasks" => "create_task",
        other => other,
    };

    // Zero-Trust Iron Barrier: verify tool permission for current stage & role
    if !is_tool_allowed(tool_policy, stage_id, normalized_tool_name) {
        bail!(
            "PermissionDenied: Инструмент '{}' отключен политикой безопасности для колонки '{}'",
            normalized_tool_name,
            stage_id.unwrap_or("global")
        );
    }
    if let Some(role) = ctx.role {
        if !is_tool_allowed(tool_policy, Some(role), normalized_tool_name) {
            bail!(
                "PermissionDenied: Инструмент '{}' отключен политикой безопасности для агента '{}'",
                normalized_tool_name,
                role
            );
        }
    }

    // Per-agent intersection: when the agent `setting` defines allowed_tools,
    // a tool outside that list is denied even if the matrix allows it.
    if !agent_allows_tool(agent_allowed_tools, normalized_tool_name) {
        bail!(
            "PermissionDenied: Инструмент '{}' запрещён настройками агента (setting.allowed_tools)",
            normalized_tool_name
        );
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

            let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(2000) as usize;

            let content = tokio::fs::read_to_string(&target)
                .await
                .with_context(|| format!("Failed to read file: {:?}", rel_path))?;

            let lines: Vec<&str> = content.lines().collect();
            let start = offset.saturating_sub(1);
            let end = (start + limit).min(lines.len());

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

            if !current.contains(old_string) {
                let informative_err = build_edit_mismatch_error(&current, rel_path, old_string);
                bail!(informative_err);
            }

            let updated = if replace_all {
                current.replace(old_string, new_string)
            } else {
                let count = current.matches(old_string).count();
                if count > 1 {
                    bail!(
                        "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more surrounding lines.",
                        count
                    );
                }
                current.replacen(old_string, new_string, 1)
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
            let timeout_secs = args.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(60);

            let res = run_scoped_command(worktree_dir, command_str, Duration::from_secs(timeout_secs), abort_handle).await?;
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
            };

            let session = SubagentSession {
                sender: Arc::new(tokio::sync::Mutex::new(Some(tx))),
                retry_info: Arc::new(tokio::sync::Mutex::new(retry_info)),
            };

            {
                let mut sessions = SUBAGENT_SESSIONS.lock().await;
                sessions.insert(subagent_id.clone(), session);
            }

            let owned_ctx = ctx.to_owned();
            let sub_id = subagent_id.clone();
            let title_s = title.to_string();
            let role_s = role.to_string();
            let prompt_s = prompt.to_string();

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

            // Parent agent blocks synchronously awaiting clean report from channel (Zero-Pollution Invariant!)
            match rx.await {
                Ok(clean_report) => clean_report,
                Err(_) => bail!("Subagent channel closed before completion"),
            }
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
            };

            let session = SubagentSession {
                sender: Arc::new(tokio::sync::Mutex::new(Some(tx))),
                retry_info: Arc::new(tokio::sync::Mutex::new(retry_info)),
            };

            {
                let mut receivers = SUBAGENT_RECEIVERS.lock().await;
                receivers.insert(subagent_id.clone(), rx);
                let mut sessions = SUBAGENT_SESSIONS.lock().await;
                sessions.insert(subagent_id.clone(), session);
            }

            let owned_ctx = ctx.to_owned();
            let sub_id = subagent_id.clone();
            let title_s = title.to_string();
            let role_s = role.to_string();
            let prompt_s = prompt.to_string();

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

            if ids.len() == 1 {
                let subagent_id = &ids[0];
                let rx_opt = {
                    let mut reg = SUBAGENT_RECEIVERS.lock().await;
                    remove_subagent_receiver(&mut reg, subagent_id)
                };

                match rx_opt {
                    Some((resolved_id, rx)) => match rx.await {
                        Ok(report) => report,
                        Err(_) => bail!("Subagent [{}] channel closed before completion", resolved_id),
                    },
                    None => bail!("Subagent [{}] not found in background registry (already collected or invalid ID)", subagent_id),
                }
            } else {
                let mut receivers = Vec::new();
                {
                    let mut reg = SUBAGENT_RECEIVERS.lock().await;
                    for id in &ids {
                        if let Some((resolved_id, rx)) = remove_subagent_receiver(&mut reg, id) {
                            receivers.push((resolved_id, rx));
                        }
                    }
                }

                if receivers.is_empty() {
                    bail!("None of the requested subagents {:?} were found in background registry", ids);
                }

                let mut reports = Vec::new();
                for (id, rx) in receivers {
                    match rx.await {
                        Ok(report) => reports.push(format!("### Subagent [{}]
{}", id, report)),
                        Err(_) => reports.push(format!("### Subagent [{}]
Channel closed before completion", id)),
                    }
                }
                reports.join("

---

")
            }
        }

        "todo_write" => {
            let raw_todos = args
                .get("todos")
                .and_then(|v| v.as_array())
                .context("Missing required parameter 'todos' (must be array)")?;

            let mut parsed_todos: Vec<TodoItem> = Vec::new();
            let mut completed_cnt = 0;
            let mut in_progress_cnt = 0;
            let mut pending_cnt = 0;

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

            // If task_id provided, atomically persist to .cvh/tasks.json
            if let Some(tid) = task_id {
                let tasks_path = repo_root.join(".cvh").join("tasks.json");
                if tasks_path.exists() {
                    if let Ok(content) = tokio::fs::read_to_string(&tasks_path).await {
                        if let Ok(mut tasks) = serde_json::from_str::<Vec<Task>>(&content) {
                            if let Some(target_task) = tasks.iter_mut().find(|t| t.id == tid) {
                                target_task.todos = Some(parsed_todos.clone());
                                target_task.updated_at = chrono::Utc::now().timestamp_millis();
                                if let Ok(serialized) = serde_json::to_string_pretty(&tasks) {
                                    let _ = crate::config::atomic_write_file(&tasks_path, &serialized).await;
                                }
                            }
                        }
                    }
                }
            }

            format!(
                "Updated todo list: {} completed, {} in progress, {} pending.",
                completed_cnt, in_progress_cnt, pending_cnt
            )
        }

        "advance_task" => {
            let summary = args
                .get("summary")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'summary'")?;

            let task_id = task_id.context("No active task ID for advance_task")?;

            crate::commands::internal_advance_task(
                repo_root,
                task_id,
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

            let task_id = task_id.context("No active task ID for reject_task")?;

            // 1. Determine active board and find current column
            let tasks = crate::commands::load_tasks_from_disk(repo_root).await;
            let current_task = tasks
                .iter()
                .find(|t| t.id == task_id)
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

            // ЖЕЛЕЗНЫЙ ИНВАРИАНТ: Если текущая колонка — первая рабочая колонка после backlog (колонка A),
            // шаг назад в backlog СТРОГО ЗАПРЕЩЁН! Возвращай ошибку:
            // "Запрещено возвращать задачу из первой рабочей стадии назад в бэклог. Задача должна быть доработана или закрыта."
            let is_prev_backlog = prev_col.id == "backlog"
                || prev_col.title.to_lowercase().contains("backlog")
                || prev_col.title.to_lowercase().contains("бэклог")
                || (curr_idx == 1 && columns[0].id == "backlog");

            if is_prev_backlog {
                bail!("Запрещено возвращать задачу из первой рабочей стадии назад в бэклог. Задача должна быть доработана или закрыта.");
            }

            let rejection_summary = format!("ОТКЛОНЕНО РЕВЬЮ: {}", reason);

            // 2. Execute move_task_stage logic with status 'idle', verdict 'CHANGES_REQUESTED', and handover_summary
            let updated_task = crate::commands::internal_move_task_stage(
                repo_root,
                task_id,
                &prev_col.id,
                Some("CHANGES_REQUESTED".to_string()),
                Some("idle".to_string()),
                Some(rejection_summary),
                ctx.app_handle.as_ref(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

            // 3. Save rejection reason to task dialog/history
            let reject_content = format!(
                "### Задача возвращена на этап: {}\n\n**Причина отклонения:**\n{}",
                prev_col.title,
                reason
            );
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
            }

            format!(
                "ОТКЛОНЕНО: Задача {} возвращена на этап '{}' ({}) по причине: {}",
                task_id,
                prev_col.title,
                prev_col.id,
                reason
            )
        }

        "question" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'text'")?;
            let options = args.get("options").cloned();
            let multiple = args.get("multiple").and_then(|v| v.as_bool()).unwrap_or(false);

            let question_id = uuid::Uuid::new_v4().to_string();
            let task_id_str = task_id.unwrap_or("unknown").to_string();

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

                    inputs.push(TaskInput {
                        title,
                        description,
                        column_id,
                        board_id,
                        tags,
                        priority,
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

                inputs.push(TaskInput {
                    title,
                    description,
                    column_id,
                    board_id,
                    tags,
                    priority,
                });
            }

            let mut tasks = crate::commands::load_tasks_from_disk(repo_root).await;
            let current_task_opt = task_id.and_then(|tid| tasks.iter().find(|t| t.id == tid).cloned());
            let boards_cfg = crate::config::load_boards_config(repo_root).await.ok();

            let mut created_results = Vec::new();
            let now = chrono::Utc::now().timestamp_millis();
            let mut next_num = crate::commands::tasks::next_task_num(&tasks);

            for input in inputs {
                while tasks.iter().any(|t| t.id == format!("CVH-{:03}", next_num)) {
                    next_num = next_num.saturating_add(1);
                }
                let new_id = format!("CVH-{:03}", next_num);
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

                // Resolve column (defaults to "dev" or active board's first actionable column)
                let effective_column_id = if let Some(ref cid) = input.column_id {
                    cid.clone()
                } else if let Some(ref cols) = target_board_cols {
                    if let Some(dev_col) = cols.iter().find(|c| crate::commands::tasks::stage_id_matches(&c.id, "dev")) {
                        dev_col.id.clone()
                    } else if let Some(first_act) = cols.iter().find(|c| {
                        !crate::commands::tasks::stage_id_matches(&c.id, "backlog")
                            && c.terminal != Some(true)
                            && !crate::commands::tasks::stage_id_matches(&c.id, "ready")
                            && !crate::commands::tasks::stage_id_matches(&c.id, "done")
                    }) {
                        first_act.id.clone()
                    } else if cols.len() > 1 {
                        cols[1].id.clone()
                    } else if let Some(first) = cols.first() {
                        first.id.clone()
                    } else {
                        "dev".to_string()
                    }
                } else {
                    "dev".to_string()
                };

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
                    model: None,
                    parent_id: task_id.map(|s| s.to_string()),
                    subtask_index: None,
                    waiting_for_subtask_id: None,
                    handover_summary: None,
                    cover_image: None,
                    attachments: None,
                    todos: None,
                    priority: input.priority,
                    tags: input.tags,
                    created_at: now,
                    updated_at: now,
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

            let current_task_id = task_id.context("No active task ID for complete_task")?;

            let mut tasks = crate::commands::load_tasks_from_disk(repo_root).await;
            let current_task_idx = tasks
                .iter()
                .position(|t| t.id == current_task_id)
                .ok_or_else(|| anyhow::anyhow!("Task {} not found", current_task_id))?;

            // Anti-Bypass Security Check:
            // Check if current task has spawned child tasks (parent_id linkage or created_subtasks), OR if the column is an orchestrator column.
            let has_child_tasks = tasks
                .iter()
                .any(|t| t.parent_id.as_deref() == Some(current_task_id));

            let current_task = &tasks[current_task_idx];
            let stage_name = ctx.stage_id.unwrap_or(&current_task.stage_id).to_lowercase();
            let role_name = ctx.role.unwrap_or_default().to_lowercase();

            let boards_cfg = crate::config::load_boards_config(repo_root).await.ok();
            let board_cols = boards_cfg.as_ref().and_then(|bc| {
                let b = if let Some(ref bid) = current_task.board_id {
                    bc.boards.iter().find(|b| &b.id == bid)
                } else {
                    bc.boards.iter().find(|b| b.id == bc.active_board_id)
                };
                b.map(|b| b.columns.clone())
            });

            let is_col_orchestrator = if let Some(ref cols) = board_cols {
                cols.iter().any(|c| {
                    c.id == current_task.stage_id
                        && (c.id.to_lowercase().contains("orchestrat")
                            || c.title.to_lowercase().contains("orchestrat")
                            || c.title.to_lowercase().contains("оркестрат")
                            || c.persona.as_deref().map(|p| p.to_lowercase().contains("orchestrat") || p.to_lowercase().contains("оркестрат")).unwrap_or(false))
                })
            } else {
                false
            };

            let is_orchestrator = is_col_orchestrator
                || stage_name.contains("orchestrat")
                || stage_name.contains("оркестрат")
                || role_name.contains("orchestrat")
                || role_name.contains("оркестрат");

            if !has_child_tasks && !is_orchestrator {
                bail!("complete_task is restricted to orchestrator tasks that created subtasks.");
            }

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

        custom_tool => {
            let custom_tools = load_custom_tools(repo_root).await;
            if let Some(ct) = custom_tools.into_iter().find(|t| t.name == custom_tool) {
                let cmd_str = ct.command.replace("{{args}}", &args.to_string());
                let res = run_scoped_command(worktree_dir, &cmd_str, Duration::from_secs(60), abort_handle).await?;
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
    let parent_id = ctx.task_id.as_deref().unwrap_or("unknown").to_string();
    let subagent_suffix = match ctx.call_id.as_deref() {
        Some(cid) if !cid.trim().is_empty() => cid.trim().to_string(),
        _ => format!(
            "{}-{}",
            chrono::Utc::now().timestamp_millis(),
            uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>()
        ),
    };
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

    // 1. Tool filtering (If tools is omitted, defaults to ALL tools available to the parent agent!)
    let all_tools = get_all_tools(&ctx.repo_root).await;
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
                && t != "advance_task"
                && t != "reject_task"
                && t != "create_task"
                && t != "create_tasks"
                && t != "complete_task"
                && is_tool_allowed(&ctx.tool_policy, ctx.stage_id.as_deref(), t)
                && agent_allows_tool(ctx.agent_allowed_tools.as_deref(), t)
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
        tool: None,
        duration_ms: None,
        images: None,
    };
    let _ = crate::db::append_dialog_items(
        &ctx.repo_root,
        &subagent_dialog_id,
        std::slice::from_ref(&prompt_item),
    );

    // 4. Resolve schemas for allowed subagent tools
    let all_tools = get_all_tools(&ctx.repo_root).await;
    let mut active_sub_tool_defs: Vec<ToolDefinition> = all_tools
        .into_iter()
        .filter(|t| effective_sub_tools.contains(&t.function.name))
        .collect();
    active_sub_tool_defs.sort_by(|a, b| a.function.name.cmp(&b.function.name));

    // 5. System persona & initial messages
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
        Working directory: {}
        Task Title: {}
        Allowed Tools: {:?}
        Invariants: Do not attempt to move kanban stages or spawn nested subagents.         Execute the task thoroughly and verify results.         When done, output your comprehensive final report.",
        role,
        persona_content.as_deref().unwrap_or("You are a specialized autonomous subagent."),
        ctx.worktree_dir.to_string_lossy(),
        title,
        effective_sub_tools
    );

    let mut sub_messages = vec![
        ChatMessage::system(system_prompt).with_cache_control(crate::llm::CacheControl::ephemeral()),
        ChatMessage::user(format!("# Objective: {}

{}", title, prompt)).with_cache_control(crate::llm::CacheControl::ephemeral()),
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
        if sub_turn > 15 || ctx.abort_handle.load(Ordering::SeqCst) {
            break;
        }

        let req = OpenAIChatRequest {
            model: model.to_string(),
            messages: sub_messages.clone(),
            temperature,
            stream: true,
            tools: if active_sub_tool_defs.is_empty() {
                None
            } else {
                Some(active_sub_tool_defs.clone())
            },
            reasoning_effort: reasoning_effort.clone(),
        };

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
        let mut turn_tool_calls: Vec<ToolCall> = Vec::new();
        let mut thinking_parser = ThinkingStreamParser::new();

        while let Some(chunk) = rx.recv().await {
            if ctx.abort_handle.load(Ordering::SeqCst) {
                break;
            }
            match chunk {
                StreamChunk::Token(token) => {
                    let parsed = thinking_parser.push(&token);
                    for (is_thought, text) in parsed {
                        if is_thought {
                            let th_item = ActivityItem {
                                id: format!("sub-th-{}-{}", subagent_suffix, uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>()),
                                item_type: "thought".to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                content: Some(text.clone()),
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
                StreamChunk::Thought(thought) => {
                    let th_item = ActivityItem {
                        id: format!("sub-th-{}-{}", subagent_suffix, uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>()),
                        item_type: "thought".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        content: Some(thought.clone()),
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
                    turn_tool_calls.push(ToolCall {
                        id,
                        call_type: "function".to_string(),
                        function: crate::llm::FunctionCall { name, arguments },
                    });
                }
                StreamChunk::Done => break,
                StreamChunk::Error(e) => {
                    eprintln!("CVH [subagent]: error: {}", e);
                    sub_error = Some(e);
                    break;
                }
                _ => {}
            }
        }

        // Flush thinking parser
        for (is_thought, text) in thinking_parser.flush() {
            if is_thought {
                let th_item = ActivityItem {
                    id: format!("sub-th-{}-{}", subagent_suffix, uuid::Uuid::new_v4().to_string().chars().take(4).collect::<String>()),
                    item_type: "thought".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    content: Some(text),
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
                stage_id: ctx.stage_id.as_deref(),
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
            let duration_ms = (chrono::Utc::now().timestamp_millis() - start_time) as f64;

            let (res_str, is_success) = match exec_res {
                Ok(s) => (s, true),
                Err(e) => (format!("Error executing {}: {}", tc.function.name, e), false),
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
        let err_item = ActivityItem {
            id: format!("sub-err-{}", uuid::Uuid::new_v4()),
            item_type: "error".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            content: Some(err_msg.clone()),
            tool: None,
            duration_ms: None,
            images: None,
        };
        let _ = crate::db::append_dialog_items(&ctx.repo_root, &subagent_dialog_id, std::slice::from_ref(&err_item));
        if let Some(ref app) = ctx.app_handle {
            let _ = app.emit("subagent:error", serde_json::json!({
                "parentTaskId": &parent_id,
                "subagentId": &subagent_id,
                "error": &err_msg,
            }));
        }

        if let Some((_, session)) = find_subagent_session(Some(&parent_id), &subagent_id).await {
            let mut info = session.retry_info.lock().await;
            info.status = "error".to_string();
            info.last_error = Some(err_msg.clone());
        }

        bail!("Subagent [{}] error: {}", subagent_id, err_msg);
    }

    let report = if final_sub_response.trim().is_empty() {
        format!(
            "[Subagent ID: {}]
Subagent [{}] '{}' executed in-process. Work complete.",
            subagent_id, role, title
        )
    } else {
        format!(
            "[Subagent ID: {}]
Subagent [{}] '{}' report:

{}",
            subagent_id, role, title, final_sub_response.trim()
        )
    };

    // Subagent completed cleanly: deliver report to waiting parent channel!
    if let Some((_, session)) = find_subagent_session(Some(&parent_id), &subagent_id).await {
        let mut info = session.retry_info.lock().await;
        info.status = "finished".to_string();
        info.last_error = None;
        if let Some(sender) = session.sender.lock().await.take() {
            let _ = sender.send(report.clone());
        }
    }

    if let Some(ref app) = ctx.app_handle {
        let _ = app.emit(
            "subagent:finished",
            serde_json::json!({
                "parentTaskId": &parent_id,
                "subagentId": &subagent_id,
                "response": &final_sub_response,
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
        }
    }
    info.status = "running".to_string();
    info.last_error = None;

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
        c.args(build_windows_cmd_args(cmd_str));
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
    if pattern == "*" || pattern == "**/*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return candidate.ends_with(suffix);
    }
    candidate.contains(pattern)
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
    pub steer_prompt: Option<String>,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<String>,
    pub max_turns: usize,
    pub app_handle: Option<AppHandle>,
    pub context_window: Option<u64>,
}

#[derive(Debug, Default)]
pub struct ThinkingStreamParser {
    in_thinking: bool,
    buffer: String,
}

impl ThinkingStreamParser {
    pub fn new() -> Self {
        Self {
            in_thinking: false,
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
                let open_thinking = self.buffer.find("<thinking>");
                let open_think = self.buffer.find("<think>");

                let earliest_open = match (open_thinking, open_think) {
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

                if let Some((pos, tag_len)) = earliest_open {
                    if pos > 0 {
                        outputs.push((false, self.buffer[..pos].to_string()));
                    }
                    self.in_thinking = true;
                    self.buffer = self.buffer[pos + tag_len..].to_string();
                    continue;
                }

                // If not found, check if end of buffer could be a partial opening tag
                let mut partial_len = 0;
                for tag in &["<thinking>", "<think>"] {
                    for len in 1..tag.len() {
                        if self.buffer.ends_with(&tag[..len]) && len > partial_len {
                            partial_len = len;
                        }
                    }
                }

                if partial_len > 0 {
                    let safe_len = self.buffer.len() - partial_len;
                    if safe_len > 0 {
                        outputs.push((false, self.buffer[..safe_len].to_string()));
                        self.buffer = self.buffer[safe_len..].to_string();
                    }
                    break;
                } else {
                    if !self.buffer.is_empty() {
                        outputs.push((false, std::mem::take(&mut self.buffer)));
                    }
                    break;
                }
            } else {
                // In thinking: look for closing tag </thinking> or </think>
                let close_thinking = self.buffer.find("</thinking>");
                let close_think = self.buffer.find("</think>");

                let earliest_close = match (close_thinking, close_think) {
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

                if let Some((pos, tag_len)) = earliest_close {
                    if pos > 0 {
                        outputs.push((true, self.buffer[..pos].to_string()));
                    }
                    self.in_thinking = false;
                    self.buffer = self.buffer[pos + tag_len..].to_string();
                    continue;
                }

                // If not found, check if end of buffer could be a partial closing tag
                let mut partial_len = 0;
                for tag in &["</thinking>", "</think>"] {
                    for len in 1..tag.len() {
                        if self.buffer.ends_with(&tag[..len]) && len > partial_len {
                            partial_len = len;
                        }
                    }
                }

                if partial_len > 0 {
                    let safe_len = self.buffer.len() - partial_len;
                    if safe_len > 0 {
                        outputs.push((true, self.buffer[..safe_len].to_string()));
                        self.buffer = self.buffer[safe_len..].to_string();
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
    // Deterministic static system prompt
    let system_prompt = compile_primed_system_prompt(&config.repo_root, &config.role).await?;

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

    let mut messages = vec![
        ChatMessage::system(system_prompt).with_cache_control(crate::llm::CacheControl::ephemeral()),
        ChatMessage::user(initial_user_prompt).with_cache_control(crate::llm::CacheControl::ephemeral()),
    ];

    if let Some(ref steer) = config.steer_prompt {
        let steer_trimmed = steer.trim();
        if !steer_trimmed.is_empty() {
            messages.push(ChatMessage::user(format!("[USER STEER]: {}", steer_trimmed)));
        }
    }

    if let Some(ref app) = config.app_handle {
        if let Some(state) = app.try_state::<crate::commands::AppState>() {
            if let Ok(mut map) = state.agent_contexts.lock() {
                map.insert(
                    config.task_id.clone(),
                    std::sync::Arc::new(std::sync::Mutex::new(compactor::AgentContextState {
                        current_turn: 0,
                        messages: messages.clone(),
                    })),
                );
            }
        }
    }

    // Tool Policy Matrix + per-agent intersection: filter out disabled tools
    // when building schema array (saving tokens!). Effective set =
    // policy lookup ∩ agent setting.allowed_tools (absent → unrestricted).
    let tool_policy = load_tool_policies(&config.repo_root)
        .await
        .unwrap_or_default();

    // Best-effort per-agent overrides (missing/broken → defaults).
    let agent_setting = load_agent_setting(&config.repo_root, &config.role).await;
    let agent_allowed = agent_setting.allowed_tools.clone();

    let all_tools = get_all_tools(&config.repo_root).await;
    let mut active_tools: Vec<ToolDefinition> = all_tools
        .into_iter()
        .filter(|t| {
            is_tool_allowed(&tool_policy, Some(&config.stage_id), &t.function.name)
                && is_tool_allowed(&tool_policy, Some(&config.role), &t.function.name)
                && agent_allows_tool(agent_allowed.as_deref(), &t.function.name)
        })
        .collect();

    // Deterministic sorting of tool definitions for prefix caching
    active_tools.sort_by(|a, b| a.function.name.cmp(&b.function.name));

    let llm_client = LlmClient::new();

    // Безлимитный ReAct-цикл (лимит шагов убран по требованию): выход только
    // через терминальный ответ модели (текст без тул-коллов), abort_flag
    // (кнопка «Остановить») или ошибку LLM-стрима.
    let mut turn: usize = 0;
    loop {
        turn += 1;
        if abort_flag.load(Ordering::SeqCst) {
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
        // Reads active column's context_window (default 128,000, or 64,000, etc.).
        // Triggers compaction whenever estimated tokens exceed 70% of context_window,
        // or > 20,000 tokens for long turns (turn > 3).
        let context_window = config.context_window.unwrap_or(128_000) as usize;
        let compaction_threshold = ((context_window as f64) * 0.70) as usize;
        let current_tokens = compactor::estimate_tokens(&messages);

        let should_compact = current_tokens > compaction_threshold || (turn > 3 && current_tokens > 20_000);
        if should_compact {
            let compaction_res = compactor::compact_conversation(&mut messages, None);
            if compaction_res.saved_chars > 0 {
                // If app_handle is present, persist to SQLite dialog_items
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
                            "savedTokens": compaction_res.estimated_saved_tokens,
                            "tokensBefore": current_tokens,
                            "tokensAfter": compactor::estimate_tokens(&messages),
                            "autoCompacted": true,
                            "contextWindow": context_window,
                        })),
                        duration_ms: None,
                        images: None,
                    };
                    let _ = crate::db::persist_compacted_dialog(&config.repo_root, &config.task_id, &compacted_item, &[]);
                    let _ = app.emit("dialog:updated", serde_json::json!({ "taskId": config.task_id }));
                }

                event_sink(AgentEvent::Compacted {
                    saved_chars: compaction_res.saved_chars,
                    estimated_saved_tokens: compaction_res.estimated_saved_tokens,
                    evaporated_count: compaction_res.evaporated_tools_count,
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

        let req = OpenAIChatRequest {
            model: config.model.clone(),
            messages: messages.clone(),
            temperature: config.temperature,
            stream: true,
            // Empty active_tools (everything disabled) is a valid case: send
            // tools=None so the field is omitted from the OpenAI-compatible
            // payload (stream_openai only sorts Some) and the model answers
            // with plain text. Sending `"tools":[]` is rejected by some proxies.
            tools: if active_tools.is_empty() {
                None
            } else {
                Some(active_tools.clone())
            },
            reasoning_effort: config.reasoning_effort.clone(),
        };

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
        let mut turn_tool_calls: Vec<ToolCall> = Vec::new();
        let mut thinking_parser = ThinkingStreamParser::new();
        let mut received_any_chunk = false;

        while let Some(chunk) = rx.recv().await {
            received_any_chunk = true;
            if abort_flag.load(Ordering::SeqCst) {
                event_sink(AgentEvent::Aborted);
                bail!("Agent execution aborted by user");
            }

            match chunk {
                StreamChunk::Token(token) => {
                    let parsed = thinking_parser.push(&token);
                    for (is_thought, text) in parsed {
                        if is_thought {
                            event_sink(AgentEvent::Thought(text));
                        } else {
                            turn_content.push_str(&text);
                            event_sink(AgentEvent::Token(text));
                        }
                    }
                }
                StreamChunk::Output(output) => {
                    event_sink(AgentEvent::Output(output));
                }
                StreamChunk::Thought(thought) => {
                    event_sink(AgentEvent::Thought(thought));
                }
                StreamChunk::ToolCall { id, name, arguments } => {
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
                    let total_input = input_tokens + cache_read_tokens;
                    let cache_hit_rate = if total_input > 0 {
                        (cache_read_tokens as f32 / total_input as f32) * 100.0
                    } else {
                        0.0
                    };
                    eprintln!(
                        "CVH [cache] turn={} model={} input={} output={} cache_read={} cache_creation={} hit_rate={:.1}% prefix_hash={:016x}",
                        turn,
                        config.model,
                        input_tokens,
                        output_tokens,
                        cache_read_tokens,
                        cache_creation_tokens,
                        cache_hit_rate,
                        prefix_hash
                    );
                    event_sink(AgentEvent::Telemetry {
                        input_tokens,
                        output_tokens,
                        cache_read_tokens,
                        cache_creation_tokens,
                        cache_hit_rate,
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
                event_sink(AgentEvent::Thought(text));
            } else {
                turn_content.push_str(&text);
                event_sink(AgentEvent::Token(text));
            }
        }

        // If no tool calls, terminal response reached
        if turn_tool_calls.is_empty() {
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

        // Execute each tool call and record response
        for tc in turn_tool_calls {
            let parsed_args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| serde_json::json!({}));

            let mut call_tool_ctx = tool_ctx.clone();
            call_tool_ctx.call_id = Some(&tc.id);

            let exec_result = execute_tool(
                &call_tool_ctx,
                &tc.function.name,
                &parsed_args,
            )
            .await;

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
                Err(e) => (format!("Error executing {}: {}", tc.function.name, e), false),
            };

            event_sink(AgentEvent::ToolResult {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                result: result_str.clone(),
                success,
            });

            messages.push(ChatMessage::tool_response(tc.id, result_str));
        }

        // Sync messages to in-memory context state
        if let Some(ref app) = config.app_handle {
            if let Some(state) = app.try_state::<crate::commands::AppState>() {
                if let Ok(map) = state.agent_contexts.lock() {
                    if let Some(cs) = map.get(&config.task_id) {
                        if let Ok(mut guard) = cs.lock() {
                            guard.current_turn = turn;
                            guard.messages = messages.clone();
                        }
                    }
                }
            }
        }
    }

    // Недостижимо: цикл выше бесконечен и выходит только через
    // return (терминальный ответ), bail! (abort/ошибка) внутри.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_prompt_is_lean_cvh_only() {
        // Regression: only `.cvh` persona + tool instructions reach the model.
        // Root `AGENTS.md` and `.agents/skills` must NOT be injected even when
        // present on disk (user mandate: send the model ONLY .cvh + persona).
        let base =
            std::path::PathBuf::from("target").join(format!("cvh_prompt_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh").join("agents").join("developer"))
            .await
            .expect("test dirs");
        tokio::fs::write(
            base.join(".cvh").join("agents").join("developer").join("persona.md"),
            "PERSONA_MARKER_ALPHA",
        )
        .await
        .expect("persona seed");
        tokio::fs::write(base.join("AGENTS.md"), "AGENTS_MARKER_BETA")
            .await
            .expect("agents seed");
        let skill_dir = base.join(".agents").join("skills").join("demo");
        tokio::fs::create_dir_all(&skill_dir)
            .await
            .expect("skill dirs");
        tokio::fs::write(skill_dir.join("SKILL.md"), "SKILL_MARKER_GAMMA")
            .await
            .expect("skill seed");

        let prompt = compile_primed_system_prompt(&base, "developer")
            .await
            .expect("prompt failed");

        assert!(
            prompt.contains("PERSONA_MARKER_ALPHA"),
            "persona from .cvh/agents must be present"
        );
        assert!(
            !prompt.contains("AGENTS_MARKER_BETA"),
            "root AGENTS.md must NOT leak into the system prompt"
        );
        assert!(
            !prompt.contains("SKILL_MARKER_GAMMA"),
            ".agents/skills must NOT leak into the system prompt"
        );
        assert!(
            !prompt.contains("Active Agent Skills"),
            "skills section must be gone"
        );
        assert!(
            prompt.contains("Scoped Tool Execution Mandate"),
            "tool instructions must be kept"
        );

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_system_prompt_flat_persona_rules_alphabetical() {
        // Single flat persona `.cvh/agents/<role>.md` + rules/*.md (alphabetical)
        // + tool instructions.
        let base =
            std::path::PathBuf::from("target").join(format!("cvh_rules_{}", uuid::Uuid::new_v4()));
        let agents = base.join(".cvh").join("agents");
        let role_dir = agents.join("dev");
        tokio::fs::create_dir_all(role_dir.join("rules"))
            .await
            .expect("test dirs");
        tokio::fs::write(role_dir.join("persona.md"), "FLAT_PERSONA_MARKER")
            .await
            .expect("persona seed");
        // Seed out of order: b before a — prompt must order a, b.
        tokio::fs::write(role_dir.join("rules").join("b_second.md"), "RULE_B_MARKER")
            .await
            .expect("rule b");
        tokio::fs::write(role_dir.join("rules").join("a_first.md"), "RULE_A_MARKER")
            .await
            .expect("rule a");
        tokio::fs::write(role_dir.join("rules").join("notes.txt"), "IGNORED")
            .await
            .expect("non-md seed");

        let prompt = compile_primed_system_prompt(&base, "dev")
            .await
            .expect("prompt failed");

        assert!(prompt.contains("FLAT_PERSONA_MARKER"));
        assert!(prompt.contains("RULE_A_MARKER"));
        assert!(prompt.contains("RULE_B_MARKER"));
        assert!(!prompt.contains("IGNORED"), "non-md rules must be skipped");
        let pos_a = prompt.find("RULE_A_MARKER").expect("rule a pos");
        let pos_b = prompt.find("RULE_B_MARKER").expect("rule b pos");
        assert!(pos_a < pos_b, "rules must be alphabetical: a_first before b_second");
        assert!(prompt.contains("Scoped Tool Execution Mandate"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_system_prompt_flat_persona_and_builtin_fallback() {
        // Directory-only persona path: <role>/persona.md or <role>/agent.md.
        // Flat .md files are strictly ignored.
        let base =
            std::path::PathBuf::from("target").join(format!("cvh_fb_{}", uuid::Uuid::new_v4()));
        let agents = base.join(".cvh").join("agents");
        tokio::fs::create_dir_all(&agents)
            .await
            .expect("test dirs");

        // 1. persona.md in role directory
        let custom_dir = agents.join("custom");
        tokio::fs::create_dir_all(&custom_dir).await.expect("custom dir");
        tokio::fs::write(custom_dir.join("persona.md"), "PERSONAMARKER")
            .await
            .expect("persona seed");
        let with_persona = compile_primed_system_prompt(&base, "custom")
            .await
            .expect("prompt failed");
        assert!(with_persona.contains("PERSONAMARKER"), "custom persona.md must be used");

        // 2. agent.md in role directory
        let alt_dir = agents.join("alt");
        tokio::fs::create_dir_all(&alt_dir).await.expect("alt dir");
        tokio::fs::write(alt_dir.join("agent.md"), "AGENTMARKER")
            .await
            .expect("agent seed");
        let with_agent = compile_primed_system_prompt(&base, "alt")
            .await
            .expect("prompt failed");
        assert!(with_agent.contains("AGENTMARKER"), "alt agent.md must be used");

        // 3. Flat dev.md must NOT be picked up (directory-only structure)
        tokio::fs::write(agents.join("dev.md"), "FLATMARKER")
            .await
            .expect("flat seed");
        let fallback_dev = compile_primed_system_prompt(&base, "dev")
            .await
            .expect("prompt failed");
        assert!(!fallback_dev.contains("FLATMARKER"), "flat dev.md must be ignored");

        // 4. developer role without persona on disk → rich built-in developer fallback
        let dev_builtin = compile_primed_system_prompt(&base, "developer")
            .await
            .expect("prompt failed");
        assert!(
            dev_builtin.contains("developer (Software Engineer)") && dev_builtin.contains("Ground Truth Sensors"),
            "rich developer fallback must fire, got: {}",
            &dev_builtin[..dev_builtin.len().min(200)]
        );

        // 5. ghost role without persona on disk → generic expert AI agent
        let builtin = compile_primed_system_prompt(&base, "ghost")
            .await
            .expect("prompt failed");
        assert!(
            builtin.contains("You are an expert AI agent in ghost."),
            "built-in fallback must fire, got: {}",
            &builtin[..builtin.len().min(200)]
        );

        // cvh-dev without file on disk → rich builtin prompt
        let cvh_dev = compile_primed_system_prompt(&base, "cvh-dev")
            .await
            .expect("prompt failed");
        assert!(cvh_dev.contains("cvh-dev"));
        assert!(cvh_dev.contains("Columns = Agents"));
        assert!(cvh_dev.contains("boards.json"));
        assert!(cvh_dev.contains("pipeline.json"));
        assert!(cvh_dev.contains("APPROVED_WITH_CAVEATS"));
        assert!(cvh_dev.contains("Instructions for Empty & New Projects"));
        // Lean-router invariants (≤150 lines, frontmatter, paths, skeletons, guards).
        assert!(
            BUILTIN_CVH_DEV_PROMPT.lines().count() <= 150,
            "router must stay ≤150 lines, got {}",
            BUILTIN_CVH_DEV_PROMPT.lines().count()
        );
        for marker in [
            "name: <role>",
            "description: Use PROACTIVELY when",
            "model: inherit",
            "tools:",
            "scope:",
            "activation:",
            ".cvh/boards.json {activeBoardId",
            ".cvh/pipeline.json {id,title,columns[",
            ".cvh/settings/tool-policy.json {globalEnabled, stageOverrides}",
            ".cvh/agents/<role>.md",
            ".cvh/tools/<tool>.json",
            ".cvh/hooks/",
            "worktrees/<slug>",
            "activeBoardId",
            "contextWindow",
            "gatekeeperSensor",
            "terminal",
            "globalEnabled && stageOverrides",
            "commands.rs:47",
            "resolve_safe_path",
            "Verification[evidence]",
            "Red Flags",
            "30 lines",
        ] {
            assert!(
                cvh_dev.contains(marker),
                "router prompt must contain marker: {}",
                marker
            );
        }

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_agent_tool_intersection_with_policy() {
        // Effective tool set = policy lookup ∩ agent allowed_tools.
        use crate::config::{is_tool_allowed, ToolPolicyConfig};
        let policy = ToolPolicyConfig::default();

        // Agent restricts to read-only: schema filter must drop the rest
        // even though the matrix allows them.
        let agent_allowed = Some(vec!["read_file".to_string(), "glob".to_string()]);
        let all = get_all_tools(&std::path::PathBuf::from("target")).await;
        assert!(!all.is_empty());
        let active: Vec<String> = all
            .iter()
            .filter(|t| {
                is_tool_allowed(&policy, Some("dev"), &t.function.name)
                    && agent_allows_tool(agent_allowed.as_deref(), &t.function.name)
            })
            .map(|t| t.function.name.clone())
            .collect();
        assert!(active.contains(&"read_file".to_string()));
        assert!(active.contains(&"glob".to_string()));
        assert!(!active.contains(&"execute_command".to_string()));
        assert!(!active.contains(&"write_file".to_string()));

        // No agent restriction → matrix alone decides.
        let open: Vec<String> = all
            .iter()
            .filter(|t| {
                is_tool_allowed(&policy, Some("dev"), &t.function.name)
                    && agent_allows_tool(None, &t.function.name)
            })
            .map(|t| t.function.name.clone())
            .collect();
        assert!(open.len() > active.len());

        // Zero-trust gate: agent-denied tool errors even when policy allows.
        let flag = Arc::new(AtomicBool::new(false));
        let workdir = std::env::temp_dir();
        let target_root = std::path::PathBuf::from("target");
        let tool_ctx = ToolContext {
            repo_root: &target_root,
            worktree_dir: &workdir,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: None,
            agent_allowed_tools: agent_allowed.as_deref(),
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };
        let err = execute_tool(
            &tool_ctx,
            "write_file",
            &serde_json::json!({"file_path": "x.txt", "content": "y"}),
        )
        .await
        .expect_err("agent-denied tool must be rejected");
        assert!(err.to_string().contains("PermissionDenied"), "unexpected: {}", err);
    }
    #[tokio::test]
    async fn test_subagent_session_and_receiver_lookup() {
        let (tx1, rx1) = oneshot::channel::<String>();
        let flag = Arc::new(AtomicBool::new(false));
        let workdir = std::env::temp_dir();
        let target_root = std::path::PathBuf::from("target");
        let policy = ToolPolicyConfig::default();

        let ctx = OwnedToolContext {
            repo_root: target_root,
            worktree_dir: workdir,
            stage_id: Some("dev".to_string()),
            role: Some("developer".to_string()),
            abort_handle: flag,
            tool_policy: policy,
            task_id: Some("CVH-100".to_string()),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: Some("call_sub_123".to_string()),
        };

        let sub_id = "CVH-100.sub-call_sub_123".to_string();
        let retry_info = SubagentRetryInfo {
            parent_task_id: "CVH-100".to_string(),
            subagent_id: sub_id.clone(),
            tool_call_id: Some("call_sub_123".to_string()),
            title: "Test subagent".to_string(),
            role: "developer".to_string(),
            prompt: "Test prompt".to_string(),
            requested_tools: None,
            ctx,
            status: "running".to_string(),
            last_error: None,
        };

        let session = SubagentSession {
            sender: Arc::new(tokio::sync::Mutex::new(Some(tx1))),
            retry_info: Arc::new(tokio::sync::Mutex::new(retry_info)),
        };

        {
            let mut sessions = SUBAGENT_SESSIONS.lock().await;
            sessions.insert(sub_id.clone(), session);
        }

        // Test 1: find by exact subagent_id
        let res1 = find_subagent_session(Some("CVH-100"), "CVH-100.sub-call_sub_123").await;
        assert!(res1.is_some());
        assert_eq!(res1.as_ref().map(|s| s.0.as_str()), Some(sub_id.as_str()));

        // Test 2: find by stripped :sub: dialog ID
        let res2 = find_subagent_session(Some("CVH-100"), "CVH-100:sub:CVH-100.sub-call_sub_123").await;
        assert!(res2.is_some());
        assert_eq!(res2.as_ref().map(|s| s.0.as_str()), Some(sub_id.as_str()));

        // Test 3: find by tool_call_id alone
        let res3 = find_subagent_session(Some("CVH-100"), "call_sub_123").await;
        assert!(res3.is_some());
        assert_eq!(res3.as_ref().map(|s| s.0.as_str()), Some(sub_id.as_str()));

        // Test 4: find by .sub-call_sub_123
        let res4 = find_subagent_session(None, ".sub-call_sub_123").await;
        assert!(res4.is_some());
        assert_eq!(res4.as_ref().map(|s| s.0.as_str()), Some(sub_id.as_str()));

        // Test remove_subagent_receiver
        let mut rec_map = HashMap::new();
        rec_map.insert(sub_id.clone(), rx1);

        let retrieved = remove_subagent_receiver(&mut rec_map, "call_sub_123");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.as_ref().map(|s| s.0.as_str()), Some(sub_id.as_str()));
        assert!(rec_map.is_empty());
    }

    #[tokio::test]
    async fn test_system_prompt_subagent_delegation_mandate() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_sub_mandate_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh").join("agents"))
            .await
            .expect("test dirs");
        tokio::fs::write(
            base.join(".cvh").join("agents").join("developer.md"),
            "PERSONA_SUB_TEST",
        )
        .await
        .expect("persona seed");

        let prompt = compile_primed_system_prompt(&base, "developer")
            .await
            .expect("prompt failed");

        assert!(prompt.contains("## Subagent Delegation Mandate"));
        assert!(prompt.contains("prefer `subagent` (oneshot sync)"));
        assert!(prompt.contains("MUST call `collect_subagent`"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }
    #[tokio::test]
    async fn test_orchestrator_create_task_and_create_tasks() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_orch_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        // Seed parent orchestrator task CVH-001
        let parent_task = Task {
            id: "CVH-001".to_string(),
            title: "Parent Orchestrator Task".to_string(),
            description: "Decomposing feature".to_string(),
            stage_id: "backlog".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: Some("high".to_string()),
            tags: Some(vec!["orchestrator".to_string()]),
            created_at: 0,
            updated_at: 0,
        };
        crate::commands::tasks::save_tasks_to_disk(&base, &[parent_task]).await.expect("save parent");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("backlog"),
            role: Some("orchestrator"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-001"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Single create_task call
        let single_args = serde_json::json!({
            "title": "Backend Rust Implementation",
            "description": "Implement create_task and complete_task tools",
            "column_id": "dev",
            "priority": "high",
            "tags": ["rust", "tauri"]
        });
        let single_res = execute_tool(&tool_ctx, "create_task", &single_args)
            .await
            .expect("create_task execution");

        let parsed_single: serde_json::Value = serde_json::from_str(&single_res).expect("parse single json");
        assert_eq!(parsed_single["status"], "created");
        assert_eq!(parsed_single["task_id"], "CVH-002");
        assert_eq!(parsed_single["title"], "Backend Rust Implementation");
        assert_eq!(parsed_single["column_id"], "dev");

        // Verify on disk that CVH-002 has parent_id = "CVH-001"
        let tasks_on_disk = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let created_child = tasks_on_disk.iter().find(|t| t.id == "CVH-002").expect("CVH-002 on disk");
        assert_eq!(created_child.parent_id.as_deref(), Some("CVH-001"));
        assert_eq!(created_child.priority.as_deref(), Some("high"));
        assert_eq!(created_child.tags.as_ref().unwrap(), &vec!["rust".to_string(), "tauri".to_string()]);

        // 2. Batch create_tasks call
        let batch_args = serde_json::json!({
            "tasks": [
                {
                    "title": "Frontend UI Matrix",
                    "description": "Integrate into toolPolicyStore.ts",
                    "column_id": "dev",
                    "priority": "medium"
                },
                {
                    "title": "End-to-End Test Suite",
                    "description": "Run cargo check and vitest",
                    "column_id": "dev",
                    "priority": "high"
                }
            ]
        });
        let batch_res = execute_tool(&tool_ctx, "create_tasks", &batch_args)
            .await
            .expect("create_tasks execution");

        let parsed_batch: serde_json::Value = serde_json::from_str(&batch_res).expect("parse batch json");
        assert_eq!(parsed_batch["status"], "created");
        assert_eq!(parsed_batch["count"], 2);

        let tasks_after_batch = crate::commands::tasks::load_tasks_from_disk(&base).await;
        assert_eq!(tasks_after_batch.len(), 4); // CVH-001, CVH-002, CVH-003, CVH-004

        let t3 = tasks_after_batch.iter().find(|t| t.id == "CVH-003").expect("CVH-003");
        let t4 = tasks_after_batch.iter().find(|t| t.id == "CVH-004").expect("CVH-004");
        assert_eq!(t3.parent_id.as_deref(), Some("CVH-001"));
        assert_eq!(t4.parent_id.as_deref(), Some("CVH-001"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_complete_task_anti_bypass_invariant() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_antibypass_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        // Seed a regular developer task CVH-010 that has NO child tasks
        let regular_task = Task {
            id: "CVH-010".to_string(),
            title: "Regular Developer Task".to_string(),
            description: "Working on code".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            created_at: 0,
            updated_at: 0,
        };
        crate::commands::tasks::save_tasks_to_disk(&base, &[regular_task]).await.expect("save regular task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let dev_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-010"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Regular dev agent attempting to bypass pipeline with complete_task MUST BE REJECTED
        let bypass_err = execute_tool(&dev_ctx, "complete_task", &serde_json::json!({
            "summary": "Trying to skip review and retro"
        }))
        .await
        .expect_err("Regular dev agent must be rejected from complete_task");

        assert!(
            bypass_err.to_string().contains("complete_task is restricted to orchestrator tasks that created subtasks."),
            "Error must match Anti-Bypass Invariant string, got: {}",
            bypass_err
        );

        // Verify task is still in 'dev' with status 'running'
        let tasks_check1 = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t10 = tasks_check1.iter().find(|t| t.id == "CVH-010").unwrap();
        assert_eq!(t10.stage_id, "dev");
        assert_eq!(t10.status.as_deref(), Some("running"));

        // 2. Now let's spawn a child task linked to CVH-010
        let child_task = Task {
            id: "CVH-011".to_string(),
            title: "Child Subtask".to_string(),
            description: "Subtask 1".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
            model: None,
            parent_id: Some("CVH-010".to_string()),
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            created_at: 0,
            updated_at: 0,
        };
        crate::commands::tasks::save_tasks_to_disk(&base, &[t10.clone(), child_task]).await.expect("save with child");

        // Now CVH-010 has spawned child tasks -> complete_task MUST SUCCEED
        let complete_res = execute_tool(&dev_ctx, "complete_task", &serde_json::json!({
            "summary": "Decomposition finished and subtasks spawned"
        }))
        .await
        .expect("complete_task must succeed for task with child subtasks");

        let parsed_res: serde_json::Value = serde_json::from_str(&complete_res).expect("parse complete json");
        assert_eq!(parsed_res["task_id"], "CVH-010");
        assert_eq!(parsed_res["status"], "done");
        assert_eq!(parsed_res["column_id"], "ready");

        // Verify on disk that CVH-010 moved to 'ready', status is 'done', and handover_summary is recorded
        let tasks_final = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t10_done = tasks_final.iter().find(|t| t.id == "CVH-010").unwrap();
        assert_eq!(t10_done.stage_id, "ready");
        assert_eq!(t10_done.status.as_deref(), Some("done"));
        assert_eq!(t10_done.handover_summary.as_deref(), Some("Decomposition finished and subtasks spawned"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

}
