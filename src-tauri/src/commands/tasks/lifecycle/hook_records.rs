use crate::agent::Task;

/// Normalizes legacy hook moments to the canonical frontend contract:
/// source-column exit hooks (`on_finish` / `on_exit`) are surfaced as
/// `on_leave` so `CardHookNode`/`TaskHookStates` (which only know
/// `on_enter` | `on_leave`) can render the running/success state.
pub fn normalize_hook_type(hook_type: &str) -> &'static str {
    match hook_type {
        "on_finish" | "on_exit" => "on_leave",
        "on_enter" => "on_enter",
        _ => "on_leave",
    }
}

#[allow(clippy::too_many_arguments)]
pub fn record_task_hook_result(
    task: &mut Task,
    stage_id: &str,
    hook_type: &str,
    hook_name: &str,
    started_at: i64,
    duration_ms: u64,
    success: bool,
    exit_code: Option<i32>,
    stdout: &str,
    stderr: &str,
    error: Option<&str>,
) {
    let hook_type = normalize_hook_type(hook_type);
    let mut map: serde_json::Map<String, serde_json::Value> = task
        .hook_states
        .as_ref()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    map.insert(
        hook_type.to_string(),
        serde_json::json!({
            "hookName": hook_name,
            "stageId": stage_id,
            "hookType": hook_type,
            "status": if success { "success" } else { "failed" },
            "startedAt": started_at,
            "durationMs": duration_ms,
            "exitCode": exit_code,
            "stdout": stdout,
            "stderr": stderr,
            "error": error,
        }),
    );

    task.hook_states = Some(serde_json::Value::Object(map));
}
