use std::collections::BTreeMap;

use crate::agent::Task;

pub fn freeze_stage_duration(task: &mut Task, now: i64) {
    let stage_id = task.stage_id.clone();
    let entered_at = task.stage_entered_at;
    freeze_stage_duration_for(task, &stage_id, entered_at, now);
}

pub fn freeze_stage_duration_for(task: &mut Task, stage_id: &str, entered_at: Option<i64>, now: i64) {
    if task.completed_at.is_some() {
        return;
    }
    task.completed_at = Some(now);
    let entered = entered_at.unwrap_or(task.created_at);
    let delta = ((now - entered) / 1000).max(0);
    let mut map: BTreeMap<String, i64> = task
        .stage_durations
        .as_ref()
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_i64().map(|n| (k.clone(), n)))
                .collect()
        })
        .unwrap_or_default();
    *map.entry(stage_id.to_string()).or_insert(0) += delta;
    let total: i64 = map.values().sum();
    task.stage_durations = Some(serde_json::json!(map));
    task.total_elapsed = Some(total);
}
