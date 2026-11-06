use super::super::lifecycle::{normalize_hook_type, record_task_hook_result};
use super::task_with_wt;

#[test]
fn test_normalize_hook_type_maps_finish_and_exit_to_leave() {
    assert_eq!(normalize_hook_type("on_finish"), "on_leave");
    assert_eq!(normalize_hook_type("on_exit"), "on_leave");
    assert_eq!(normalize_hook_type("on_leave"), "on_leave");
    assert_eq!(normalize_hook_type("on_enter"), "on_enter");
}

#[test]
fn test_record_hook_result_normalizes_on_finish_key() {
    let mut task = task_with_wt("CVH-HOOK-1", None);
    record_task_hook_result(
        &mut task,
        "dev",
        "on_finish",
        "cleanup.bat",
        1000,
        50,
        true,
        Some(0),
        "ok",
        "",
        None,
    );

    let states = task.hook_states.expect("hook_states recorded");
    let obj = states.as_object().expect("hook_states object");
    assert!(obj.contains_key("on_leave"), "on_finish must normalize to on_leave: {obj:?}");
    assert!(!obj.contains_key("on_finish"));
    assert_eq!(obj["on_leave"]["hookName"], "cleanup.bat");
    assert_eq!(obj["on_leave"]["hookType"], "on_leave");
    assert_eq!(obj["on_leave"]["status"], "success");
}
