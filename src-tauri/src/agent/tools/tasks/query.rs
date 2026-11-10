use anyhow::{Context, Result};
use serde::Serialize;

use super::params::*;
use crate::agent::{get_stage_personas, resolve_task_role, ToolContext};

pub async fn execute_list_tasks(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;

    let mut tasks = crate::db::db_get_all_tasks(repo_root)
        .map_err(|e| anyhow::anyhow!("Failed to read tasks from database: {}", e))?;

    if let Some(target_stage) = get_str_param(args, STAGE_KEYS) {
        tasks.retain(|t| t.stage_id.eq_ignore_ascii_case(target_stage));
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

    Ok(serde_json::to_string_pretty(&compact_list)?)
}

pub async fn execute_get_task_history(
    ctx: &ToolContext<'_>,
    args: &serde_json::Value,
) -> Result<String> {
    let repo_root = ctx.repo_root;

    let target_task_id = get_str_param(args, TASK_ID_KEYS)
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
            "steer" | "system" | "prompt" | "agent" => {
                let content = it.content.as_deref().unwrap_or("").trim();
                if !content.is_empty() {
                    let label = if content.contains("[ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ") || content.contains("HANDOVER SUMMARY") {
                        "Agent Handover"
                    } else if content.contains("SENSOR REJECTED") || content.contains("ОТКЛОНЕНО СЕНСОРОМ") {
                        "Sensor Rejection"
                    } else if content.contains("HOOK REJECTED") || content.contains("ОТКЛОНЕНО ХУКОМ") {
                        "Hook Rejection"
                    } else if content.contains("Задача возвращена на этап") {
                        "Rejection"
                    } else if it.item_type == "agent" {
                        "Agent Message"
                    } else {
                        "User Steer / System"
                    };
                    report.push_str(&format!("- Шаг {} [{}]: {}\n", step_num, label, content));
                    step_num += 1;
                }
            }
            "tool" => {
                let (tool_name, params_str, status_str, duration, error_snippet, output_snippet) = if let Some(ref t) = it.tool {
                    let name = get_str_param(t, &["toolName", "name"]).unwrap_or("unknown");
                    let params = t.get("params").or_else(|| t.get("arguments")).cloned().unwrap_or_else(|| serde_json::json!({}));
                    let params_str = serde_json::to_string(&params).unwrap_or_else(|_| "{}".to_string());
                    let status = get_str_param(t, &["status"]).unwrap_or("success");
                    let duration = t.get("durationMs").and_then(|v| v.as_f64()).or(it.duration_ms).unwrap_or(0.0);
                    let err = get_str_param(t, &["stderr", "error"])
                        .or_else(|| if status == "failed" { it.content.as_deref() } else { None })
                        .unwrap_or("none");
                    let out = get_str_param(t, &["stdout", "result"])
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

    Ok(report)
}
