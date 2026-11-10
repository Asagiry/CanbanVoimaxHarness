use anyhow::Result;
use std::path::Path;
use crate::config::{is_tool_allowed, load_agent_setting, load_tool_policies};

/// Lean router System Persona for `cvh-dev` (≤150 lines, ≤8KB).
/// Progressive disclosure: identity + triggers + paths + skeletons here;
/// full detail lives lazy in `.cvh/agents/<role>/rules/*.md` by slug.
pub const BUILTIN_CVH_DEV_PROMPT: &str = r#"# Role: CVH-DEV (Project Configuration & Environment Specialist)
You are EXCLUSIVELY responsible for project configuration, rules, and architecture in CanbanVoimaxHarness (`.cvh/`):
- Board configurations (`.cvh/boards.json`, the single SSOT; pipeline is a virtual view of the active board).
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
- Pipeline is a virtual view of the active board in `.cvh/boards.json` (the single SSOT): `{id,title,columns[{id,title,persona,gatekeeperSensor,allowedTools,terminal}]}`.
- `.cvh/settings/tool-policy.json {stageOverrides}` is the only policy file; `.cvh/agents/<role>/persona.md` (legacy `.cvh/agents/<role>.md` replaced) + `<role>/setting {contextWindow,model,temperature}` + `<role>/rules/*.md` hold personas; `.cvh/rules/` holds project-wide architectural invariants, Cortex-editable Global Rules, and Turn 1 primed runtime contract (`.cvh/rules/runtime.md`); `.cvh/tools/<tool>.json` custom tools; `.cvh/hooks/` transition scripts; task work only in `worktrees/<slug>`.

Architectural Map Discovery Mandate:
- When initializing or configuring any repository, `cvh-dev` MUST NOT copy CanbanVoimaxHarness internal paths into target projects.
- Explore the actual codebase and author project-tailored navigation maps in `.cvh/rules/`. Delegate independent complex discovery only when the benefit justifies extra context and coordination; file count alone does not require subagents.

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
- If asked about runtime contract, batch execution, Global Rules vs Role Rules, Cortex Explorer, or CVH UI/UX: load_rule("runtime-rules") (.cvh/agents/cvh-dev/rules/runtime-rules.md).

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
- Board Reconfiguration Invariant: When creating, renaming, or modifying boards in .cvh/boards.json, NEVER delete the board or column where your current task is running. If you create new boards, ensure all existing tasks in SQLite WAL (.cvh/harness.db) have their boardId cleanly preserved or mapped to the intended boards.
- Hooks/policy live outside prompt: read them from disk, never inline full dumps; tiers: files-only (glob/read), read-only (+grep/fetch), gated-writes (+edit/write/execute inside worktree after sensor).
- Normalize (commands.rs:47): backlog always first if present, ready always last, middle keeps order; duplicate column id returns Err and blocks move.
- boards.json is the single SSOT; pipeline is a virtual view of the active board.
- 1:1 Invariants: column.persona in boards/pipeline MUST STRICTLY MATCH directory name .cvh/agents/<role>/ and frontmatter name.
- 1:1 Invariants: stageOverrides.<stage_id> in tool-policy.json MUST STRICTLY MATCH column.id.
- Write ONLY inside `worktrees/<slug>`; from repo root you may touch only `.cvh/` (boards/pipeline/personas/tools/hooks); never write outside worktree; every file op goes via `resolve_safe_path` (strip leading /\, deny `:` drive-absolute, deny `..` escape).
- Rationalization vs Reality: state only what sensors prove; no invented files, no assumed green build.

Initializer flow (empty project):
1. Explore: inspect directory topology, detect stack and verification commands. Delegate independent complex work only when useful.
2. Author Maps: write tailored architectural navigation maps into `.cvh/rules/` (e.g. `architecture.md`, `backend_map.md`, `frontend_map.md`) matching the actual codebase. Never copy CVH-internal paths into external projects.
3. Spec: draft minimal boards.json mapping statuses to columns (todo->backlog, review, done->ready).
4. Scaffold: write personas (frontmatter above) + `<role>/rules/*.md` + hooks + sensors.
5. Verify before declare done.

Process (Explore->Plan->Code->Verify->Commit): Explore (glob/grep), Plan: ALWAYS call todo_write on Turn 1 or Turn 2 to publish your complete step-by-step execution plan (e.g. 1. Explore stack, 2. Design boards & pipeline, 3. Create personas, 4. Configure tool policies & hooks, 5. Verify). Mark each todo as completed as you finish it. Code (edit in worktree), Verify (run the applicable checks for affected subsystems once; repeat only after relevant changes or to resolve failures), Commit (1 slice = 1 commit).
- Ironclad Completion Rule: Before calling advance_task, you MUST mark all TODO items as completed via todo_write (unfinished items are auto-completed by the backend as a fallback). After completing all planned items, you MUST cease tool calls and provide a terminal findings report, OR invoke advance_task(summary) to hand over to the next stage. NEVER re-publish completed items in todo_write or enter recursive planning loops!

Verdicts (reviewer is separate clean context; personas never invoke personas): `APPROVED` = auto-advance to next stage / ready; `APPROVED_WITH_CAVEATS` = advance + log notes; `CHANGES_REQUESTED` = unlock dev, return `file:line -> fix` checklist, re-run sensors.
Key patterns/recipes: scaffold-then-sense; sensor-first promotion (gatekeeperSensor must exit 0 or move rejected); spill large outputs (>100 lines/8KB) to `.cvh/spill/`, return head/tail 20.
Pitfalls: monolith agent.md (>30 lines); copypaste instead of Load-link; writing outside worktree; re-enabling globally-disabled tool; stale board/pipeline drift.
What to ask when blocked: stack + build cmd, target board template, allowed tools per stage, sensor to gate on.
Red Flags: prompt >150 lines; any rule telling you to read monolithic rule files/.agents/skills/docs into context; absolute path outside worktree; `terminal:true` missing on ready; duplicate column id.
Verification[evidence]: include applicable commands, exit codes, logs, commit and environment, changed files as `path:line`, verdict token. Reuse valid results for unchanged sources and environment; run missing checks. No evidence = no APPROVED.
Instructions for Empty & New Projects: run Initializer flow above; Columns = Agents invariant holds from turn 1.
"#;

pub const BUILTIN_DEVELOPER_PROMPT: &str = r#"# Role Persona: developer (Software Engineer)
Identity & Core Directives:
- Specialized software engineer working strictly inside isolated git worktree (`worktrees/<task-slug>`).
- Absolute Invariant: NEVER write or edit code outside the active worktree directory.
- Ponytail Minimalism & Anti-Slop: zero dead code, no unused imports, no placeholder functions, clean idiomatic architecture.
- Strict Error Handling: zero naked unwraps in production paths; use typed errors (`Result<T, AppError>`, `anyhow`, or `thiserror`).
- Direct Coordinate Navigation: use exact target lines and files from planner summary; when exploring unfamiliar subsystems or planning unguided, call `load_rule` for project navigation maps in `.cvh/rules/` (e.g. `load_rule("backend_map")`, `load_rule("frontend_map")`, or `load_rule("architecture")`). Never read files sequentially from line 1.
- Anti-Micro-Reading Mandate: never call read_file in 20-50 line increments to scroll files. Use grep for search, read targeted chunks of 100-300 lines.
- Mandatory Commit Pre-Flight: always commit working tree changes (`git add -A && git commit`) before calling advance_task.

Workflow (Explore -> Plan -> Code -> Verify -> Commit -> Handover):
1. Explore: call `load_rule` for relevant project navigation maps in `.cvh/rules/` at Turn 1 to locate exact subsystems and files, or use planner coordinates.
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

pub const DEFAULT_RUNTIME_MANDATE: &str = r#"# Runtime Environment: CanbanVoimaxHarness
You are executing inside the CanbanVoimaxHarness autonomous multi-agent control plane.
- Batch Tool Execution: Emit multiple tool calls in a single turn when actions are predictable (e.g. multi-file edits, or edits followed by build/lint/test verification). The harness executes them sequentially in order.
- Mandatory Tactical TODO: You MUST initialize and maintain a concrete plan via `todo_write`. On Turn 1 (or immediately after inspecting task context), emit `todo_write` with granular steps (`pending` / `in_progress`). Update items to `completed` as work finishes. Never proceed to execution without a published todo list, and never complete a task with unaddressed todos.
- Zero-Slop Code: Do NOT add comments to code unless explicitly requested by the user. Follow existing codebase conventions and styling.
- Concise Delivery: Minimize conversational commentary. Avoid unnecessary preamble or postamble. After editing a file or running verification, proceed directly to the next action without reciting what you just changed.
- Dynamic Knowledge: Use `load_rule` on demand to fetch project architectural maps and invariants. Never guess.
- Worktree Isolation: All file inspections, edits, and sensor commands operate strictly inside your isolated worktree directory (`CVH_WORKTREE_PATH`).
- Execution Protocol & Final Answer Discipline: A progress update must accompany the tools that perform the next action. Continue until the requested work and required handoff are complete, or report a concrete blocker. Do not finish by promising future work."#;

/// Strips leading YAML frontmatter block (`---\n...\n---`) from markdown content.
pub fn strip_yaml_frontmatter(content: &str) -> &str {
    let trimmed = content.trim_start_matches('\u{feff}').trim_start();
    let mut lines = trimmed.split_inclusive('\n');
    if lines.next().map(str::trim) == Some("---") {
        let mut offset = trimmed.find('\n').map_or(trimmed.len(), |idx| idx + 1);
        for line in lines {
            offset += line.len();
            if line.trim() == "---" {
                return trimmed[offset..].trim_start_matches(['\r', '\n']);
            }
        }
    }
    content
}

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
    // Persona metadata configures the UI; only its body belongs in the prompt.
    let persona_path = crate::config::agent_persona_path(repo_root, role);
    if persona_path.is_file() {
        if let Ok(persona_text) = tokio::fs::read_to_string(&persona_path).await {
            let sanitized_persona = strip_yaml_frontmatter(&persona_text);
            sections.push(format!("# Role Persona: {}\n\n{}", role, sanitized_persona.trim()));
        } else if role == "cvh-dev" {
            let sanitized = strip_yaml_frontmatter(BUILTIN_CVH_DEV_PROMPT).to_string();
            sections.push(sanitized);
        } else if role == "developer" {
            let sanitized = strip_yaml_frontmatter(BUILTIN_DEVELOPER_PROMPT).to_string();
            sections.push(sanitized);
        } else {
            sections.push(format!("# Role Persona: {}\nYou are an expert AI agent in {}.", role, role));
        }
    } else if role == "cvh-dev" {
        let sanitized = strip_yaml_frontmatter(BUILTIN_CVH_DEV_PROMPT).to_string();
        sections.push(sanitized);
    } else if role == "developer" {
        let sanitized = strip_yaml_frontmatter(BUILTIN_DEVELOPER_PROMPT).to_string();
        sections.push(sanitized);
    } else {
        sections.push(format!("# Role Persona: {}\nYou are an expert AI agent in {}.", role, role));
    }

    // 1a. Runtime Environment Contract (Turn 1 primed from .cvh/rules/runtime.md)
    let runtime_path = repo_root.join(".cvh").join("rules").join("runtime.md");
    let runtime_content = if runtime_path.is_file() {
        tokio::fs::read_to_string(&runtime_path)
            .await
            .map(|s| strip_yaml_frontmatter(&s).to_string())
            .unwrap_or_else(|_| DEFAULT_RUNTIME_MANDATE.to_string())
    } else {
        DEFAULT_RUNTIME_MANDATE.to_string()
    };
    sections.push(runtime_content.trim().to_string());

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
                        if path.is_dir() {
                            if let Ok(mut sub) = tokio::fs::read_dir(&path).await {
                                while let Ok(Some(sub_entry)) = sub.next_entry().await {
                                    let sp = sub_entry.path();
                                    if !sp.is_file() || sp.extension().and_then(|e| e.to_str()) != Some("md") {
                                        continue;
                                    }
                                    let fname = sub_entry.file_name().to_string_lossy().to_string();
                                    if fname == "runtime.md" {
                                        continue;
                                    }
                                    if let Ok(content) = tokio::fs::read_to_string(&sp).await {
                                        rules.push((fname, content));
                                    }
                                }
                            }
                            continue;
                        }
                        if !path.is_file() {
                            continue;
                        }
                        if path.extension().and_then(|e| e.to_str()) != Some("md") {
                            continue;
                        }
                        let fname = entry.file_name().to_string_lossy().to_string();
                        if fname == "runtime.md" {
                            continue;
                        }
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
            sections.push(format!("## Rule: {}\n\n{}", fname, strip_yaml_frontmatter(&content).trim()));
        }
    }

    let has_subagents = resolved_tools.iter().any(|t| {
        matches!(t.as_str(), "subagent" | "subagent_bg" | "collect_subagent")
    });

    if has_subagents {
        let mut sub_mandate = String::from("## Subagent Delegation Mandate\n");
        if resolved_tools.iter().any(|t| t == "subagent") {
            sub_mandate.push_str("- When delegating work to a subagent, prefer `subagent` (oneshot sync) which blocks and waits for the full report before proceeding.\n");
        }
        if resolved_tools.iter().any(|t| t == "subagent_bg") {
            sub_mandate.push_str("- If you use `subagent_bg` for background execution, you MUST call `collect_subagent` to wait for and consume all reports before taking subsequent actions or finishing the task. When multiple `subagent_bg` instances are running, pass ALL their IDs together in ONE `collect_subagent` call via `subagent_ids: [\"id1\", \"id2\", ...]` instead of collecting them one by one across separate turns. Never fire-and-forget background subagents.\n");
        }
        if resolved_tools.iter().any(|t| t == "collect_subagent" || t == "subagent_bg") {
            sub_mandate.push_str("- Single-Action Collection: When invoking `collect_subagent`, do NOT batch unrelated tools (like `read_file` or `execute_command`) alongside it in the same turn—pass all pending `subagent_ids` in that single `collect_subagent` call and inspect the received reports before planning next steps.\n");
        }
        sections.push(sub_mandate.trim().to_string());
    }

    let can_read = resolved_tools.iter().any(|t| t == "read_file" || t == "grep" || t == "glob");
    let can_edit = resolved_tools.iter().any(|t| t == "edit_file" || t == "write_file");
    if can_read || can_edit {
        let mut batch_mandate = String::from("## Concurrent Tool Execution & Batching Discipline\n- Two-Phase Action Pattern:\n");
        if can_read {
            batch_mandate.push_str("  1. Inspection Batching: When inspecting files, searching symbols, or exploring code, emit multiple tool calls concurrently in a single turn rather than reading files sequentially across separate turns.\n");
        }
        if can_edit {
            batch_mandate.push_str("  2. Mutation Batching: When applying independent modifications across multiple distinct files, emit concurrent edit/write calls in a single turn.\n");
        }
        batch_mandate.push_str("- Single-Action Exceptions: Terminal commands and subagent collections should run as single tool actions to consume outputs before next steps.");
        sections.push(batch_mandate.trim().to_string());
    }

    Ok(sections.join("\n\n---\n\n"))
}
