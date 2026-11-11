//! Seamless automatic data migration from legacy JSON files into SQLite tables.

use crate::agent::Task;
use rusqlite::{params, Connection, TransactionBehavior};
use std::path::Path;

pub fn auto_migrate_if_needed(conn: &mut Connection, root: &Path) -> Result<(), rusqlite::Error> {
    // 1. Tasks & Todos migration
    let tasks_file = root.join(".cvh").join("tasks.json");
    let task_count: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))?;

    if tasks_file.exists() && task_count > 0 {
        if let Err(e) = std::fs::rename(&tasks_file, root.join(".cvh").join("tasks.json.migrated")) {
            eprintln!("CVH [db]: Failed to rename legacy tasks.json: {}", e);
        }
    }

    if task_count == 0 {
        let mut loaded_tasks: Option<Vec<Task>> = None;
        if tasks_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&tasks_file) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    if let Ok(parsed) = serde_json::from_str::<Vec<Task>>(trimmed) {
                        if !parsed.is_empty() {
                            loaded_tasks = Some(parsed);
                        }
                    }
                }
            }
        }
        if loaded_tasks.is_none() {
            let backup_file = crate::commands::tasks::get_project_tasks_backup_file(root);
            if backup_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&backup_file) {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        if let Ok(parsed) = serde_json::from_str::<Vec<Task>>(trimmed) {
                            if !parsed.is_empty() {
                                loaded_tasks = Some(parsed);
                            }
                        }
                    }
                }
            }
        }

        if let Some(tasks) = loaded_tasks {
            if let Ok(tx) = conn.transaction_with_behavior(TransactionBehavior::Immediate) {
                let migrate_res: Result<(), rusqlite::Error> = (|| {
                    tx.execute_batch("PRAGMA defer_foreign_keys = ON;")?;
                    for t in &tasks {
                        let custom_flow_json = t.custom_flow.as_ref().and_then(|v| serde_json::to_string(v).ok());
                        let attachments_json = t.attachments.as_ref().and_then(|v| serde_json::to_string(v).ok());
                        let tags_json = t.tags.as_ref().and_then(|v| serde_json::to_string(v).ok());
                        let hook_states_json = t.hook_states.as_ref().and_then(|v| serde_json::to_string(v).ok());
                        let skip_wt = if t.skip_worktree.unwrap_or(false) { 1i64 } else { 0i64 };
                        let cur_tokens = t.current_tokens.map(|v| v as i64);
                        let sub_idx = t.subtask_index.map(|v| v as i64);
                        let pos_val = t.position.unwrap_or(0.0);

                        tx.execute(
                            "INSERT OR REPLACE INTO tasks (
                                id, title, description, stage_id, board_id, worktree_path, branch, verdict,
                                custom_flow, status, model, parent_id, subtask_index, waiting_for_subtask_id,
                                handover_summary, cover_image, priority, current_tokens, skip_worktree,
                                created_at, updated_at, completed_at, attachments, tags, position, hook_states,
                                stage_entered_at
                            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)",
                            params![
                                t.id,
                                t.title,
                                t.description,
                                t.stage_id,
                                t.board_id.as_deref().unwrap_or("development"),
                                t.worktree_path,
                                t.branch,
                                t.verdict,
                                custom_flow_json,
                                t.status.as_deref().unwrap_or("idle"),
                                t.model,
                                t.parent_id,
                                sub_idx,
                                t.waiting_for_subtask_id,
                                t.handover_summary,
                                t.cover_image,
                                t.priority,
                                cur_tokens,
                                skip_wt,
                                t.created_at,
                                t.updated_at,
                                t.completed_at,
                                attachments_json,
                                tags_json,
                                pos_val,
                                hook_states_json,
                                t.stage_entered_at.unwrap_or(t.created_at),
                            ],
                        )?;

                        if let Some(ref todos) = t.todos {
                            for (idx, td) in todos.iter().enumerate() {
                                tx.execute(
                                    "INSERT OR REPLACE INTO task_todos (id, task_id, content, status, order_idx)
                                     VALUES (?1, ?2, ?3, ?4, ?5)",
                                    params![td.id, t.id, td.content, td.status, idx as i64],
                                )?;
                            }
                        }
                    }
                    tx.commit()?;
                    Ok(())
                })();

                match migrate_res {
                    Ok(()) => {
                        if tasks_file.exists() {
                            let _ = std::fs::rename(&tasks_file, root.join(".cvh").join("tasks.json.migrated"));
                        }
                    }
                    Err(e) => {
                        eprintln!("CVH [db]: Auto-migration failed, leaving tasks.json intact: {}", e);
                    }
                }
            }
        }
    }

    let todo_count: i64 = conn.query_row("SELECT COUNT(*) FROM task_todos", [], |r| r.get(0))?;

    if todo_count == 0 && task_count > 0 {
        let tasks_file = root.join(".cvh").join("tasks.json");
        let mut loaded_tasks: Option<Vec<Task>> = None;
        if tasks_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&tasks_file) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    if let Ok(parsed) = serde_json::from_str::<Vec<Task>>(trimmed) {
                        if !parsed.is_empty() {
                            loaded_tasks = Some(parsed);
                        }
                    }
                }
            }
        }
        if loaded_tasks.is_none() {
            let backup_file = crate::commands::tasks::get_project_tasks_backup_file(root);
            if backup_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&backup_file) {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        if let Ok(parsed) = serde_json::from_str::<Vec<Task>>(trimmed) {
                            if !parsed.is_empty() {
                                loaded_tasks = Some(parsed);
                            }
                        }
                    }
                }
            }
        }

        if let Some(tasks) = loaded_tasks {
            if let Ok(tx) = conn.transaction_with_behavior(TransactionBehavior::Immediate) {
                let todo_res: Result<(), rusqlite::Error> = (|| {
                    for t in &tasks {
                        if let Some(ref todos) = t.todos {
                            for (idx, td) in todos.iter().enumerate() {
                                tx.execute(
                                    "INSERT OR REPLACE INTO task_todos (id, task_id, content, status, order_idx)
                                     VALUES (?1, ?2, ?3, ?4, ?5)",
                                    params![td.id, t.id, td.content, td.status, idx as i64],
                                )?;
                            }
                        }
                    }
                    tx.commit()?;
                    Ok(())
                })();
                if let Err(e) = todo_res {
                    eprintln!("CVH [db]: Todo auto-migration failed: {}", e);
                }
            }
        }
    }

    // 2. Sequence migration
    let seq_count: i64 = conn.query_row("SELECT COUNT(*) FROM sequences WHERE name = 'task'", [], |r| r.get(0))?;

    if seq_count == 0 {
        let seq_paths = [
            root.join(".cvh").join("settings").join("sequence.json"),
            root.join(".cvh").join("task-sequence.json"),
        ];
        let mut last_num: u32 = 0;
        for p in &seq_paths {
            if p.exists() {
                if let Ok(c) = std::fs::read_to_string(p) {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                            if let Some(n) = val.get("last_task_num").and_then(|v| v.as_u64()) {
                                if n > 0 {
                                    last_num = n as u32;
                                    break;
                                }
                            }
                            if let Some(n) = val.get("sequence").and_then(|v| v.as_u64()) {
                                if n > 0 {
                                    last_num = n as u32;
                                    break;
                                }
                            }
                        }
                        if let Ok(n) = trimmed.parse::<u32>() {
                            if n > 0 {
                                last_num = n;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Also inspect existing task numbers in tasks table
        if let Ok(mut stmt) = conn.prepare("SELECT id FROM tasks") {
            if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                for id_res in rows.flatten() {
                    let base = id_res.split('.').next().unwrap_or(&id_res);
                    let digits: String = base.chars().filter(|c| c.is_ascii_digit()).collect();
                    if let Ok(n) = digits.parse::<u32>() {
                        if n > last_num {
                            last_num = n;
                        }
                    }
                }
            }
        }

        if last_num > 0 {
            conn.execute(
                "INSERT OR REPLACE INTO sequences (name, current_val) VALUES ('task', ?1)",
                params![last_num as i64],
            )?;
        }
    }

    // 3. Ledger migration
    let ledger_count: i64 = conn.query_row("SELECT COUNT(*) FROM task_ledger", [], |r| r.get(0))?;

    if ledger_count == 0 {
        let ledger_paths = [
            root.join(".cvh").join("tasks-history.json"),
            root.join(".agents").join("tasks-history.json"),
        ];
        for p in &ledger_paths {
            if p.exists() {
                if let Ok(c) = std::fs::read_to_string(p) {
                    let trimmed = c.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        if let Some(obj) = val.as_object() {
                            for (key, item) in obj {
                                let task_id = item.get("taskId").and_then(|v| v.as_str()).unwrap_or(key).to_string();
                                let commit_hash = item.get("commitHash").and_then(|v| v.as_str()).map(|s| s.to_string());
                                let summary = item.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let verdict = item.get("verdict").and_then(|v| v.as_str()).map(|s| s.to_string());
                                let author = item.get("author").and_then(|v| v.as_str()).map(|s| s.to_string());
                                let files_json = item.get("files").map(|v| v.to_string());
                                let created_at = item.get("timestamp").and_then(|v| v.as_i64()).unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
                                let id = format!("ledger-{}", key);
                                conn.execute(
                                    "INSERT OR IGNORE INTO task_ledger (id, task_id, commit_hash, summary, verdict, author, files_json, created_at)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                                    params![id, task_id, commit_hash, summary, verdict, author, files_json, created_at],
                                )?;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
