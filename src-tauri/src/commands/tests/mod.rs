use super::super::*;
use crate::agent::Task;
use crate::config::PipelineColumn;
use std::collections::HashMap;

pub(crate) fn test_col(id: &str) -> PipelineColumn {
    PipelineColumn {
        id: id.to_string(),
        title: id.to_string(),
        color: None,
        icon: None,
        persona: None,
        on_enter_script: None,
        context_window: None,
        model_override: None,
        reasoning_effort: None,
        temperature: None,
        gatekeeper_sensor: None,
        allowed_tools: None,
        hooks: HashMap::new(),
        routes: None,
        auto_run: None,
        terminal: None,
        is_intake: None,
    }
}

pub(crate) fn ids(cols: &[PipelineColumn]) -> Vec<String> {
    cols.iter().map(|c| c.id.clone()).collect()
}

#[allow(dead_code)]
pub(crate) fn hook_col(id: &str, on_enter: Option<&str>) -> PipelineColumn {
    let mut col = test_col(id);
    if let Some(v) = on_enter {
        col.hooks.insert("on_enter".to_string(), v.into());
    }
    col
}

pub(crate) fn task_with_wt(id: &str, wt: Option<&str>) -> Task {
    let mut t: Task = serde_json::from_value(serde_json::json!({
        "id": id, "title": "t", "stageId": "backlog"
    }))
    .expect("deserialize task");
    t.worktree_path = wt.map(|s| s.to_string());
    t
}

mod boards_tests;
mod cortex_tests;
mod dialogs_tests;
mod git_tests;
mod tasks_id_tests;
mod tasks_lifecycle_tests;
mod tasks_subtasks_tests;
mod tasks_runner_tests;
