use super::*;

#[test]
fn test_registration_and_active_queries() {
    let registry = AgentSessionRegistry::new();
    let abort_flag = Arc::new(AtomicBool::new(false));
    let join_slot = Arc::new(tokio::sync::Mutex::new(None));

    let summary = AgentSessionSummary {
        session_id: "sess-CVH-043-1".to_string(),
        task_id: "CVH-043".to_string(),
        stage_id: Some("dev".to_string()),
        started_at: 1000,
        status: SessionStatus::Running,
        project_path: None,
    };

    registry.register(summary, Arc::clone(&abort_flag), join_slot);

    assert!(registry.has_active_session("CVH-043"));
    assert!(registry.has_active_session("cvh-043"));
    assert!(!registry.has_active_session("CVH-999"));

    let active = registry.get_active_sessions();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].task_id, "CVH-043");
    assert_eq!(active[0].status, SessionStatus::Running);

    let retrieved = registry.get_session("sess-CVH-043-1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().session_id, "sess-CVH-043-1");
}

#[test]
fn test_status_transitions() {
    let registry = AgentSessionRegistry::new();
    let abort_flag = Arc::new(AtomicBool::new(false));
    let join_slot = Arc::new(tokio::sync::Mutex::new(None));

    let summary = AgentSessionSummary {
        session_id: "sess-CVH-100-1".to_string(),
        task_id: "CVH-100".to_string(),
        stage_id: Some("dev".to_string()),
        started_at: 2000,
        status: SessionStatus::Running,
        project_path: None,
    };

    registry.register(summary, Arc::clone(&abort_flag), join_slot);
    assert_eq!(registry.get_active_sessions().len(), 1);

    // Transition to Paused
    registry.update_status("CVH-100", SessionStatus::Paused);
    let s = registry.get_session("sess-CVH-100-1").unwrap();
    assert_eq!(s.status, SessionStatus::Paused);
    assert!(registry.has_active_session("CVH-100"));

    // Transition to Finished
    registry.update_status("sess-CVH-100-1", SessionStatus::Finished);
    let s = registry.get_session("sess-CVH-100-1").unwrap();
    assert_eq!(s.status, SessionStatus::Finished);

    // Finished sessions are excluded from active list and has_active_session
    assert!(!registry.has_active_session("CVH-100"));
    assert_eq!(registry.get_active_sessions().len(), 0);
}

#[tokio::test]
async fn test_abort_task() {
    let registry = AgentSessionRegistry::new();
    let abort_flag_parent = Arc::new(AtomicBool::new(false));
    let abort_flag_sub = Arc::new(AtomicBool::new(false));

    let join_parent = tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    });
    let join_sub = tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    });

    registry.register(
        AgentSessionSummary {
            session_id: "sess-CVH-200-main".to_string(),
            task_id: "CVH-200".to_string(),
            stage_id: Some("dev".to_string()),
            started_at: 3000,
            status: SessionStatus::Running,
            project_path: None,
        },
        Arc::clone(&abort_flag_parent),
        Arc::new(tokio::sync::Mutex::new(Some(join_parent))),
    );

    registry.register(
        AgentSessionSummary {
            session_id: "sess-CVH-200.sub-1".to_string(),
            task_id: "CVH-200.sub-1".to_string(),
            stage_id: Some("dev".to_string()),
            started_at: 3050,
            status: SessionStatus::Running,
            project_path: None,
        },
        Arc::clone(&abort_flag_sub),
        Arc::new(tokio::sync::Mutex::new(Some(join_sub))),
    );

    let aborted = registry.abort_task("CVH-200");
    assert_eq!(aborted.len(), 2);
    assert!(abort_flag_parent.load(Ordering::SeqCst));
    assert!(abort_flag_sub.load(Ordering::SeqCst));

    let s_parent = registry.get_session("sess-CVH-200-main").unwrap();
    let s_sub = registry.get_session("sess-CVH-200.sub-1").unwrap();
    assert_eq!(s_parent.status, SessionStatus::Aborting);
    assert_eq!(s_sub.status, SessionStatus::Aborting);
}

#[test]
fn test_remove_session() {
    let registry = AgentSessionRegistry::new();
    let abort_flag = Arc::new(AtomicBool::new(false));
    let join_slot = Arc::new(tokio::sync::Mutex::new(None));

    registry.register(
        AgentSessionSummary {
            session_id: "sess-CVH-300-1".to_string(),
            task_id: "CVH-300".to_string(),
            stage_id: Some("review".to_string()),
            started_at: 4000,
            status: SessionStatus::Running,
            project_path: None,
        },
        Arc::clone(&abort_flag),
        join_slot,
    );

    assert!(registry.has_active_session("CVH-300"));
    registry.remove("sess-CVH-300-1");
    assert!(!registry.has_active_session("CVH-300"));
    assert!(registry.get_session("sess-CVH-300-1").is_none());
}

#[test]
fn test_concurrent_access() {
    let registry = Arc::new(AgentSessionRegistry::new());
    let mut handles = Vec::new();

    for i in 0..10 {
        let reg = Arc::clone(&registry);
        handles.push(std::thread::spawn(move || {
            let sid = format!("sess-CONCURRENT-{}", i);
            let tid = format!("CONCURRENT-{}", i);
            let flag = Arc::new(AtomicBool::new(false));
            let slot = Arc::new(tokio::sync::Mutex::new(None));
            reg.register(
                AgentSessionSummary {
                    session_id: sid.clone(),
                    task_id: tid.clone(),
                    stage_id: Some("dev".to_string()),
                    started_at: 1000 + i,
                    status: SessionStatus::Running,
                    project_path: None,
                },
                flag,
                slot,
            );
            assert!(reg.has_active_session(&tid));
            reg.update_status(&sid, SessionStatus::Finished);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(registry.get_active_sessions().len(), 0);
}
