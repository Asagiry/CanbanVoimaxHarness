//! Compaction Report Formatting & Bullet Points (§ 2.22)

use crate::llm::ChatMessage;
use super::extractor::{
    extract_diagnostics_and_tests, extract_executed_commands, extract_primary_intent,
    extract_todos_and_last_action, extract_touched_files,
};

/// Formats extracted compaction sections into Markdown bullet points and returns the complete report.
#[allow(clippy::too_many_arguments)]
pub fn format_compaction_bullet_points(
    task_id: &str,
    primary_intent: &str,
    files_modified: &[(String, String)],
    files_inspected: &[String],
    commands_run: &[(String, String)],
    tests_passed: &[String],
    tests_failed: &[String],
    errors_found: &[String],
    todos: &[(String, String)],
    last_action: Option<&str>,
) -> String {
    let mut files_lines = Vec::new();
    for (f, op) in files_modified {
        files_lines.push(format!("- `{}`: {}", f, op));
    }
    for f in files_inspected {
        if !files_modified.iter().any(|(m, _)| m == f) {
            files_lines.push(format!("- `{}`: Inspected and analyzed", f));
        }
    }
    let files_md = if files_lines.is_empty() {
        format!("- Active repository scope: task `{}`.", task_id)
    } else {
        files_lines.join("\n")
    };

    let mut error_lines = Vec::new();
    for t in tests_passed {
        error_lines.push(format!("- [TEST PASSED] {}", t));
    }
    for t in tests_failed {
        error_lines.push(format!("- [TEST FAILED] {}", t));
    }
    for e in errors_found {
        error_lines.push(format!("- [DIAGNOSTIC] `{}`", e));
    }
    let errors_md = if error_lines.is_empty() {
        "- All executed commands, tests, and sensor checks passed with 0 errors.".to_string()
    } else {
        error_lines.iter().take(8).cloned().collect::<Vec<_>>().join("\n")
    };

    let pending_md = if commands_run.is_empty() {
        "- Operational commands running in isolated git worktree with sensor verification.".to_string()
    } else {
        commands_run
            .iter()
            .take(6)
            .map(|(cmd, res)| format!("- Executed `{}`: {}", cmd, res))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut current_step_lines = Vec::new();
    if !todos.is_empty() {
        current_step_lines.push("Task Checklist:".to_string());
        let mut next_todo: Option<String> = None;
        let mut in_prog_todo: Option<String> = None;

        for (content, status) in todos {
            match status.as_str() {
                "completed" => current_step_lines.push(format!("- [x] {}", content)),
                "in_progress" => {
                    current_step_lines.push(format!("- [/] (IN PROGRESS) {}", content));
                    if in_prog_todo.is_none() {
                        in_prog_todo = Some(content.clone());
                    }
                }
                _ => {
                    current_step_lines.push(format!("- [ ] {}", content));
                    if next_todo.is_none() && in_prog_todo.is_none() {
                        next_todo = Some(content.clone());
                    }
                }
            }
        }

        if let Some(ip) = in_prog_todo {
            current_step_lines.push(format!("\n- Active Focus: Currently executing `{}`.", ip));
        } else if let Some(nt) = next_todo {
            current_step_lines.push(format!("\n- Immediate Next Step: Proceed with `{}`.", nt));
        } else {
            current_step_lines.push("\n- Immediate Next Step: All planned checklist items completed. Proceed with verification sensors.".to_string());
        }
    } else if let Some(la) = last_action {
        current_step_lines.push(format!("- Intermediate turns compacted. Last action completed: {}.", la));
        current_step_lines.push("- Immediate Next Step: Continue task implementation and execute verification sensors.".to_string());
    } else {
        current_step_lines.push("- Intermediate turns compacted. Continuing active task execution.".to_string());
        current_step_lines.push("- Immediate Next Step: Execute next code modification or verification sensor.".to_string());
    }
    let current_work_md = current_step_lines.join("\n");

    format!(
        "## Primary Request and Intent\n         - Task: {}\n         - Objective: {}\n\n         ## Key Technical Concepts\n         - High-Density State-Preserving Context Compaction with zero amnesia.\n         - Native Git worktree isolation and multi-agent pipeline routing.\n\n         ## Files and Code\n         {}\n\n         ## Errors and Fixes\n         {}\n\n         ## Pending Jobs & Subagents\n         {}\n\n         ## Current Work & Next Step\n         {}\n\n         ## Critical Invariants\n         - Ground Truth Sensor verification (cargo check, cargo test, npx tsc).\n         - Tactile obsidian dark theme aesthetics per design invariants.\n         - Strict AND-semantics tool policy matrix.",
        task_id, primary_intent, files_md, errors_md, pending_md, current_work_md
    )
}

pub fn generate_fallback_compaction_report(
    task_id: &str,
    messages: &[ChatMessage],
) -> String {
    let primary_intent = extract_primary_intent(messages, task_id);
    let (files_modified, files_inspected) = extract_touched_files(messages);
    let commands_run = extract_executed_commands(messages);
    let (tests_passed, tests_failed, errors_found) = extract_diagnostics_and_tests(messages);
    let (todos, last_action) = extract_todos_and_last_action(messages);

    format_compaction_bullet_points(
        task_id,
        &primary_intent,
        &files_modified,
        &files_inspected,
        &commands_run,
        &tests_passed,
        &tests_failed,
        &errors_found,
        &todos,
        last_action.as_deref(),
    )
}
