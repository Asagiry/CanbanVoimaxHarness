use crate::llm::ToolDefinition;
use std::path::Path;

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

/// Extracts the 1-sentence description from YAML frontmatter (`description: ...`).
pub fn extract_frontmatter_description(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    let rest = trimmed.strip_prefix("---")?;
    let end_idx = rest.find("\n---")?;
    let fm_block = &rest[..end_idx];
    for line in fm_block.lines() {
        let trimmed_line = line.trim();
        if let Some(val) = trimmed_line.strip_prefix("description:") {
            let cleaned = val.trim().trim_matches(|c| c == '\'' || c == '"').trim();
            if !cleaned.is_empty() {
                return Some(cleaned.to_string());
            }
        }
    }
    None
}

/// Discovers available rules for an agent, collecting their stems and optional frontmatter descriptions.
/// Checks `<repo_root>/.cvh/agents/<role>/rules/*.md` and global shared `<repo_root>/.cvh/rules/*.md`.
pub fn get_available_rules_with_descriptions(
    repo_root: &Path,
    role: Option<&str>,
) -> Vec<(String, Option<String>)> {
    let mut rules: Vec<(String, Option<String>)> = Vec::new();
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
                            let desc = std::fs::read_to_string(&path)
                                .ok()
                                .and_then(|text| extract_frontmatter_description(&text));
                            rules.push((stem.to_string(), desc));
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
                                        let desc = std::fs::read_to_string(&path)
                                            .ok()
                                            .and_then(|text| extract_frontmatter_description(&text));
                                        rules.push((stem.to_string(), desc));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    rules.sort_by(|a, b| a.0.cmp(&b.0));
    rules.dedup_by(|a, b| a.0 == b.0);
    rules
}

pub fn get_available_rules(repo_root: &Path, role: Option<&str>) -> Vec<String> {
    get_available_rules_with_descriptions(repo_root, role)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
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
