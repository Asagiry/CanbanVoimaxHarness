pub mod compactor;
pub use compactor::*;

use crate::config::{
    agent_allows_tool, is_tool_allowed, load_agent_setting, load_tool_policies,
    ToolPolicyConfig,
};
use crate::hooks::kill_process_tree;
use crate::llm::{ChatMessage, LlmClient, OpenAIChatRequest, StreamChunk, ToolCall, ToolDefinition};
use anyhow::{bail, Context, Result};
use tauri::{AppHandle, Emitter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;

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
            "Delegate a focused, independent sub-task to a specialized subagent.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "role": { "type": "string", "description": "Role persona for subagent (e.g. 'reviewer', 'developer')." },
                    "prompt": { "type": "string", "description": "Detailed task instructions." }
                },
                "required": ["role", "prompt"]
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
            let role = args
                .get("role")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'role'")?;
            let prompt = args
                .get("prompt")
                .and_then(|v| v.as_str())
                .context("Missing required parameter 'prompt'")?;
            format!("Subagent [{}] dispatched with task: {}", role, prompt)
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
        if norm.ends_with("boards.json") {
            if let Ok(bc) = crate::config::load_boards_config(ctx.repo_root).await {
                let _ = app.emit(
                    "boards:updated",
                    serde_json::json!({ "boards": &bc.boards, "activeBoardId": &bc.active_board_id }),
                );
            }
            if let Ok(pipe) = crate::config::load_pipeline_config(ctx.repo_root).await {
                let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
            }
        } else if norm.ends_with("pipeline.json") {
            if let Ok(pipe) = crate::config::load_pipeline_config(ctx.repo_root).await {
                let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
            }
            if let Ok(bc) = crate::config::load_boards_config(ctx.repo_root).await {
                let _ = app.emit(
                    "boards:updated",
                    serde_json::json!({ "boards": &bc.boards, "activeBoardId": &bc.active_board_id }),
                );
            }
        } else if norm.contains(".cvh/agents") || norm.contains("/agents/") {
            let list = crate::commands::list_agent_personas_in(ctx.repo_root).await;
            let _ = app.emit("agent:personas_updated", &list);
        }
    }
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
        c.args(["/C", &format!("chcp 65001 >nul && {}", cmd_str)]);
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

        // State-Preserving Context Compactor: evaporate bloated tool outputs >3 turns old.
        // Prefix-cache guard: mutating old tool outputs rewrites the prompt prefix and
        // invalidates Gemini implicit caching from that byte onward. Fire only on real
        // need (large estimated context), not on message count, to keep prefix stable.
        if turn > 3 && compactor::estimate_tokens(&messages) > 40_000 {
            let compaction_res = compactor::compact_conversation(&mut messages, None);
            if compaction_res.saved_chars > 0 {
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
        };

        // Execute each tool call and record response
        for tc in turn_tool_calls {
            let parsed_args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| serde_json::json!({}));

            let exec_result = execute_tool(
                &tool_ctx,
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
        tokio::fs::create_dir_all(base.join(".cvh").join("agents"))
            .await
            .expect("test dirs");
        tokio::fs::write(
            base.join(".cvh").join("agents").join("developer.md"),
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
        tokio::fs::write(agents.join("dev.md"), "FLAT_PERSONA_MARKER")
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
}
