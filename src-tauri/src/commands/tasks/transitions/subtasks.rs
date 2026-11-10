//! Subtask Hierarchy Propagation & Relay Advancement (§ 2.2, § 2.7)

use chrono::Utc;
use crate::agent::Task;
use crate::config::{BoardsConfig, PipelineConfig};
use crate::commands::tasks::healing::{stage_id_matches, task_id_matches};
use crate::commands::tasks::storage::get_next_subtask_index;
use crate::commands::tasks::timing::{freeze_stage_duration, freeze_stage_duration_for};

#[derive(Debug, Default)]
pub struct ParentAdvanceResult {
    pub moved_events: Vec<serde_json::Value>,
    pub created_tasks: Vec<Task>,
    pub autorun_task_ids: Vec<String>,
    pub hook_finish_events: Vec<serde_json::Value>,
    pub mutated_task_ids: Vec<String>,
}

/// Propagates advancement of completed/terminal subtasks up the hierarchy to parent tasks (§ 2.2 & § 2.7).
/// Universal relay progression: if next stage is non-terminal, parent spawns next isolated child subtask;
/// if next stage is terminal (ready/done), parent moves to ready with status done.
pub fn propagate_parent_subtask_advance(
    tasks: &mut Vec<Task>,
    completed_subtask_id: &str,
    subtask_source_stage: &str,
    verdict: Option<&str>,
    boards_cfg: Option<&BoardsConfig>,
    pipeline: Option<&PipelineConfig>,
) -> ParentAdvanceResult {
    let mut moved_events = Vec::new();
    let mut created_tasks = Vec::new();
    let mut autorun_task_ids = Vec::new();
    let mut hook_finish_events = Vec::new();
    let mut mutated_task_ids = Vec::new();

    let mut current_child_id = completed_subtask_id.to_string();
    let mut current_source_stage = subtask_source_stage.to_string();

    // 1. Ensure the completed subtask has status "done"
    if let Some(sub_idx) = tasks.iter().position(|t| task_id_matches(&t.id, completed_subtask_id)) {
        tasks[sub_idx].status = Some("done".to_string());
        if tasks[sub_idx].completed_at.is_none() {
            tasks[sub_idx].completed_at = Some(Utc::now().timestamp_millis());
        }
        tasks[sub_idx].updated_at = Utc::now().timestamp_millis();
        mutated_task_ids.push(tasks[sub_idx].id.clone());
    }

    while let Some(parent_idx) = tasks.iter().position(|t| {
        if t.waiting_for_subtask_id.as_deref() == Some(&current_child_id) {
            return true;
        }
        let is_parent_of_child = tasks
            .iter()
            .find(|sub| sub.id == current_child_id)
            .and_then(|sub| sub.parent_id.as_deref())
            == Some(&t.id);
        if is_parent_of_child {
            if let Some(ref waiting_sub) = t.waiting_for_subtask_id {
                if waiting_sub != &current_child_id {
                    return false;
                }
            }
            let parent_is_terminal = stage_id_matches(&t.stage_id, "ready")
                || stage_id_matches(&t.stage_id, "done");
            if parent_is_terminal {
                return false;
            }
            return true;
        }
        false
    }) {
        let parent_id = tasks[parent_idx].id.clone();
        let old_parent_stage = tasks[parent_idx].stage_id.clone();
        let parent_board_id = tasks[parent_idx].board_id.clone();
        let parent_title = tasks[parent_idx].title.clone();

        // Resolve stages for parent task's board
        let cols = {
            let from_boards = boards_cfg.and_then(|bc| {
                if let Some(ref bid) = parent_board_id {
                    if let Some(b) = bc.boards.iter().find(|b| &b.id == bid) {
                        return Some(b.stages.clone());
                    }
                }
                if let Some(b) = bc.boards.iter().find(|b| b.id == bc.active_board_id) {
                    return Some(b.stages.clone());
                }
                bc.boards.first().map(|b| b.stages.clone())
            });
            from_boards.or_else(|| pipeline.map(|p| p.stages.clone())).unwrap_or_default()
        };

        // Find the next stage for the parent task:
        // Look up index of the subtask's source stage (e.g. "dev" or "review"):
        let sub_stage_idx = cols
            .iter()
            .position(|c| stage_id_matches(&c.id, &current_source_stage))
            .or_else(|| cols.iter().position(|c| stage_id_matches(&c.id, &old_parent_stage)));

        let next_col_opt = if let Some(idx) = sub_stage_idx {
            if idx + 1 < cols.len() {
                Some(cols[idx + 1].clone())
            } else {
                cols.last().cloned()
            }
        } else {
            cols.iter()
                .find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done"))
                .cloned()
                .or_else(|| cols.last().cloned())
        };

        let next_col = match next_col_opt {
            Some(c) => c,
            None => break,
        };

        let is_terminal = next_col.terminal == Some(true)
            || stage_id_matches(&next_col.id, "ready")
            || stage_id_matches(&next_col.id, "done");

        if is_terminal {
            let old_stage_id = tasks[parent_idx].stage_id.clone();
            let old_entered = tasks[parent_idx].stage_entered_at;
            tasks[parent_idx].waiting_for_subtask_id = None;
            tasks[parent_idx].stage_id = next_col.id.clone();
            tasks[parent_idx].verdict = Some(verdict.unwrap_or("APPROVED").to_string());
            tasks[parent_idx].status = Some("done".to_string());
            freeze_stage_duration_for(
                &mut tasks[parent_idx],
                &old_stage_id,
                old_entered,
                Utc::now().timestamp_millis(),
            );
            tasks[parent_idx].updated_at = Utc::now().timestamp_millis();
            mutated_task_ids.push(parent_id.clone());

            // Also mark any child subtasks of this parent as done so they collapse into parent (§ 3)
            for sub in tasks.iter_mut() {
                if sub.parent_id.as_deref() == Some(&parent_id) {
                    sub.status = Some("done".to_string());
                    if sub.completed_at.is_none() {
                        sub.completed_at = Some(Utc::now().timestamp_millis());
                    }
                    sub.updated_at = Utc::now().timestamp_millis();
                    mutated_task_ids.push(sub.id.clone());
                }
            }

            let parent_clone = tasks[parent_idx].clone();
            moved_events.push(serde_json::json!({
                "taskId": parent_id,
                "fromStage": old_parent_stage,
                "toStage": &next_col.id,
                "stageId": &next_col.id,
                "status": parent_clone.status,
                "verdict": parent_clone.verdict,
                "task": &parent_clone
            }));

            // If parent task also advanced to a terminal column, check if grandparent was waiting on it!
            current_child_id = parent_id;
            current_source_stage = next_col.id;
        } else {
            // Next stage is not terminal -> universal progression spawns next isolated child subtask!
            let next_subtask_idx = get_next_subtask_index(tasks, &parent_id);
            let new_subtask_id = format!("{}.{}", parent_id, next_subtask_idx);
            let now = Utc::now().timestamp_millis();

            tasks[parent_idx].waiting_for_subtask_id = Some(new_subtask_id.clone());
            tasks[parent_idx].status = Some("done".to_string());
            freeze_stage_duration(&mut tasks[parent_idx], now);
            tasks[parent_idx].updated_at = now;
            mutated_task_ids.push(parent_id.clone());

            let has_autorun = next_col.persona.is_some() || next_col.auto_run == Some(true);
            let subtask_status = if has_autorun { "running" } else { "idle" };

            let parent_wt = tasks[parent_idx].worktree_path.clone();
            let parent_branch = tasks[parent_idx].branch.clone();
            let parent_model = tasks[parent_idx].model.clone();
            let parent_skip_wt = tasks[parent_idx].skip_worktree;
            let parent_priority = tasks[parent_idx].priority.clone();

            let next_subtask_pos = tasks
                .iter()
                .filter(|t| t.stage_id == next_col.id)
                .filter_map(|t| t.position)
                .fold(0.0f64, f64::max) + 1000.0;

            let new_subtask = Task {
                id: new_subtask_id.clone(),
                title: format!("{} ({})", parent_title, next_col.title),
                description: format!(
                    "### Подзадача этапа: {}\nРодительская задача: {}\n\n{}",
                    next_col.title, parent_id, tasks[parent_idx].description
                ),
                stage_id: next_col.id.clone(),
                board_id: tasks[parent_idx].board_id.clone(),
                worktree_path: parent_wt,
                branch: parent_branch,
                verdict: None,
                custom_flow: None,
                status: Some(subtask_status.to_string()),
                model: parent_model.clone(),
                provider_id: tasks[parent_idx].provider_id.clone(),
                reasoning_effort: tasks[parent_idx].reasoning_effort.clone(),
                parent_id: Some(parent_id.clone()),
                subtask_index: Some(next_subtask_idx),
                waiting_for_subtask_id: None,
                handover_summary: None,
                cover_image: None,
                attachments: None,
                todos: None,
                priority: parent_priority,
                tags: None,
                current_tokens: None,
                skip_worktree: parent_skip_wt,
                created_at: now,
                updated_at: now,
                stage_entered_at: Some(now),
                completed_at: None,
                position: Some(next_subtask_pos),
                hook_states: tasks[parent_idx].hook_states.clone(),
                project_path: tasks[parent_idx].project_path.clone(),
                stage_durations: None,
                total_elapsed: None,
            };

            if has_autorun {
                autorun_task_ids.push(new_subtask_id.clone());
            }

            if let Some(hook_states) = tasks[parent_idx].hook_states.clone() {
                if let Some(enter) = hook_states.get("on_enter").cloned() {
                    match enter.get("status").and_then(|s| s.as_str()) {
                        Some("success") | Some("failed") => {
                            let success = enter.get("status").and_then(|s| s.as_str()) == Some("success");
                            hook_finish_events.push(serde_json::json!({
                                "taskId": new_subtask_id,
                                "stageId": next_col.id,
                                "hookType": "on_enter",
                                "success": success,
                                "exitCode": enter.get("exitCode"),
                                "stdout": enter.get("stdout").and_then(|v| v.as_str()).unwrap_or(""),
                                "stderr": enter.get("stderr").and_then(|v| v.as_str()).unwrap_or(""),
                                "error": enter.get("error"),
                                "hookName": enter.get("hookName").and_then(|v| v.as_str()).unwrap_or("on_enter"),
                            }));
                        }
                        Some("running") => {
                            hook_finish_events.push(serde_json::json!({
                                "taskId": new_subtask_id,
                                "stageId": next_col.id,
                                "hookType": "on_enter",
                                "hookName": enter.get("hookName").and_then(|v| v.as_str()).unwrap_or("on_enter"),
                                "description": enter.get("description"),
                                "_hookStart": true,
                            }));
                        }
                        _ => {}
                    }
                }
            }

            let parent_clone = tasks[parent_idx].clone();
            moved_events.push(serde_json::json!({
                "taskId": parent_id,
                "fromStage": &old_parent_stage,
                "toStage": &old_parent_stage,
                "stageId": &old_parent_stage,
                "status": "idle",
                "waitingForSubtaskId": &new_subtask_id,
                "verdict": parent_clone.verdict,
                "task": &parent_clone
            }));

            created_tasks.push(new_subtask.clone());
            tasks.push(new_subtask);

            // Parent is now waiting on the newly created subtask; break recursion
            break;
        }
    }

    ParentAdvanceResult {
        moved_events,
        created_tasks,
        autorun_task_ids,
        hook_finish_events,
        mutated_task_ids,
    }
}
