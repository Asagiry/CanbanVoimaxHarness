pub const STAGE_KEYS: &[&str] = &["stage_id", "stageId", "column_id", "columnId"];
pub const TASK_ID_KEYS: &[&str] = &["task_id", "taskId"];
pub const BOARD_KEYS: &[&str] = &["board_id", "boardId"];
pub const PARENT_ID_KEYS: &[&str] = &["parent_id", "parentId"];
pub const SUBTASK_KEYS: &[&str] = &["is_subtask", "isSubtask"];

pub fn get_str_param<'a>(val: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|&k| {
        val.get(k)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
    })
}

pub fn get_bool_param(val: &serde_json::Value, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|&k| val.get(k).and_then(|v| v.as_bool()))
}
