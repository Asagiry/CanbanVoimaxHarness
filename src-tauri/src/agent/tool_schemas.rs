use crate::llm::ToolDefinition;
use std::path::Path;

pub fn get_standard_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition::function(
            "read_file",
            "Read UTF-8 text file contents with line numbers from the isolated worktree. (Batch-friendly: emit multiple read_file calls concurrently in one turn to inspect several files at once).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative file path within worktree (or 'path')." },
                    "offset": { "type": "integer", "description": "1-based starting line number (default: 1)." },
                    "limit": { "type": "integer", "description": "Maximum number of lines to return (default: 200)." }
                },
                "required": ["file_path"]
            }),
        ),
        ToolDefinition::function(
            "write_file",
            "Write complete UTF-8 content to a file inside the isolated worktree. (Batch-friendly: safe to emit concurrently alongside writes to other distinct files).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative file path within worktree (or 'path')." },
                    "content": { "type": "string", "description": "Complete file text content to write." }
                },
                "required": ["file_path", "content"]
            }),
        ),
        ToolDefinition::function_with_aliases(
            "edit_file",
            "Targeted text replacement in an existing file inside the isolated worktree. (Batch-friendly: safe to emit concurrently alongside edits to other distinct files).",
            vec!["str_replace".to_string()],
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
            "Find files matching a glob pattern inside the isolated worktree. (Batch-friendly: safe to emit concurrently alongside other search/read operations).",
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
            "Search file contents line by line with a Rust regular expression inside the isolated worktree. Use | for alternatives; returns at most 200 matching lines. (Batch-friendly: emit multiple grep/read_file calls concurrently to search different patterns or paths).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Rust regex: foo|bar, ^prefix, [A-Z]+, or (?i)name for case-insensitive search. No look-around or backreferences." },
                    "path": { "type": "string", "description": "Directory or file to search (default: '.')." }
                },
                "required": ["pattern"]
            }),
        ),
        ToolDefinition::function_with_aliases(
            "execute_command",
            "Execute a shell command inside the isolated worktree with piped stdout/stderr.",
            vec!["bash".to_string()],
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command string to run (e.g. 'cargo check', 'git status')." },
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
                    "persona": { "type": "string", "description": "Target persona/role for subagent" },
                    "role": { "type": "string", "description": "Target persona/role for subagent (backward-compatible alias for 'persona'). Default: 'general'." },
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
            "Launch a specialized subagent in the background (asynchronous). Returns immediately with a subagent_id and status 'running'. (Batch-friendly: emit multiple subagent_bg calls in one turn for parallel execution across decoupled modules, then collect them via collect_subagent).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Short task description (e.g. 'Run long benchmark', 'Deep search repo')." },
                    "persona": { "type": "string", "description": "Target persona/role for subagent" },
                    "role": { "type": "string", "description": "Target persona/role for subagent (backward-compatible alias for 'persona'). Default: 'general'." },
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
            "Await the completion of one or ALL background subagents (launched via subagent_bg) and retrieve their final reports in one call without polling. When multiple subagents were launched, ALWAYS pass all of them together via 'subagent_ids' (array of strings) in a SINGLE call rather than collecting one by one across turns. Do not batch unrelated tools in the same turn.",
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
        ToolDefinition::function_with_aliases(
            "subagent_list",
            "List all background and synchronous subagents for the current task with their status (running, completed, aborted), role, title, steps, and subagent_id for collection.",
            vec!["list_subagents".to_string()],
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
        ToolDefinition::function_with_aliases(
            "advance_task",
            "Advance task to the next sequential pipeline stage or transition to a specific stage (stage_id / target_stage). Unfinished TODO items will be auto-completed on advance; mark them completed via todo_write first for accurate progress. Requires comprehensive summary of completed work and verification proof.",
            vec![
                "move_task".to_string(),
                "move_task_stage".to_string(),
                "advance_stage".to_string(),
                "next_stage".to_string(),
            ],
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
        ToolDefinition::function_with_aliases(
            "question",
            "Ask an interactive clarifying question to the human user and suspend agent execution until answered (Human-in-the-Loop). 'question' is accepted as an alias for 'text'. If the user has already answered this question in this task, do not call question again with the same text — use the recorded answer.",
            vec!["ask_user".to_string()],
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "The specific question to ask the user." },
                    "question": { "type": "string", "description": "Alias for 'text' — the specific question to ask the user." },
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
        ToolDefinition::function_with_aliases(
            "create_task",
            "Decompose requirements and create a new task card on the Kanban board.",
            vec!["create_tasks".to_string()],
            serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Concise imperative task title." },
                    "description": { "type": "string", "description": "Detailed technical requirements, acceptance criteria, and file targets." },
                    "stage_id": { "type": "string", "description": "Target stage/column ID (e.g. 'backlog', 'plan', 'dev'). Defaults to 'backlog'." },
                    "column_id": { "type": "string", "description": "Legacy alias for stage_id." },
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
                                "stage_id": { "type": "string", "description": "Target stage/column ID (e.g. 'backlog', 'plan', 'dev'). Defaults to 'backlog'." },
                                "column_id": { "type": "string", "description": "Legacy alias for stage_id." },
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

pub use crate::agent::rule_tools::*;

