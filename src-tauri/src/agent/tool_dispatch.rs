use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use crate::config::{agent_allows_tool, is_tool_allowed};
use crate::agent::prompts::is_tool_active_for_agent;
use crate::agent::spill::maybe_spill_output;
use crate::agent::subagent_runtime::{canonical_tool_name, ToolContext};
use crate::agent::tools::*;

pub async fn get_stage_personas(repo_root: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(boards_cfg) = crate::config::load_boards_config(repo_root).await {
        if let Some(active) = boards_cfg.boards.iter().find(|b| b.id == boards_cfg.active_board_id) {
            for c in &active.stages {
                if let Some(ref p) = c.persona {
                    map.insert(c.id.clone(), p.clone());
                }
            }
        }
        for b in &boards_cfg.boards {
            if b.id != boards_cfg.active_board_id {
                for c in &b.stages {
                    if let Some(ref p) = c.persona {
                        map.entry(c.id.clone()).or_insert_with(|| p.clone());
                    }
                }
            }
        }
    }
    if let Ok(pipeline) = crate::config::load_pipeline_config(repo_root).await {
        for c in pipeline.stages {
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
            "retro" => "retro".to_string(),
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
        "create_task" | "create_tasks" => {
            let is_batch = tool_name == "create_tasks"
                || args.get("tasks").and_then(|v| v.as_array()).is_some();
            execute_create_task(ctx, args, is_batch).await?
        }
        "complete_task" => execute_complete_task(ctx, args).await?,
        "list_tasks" => execute_list_tasks(ctx, args).await?,
        "get_task_history" => execute_get_task_history(ctx, args).await?,
        "load_rule" => execute_load_rule(ctx, args).await?,
        custom_tool => execute_custom_tool(ctx, custom_tool, args).await?,
    };

    if abort_handle.load(Ordering::SeqCst) {
        bail!("Operation aborted");
    }

    maybe_spill_output(repo_root, Some(ctx.worktree_dir), normalized_tool_name, raw_result).await
}
