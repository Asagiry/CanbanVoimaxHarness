use super::*;
use crate::config::ToolPolicyConfig;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

    #[tokio::test]
    async fn test_subagent_session_and_receiver_lookup() {
        let (tx1, rx1) = oneshot::channel::<String>();
        let flag = Arc::new(AtomicBool::new(false));
        let workdir = std::path::PathBuf::from("target").join(format!("cvh_test_subagent_lookup_{}", uuid::Uuid::new_v4()));
        let target_root = std::path::PathBuf::from("target");
        let policy = ToolPolicyConfig::default();

        let ctx = OwnedToolContext {
            repo_root: target_root,
            worktree_dir: workdir,
            stage_id: Some("dev".to_string()),
            role: Some("developer".to_string()),
            abort_handle: flag,
            tool_policy: policy,
            task_id: Some("CVH-100".to_string()),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: Some("call_sub_123".to_string()),
        };

        let sub_id = "CVH-100.sub-call_sub_123".to_string();
        let retry_info = SubagentRetryInfo {
            parent_task_id: "CVH-100".to_string(),
            subagent_id: sub_id.clone(),
            tool_call_id: Some("call_sub_123".to_string()),
            title: "Test subagent".to_string(),
            role: "developer".to_string(),
            prompt: "Test prompt".to_string(),
            requested_tools: None,
            ctx,
            status: "running".to_string(),
            last_error: None,
            report: None,
            is_bg: false,
            duration_ms: None,
            steps: 0,
        };

        let session = SubagentSession::new(Some(tx1), retry_info);

        {
            let mut sessions = SUBAGENT_SESSIONS.lock().await;
            sessions.insert(sub_id.clone(), session);
        }

        // Test 1: find by exact subagent_id
        let res1 = find_subagent_session(Some("CVH-100"), "CVH-100.sub-call_sub_123").await;
        assert!(res1.is_some());
        assert_eq!(res1.as_ref().map(|s| s.0.as_str()), Some(sub_id.as_str()));

        // Test 2: find by stripped :sub: dialog ID
        let res2 = find_subagent_session(Some("CVH-100"), "CVH-100:sub:CVH-100.sub-call_sub_123").await;
        assert!(res2.is_some());
        assert_eq!(res2.as_ref().map(|s| s.0.as_str()), Some(sub_id.as_str()));

        // Test 3: find by tool_call_id alone
        let res3 = find_subagent_session(Some("CVH-100"), "call_sub_123").await;
        assert!(res3.is_some());
        assert_eq!(res3.as_ref().map(|s| s.0.as_str()), Some(sub_id.as_str()));

        // Test 4: find by .sub-call_sub_123
        let res4 = find_subagent_session(None, ".sub-call_sub_123").await;
        assert!(res4.is_some());
        assert_eq!(res4.as_ref().map(|s| s.0.as_str()), Some(sub_id.as_str()));

        // Test remove_subagent_receiver
        let mut rec_map = HashMap::new();
        rec_map.insert(sub_id.clone(), rx1);

        let retrieved = remove_subagent_receiver(&mut rec_map, "call_sub_123");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.as_ref().map(|s| s.0.as_str()), Some(sub_id.as_str()));
        assert!(rec_map.is_empty());
    }


    #[tokio::test]
    async fn test_system_prompt_subagent_delegation_mandate() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_sub_mandate_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh").join("agents"))
            .await
            .expect("test dirs");
        tokio::fs::write(
            base.join(".cvh").join("agents").join("developer.md"),
            "PERSONA_SUB_TEST",
        )
        .await
        .expect("persona seed");

        let prompt = compile_primed_system_prompt(&base, "developer")
            .await
            .expect("prompt failed");

        assert!(prompt.contains("## Subagent Delegation Mandate"));
        assert!(prompt.contains("prefer `subagent` (oneshot sync)"));
        assert!(prompt.contains("MUST call `collect_subagent`"));
        assert!(prompt.contains("Single-Action Collection"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_subagent_tool_filtering_and_forbidden_operations() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_sub_filter_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        // Allowed subagent tools strictly exclude task card manipulation
        let forbidden = [
            "create_task",
            "create_tasks",
            "complete_task",
            "advance_task",
            "move_task",
            "move_task_stage",
            "advance_stage",
            "next_stage",
            "reject_task",
            "subagent",
            "subagent_bg",
            "collect_subagent",
        ];

        let allowed_subagent_tools = vec![
            "read_file".to_string(),
            "write_file".to_string(),
            "edit_file".to_string(),
            "glob".to_string(),
            "grep".to_string(),
            "execute_command".to_string(),
        ];

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let sub_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-070.sub-1"),
            agent_allowed_tools: Some(&allowed_subagent_tools),
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Every forbidden tool must be rejected with PermissionDenied
        for tool in &forbidden {
            let err = execute_tool(&sub_ctx, tool, &serde_json::json!({
                "title": "Hack",
                "summary": "Hack",
                "reason": "Hack"
            }))
            .await
            .expect_err(&format!("Subagent must be denied from calling '{}'", tool));

            assert!(
                err.to_string().contains("PermissionDenied"),
                "Expected PermissionDenied for tool '{}', got: {}",
                tool,
                err
            );
        }

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_subagent_bg_and_collect_single_and_batch() {
        let (tx1, rx1) = oneshot::channel::<String>();
        let (tx2, rx2) = oneshot::channel::<String>();

        let sub_id1 = "CVH-080.sub-bg-test-1".to_string();
        let sub_id2 = "CVH-080.sub-bg-test-2".to_string();

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let target_root = std::path::PathBuf::from("target");
        let workdir = std::path::PathBuf::from("target").join(format!("cvh_test_subagent_bg_{}", uuid::Uuid::new_v4()));

        let ctx = OwnedToolContext {
            repo_root: target_root.clone(),
            worktree_dir: workdir.clone(),
            stage_id: Some("dev".to_string()),
            role: Some("developer".to_string()),
            abort_handle: Arc::clone(&flag),
            tool_policy: policy.clone(),
            task_id: Some("CVH-080".to_string()),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: Some("gemini-3.8-flash-high".to_string()),
            temperature: None,
            reasoning_effort: None,
            call_id: Some("call_bg_1".to_string()),
        };

        let session1 = SubagentSession::new(
            Some(tx1),
            SubagentRetryInfo {
                parent_task_id: "CVH-080".to_string(),
                subagent_id: sub_id1.clone(),
                tool_call_id: Some("call_bg_1".to_string()),
                title: "Task 1".to_string(),
                role: "developer".to_string(),
                prompt: "Run 1".to_string(),
                requested_tools: None,
                ctx: ctx.clone(),
                status: "running".to_string(),
                last_error: None,
                report: None,
                is_bg: true,
                duration_ms: None,
                steps: 0,
            },
        );

        let session2 = SubagentSession::new(
            Some(tx2),
            SubagentRetryInfo {
                parent_task_id: "CVH-080".to_string(),
                subagent_id: sub_id2.clone(),
                tool_call_id: Some("call_bg_2".to_string()),
                title: "Task 2".to_string(),
                role: "developer".to_string(),
                prompt: "Run 2".to_string(),
                requested_tools: None,
                ctx: ctx.clone(),
                status: "running".to_string(),
                last_error: None,
                report: None,
                is_bg: true,
                duration_ms: None,
                steps: 0,
            },
        );

        // Register receivers and sessions
        {
            let mut receivers = SUBAGENT_RECEIVERS.lock().await;
            receivers.insert(sub_id1.clone(), rx1);
            receivers.insert(sub_id2.clone(), rx2);

            let mut sessions = SUBAGENT_SESSIONS.lock().await;
            sessions.insert(sub_id1.clone(), session1);
            sessions.insert(sub_id2.clone(), session2);
        }

        let tool_ctx = ToolContext {
            repo_root: &target_root,
            worktree_dir: &workdir,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-080"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Single collection
        // Send report to sub_id1
        {
            let sessions = SUBAGENT_SESSIONS.lock().await;
            if let Some(s) = sessions.get(&sub_id1) {
                if let Some(sender) = s.sender.lock().await.take() {
                    let _ = sender.send("Report from task 1".to_string());
                }
            }
        }

        let res1 = execute_tool(&tool_ctx, "collect_subagent", &serde_json::json!({
            "subagent_id": &sub_id1
        }))
        .await
        .expect("collect single subagent");
        assert_eq!(res1, "Report from task 1");

        // Collecting sub_id1 again must fail (already collected)
        let err_again = execute_tool(&tool_ctx, "collect_subagent", &serde_json::json!({
            "subagent_id": &sub_id1
        }))
        .await
        .expect_err("Already collected subagent must error");
        assert!(err_again.to_string().contains("not found in background registry"));

        // 2. Batch collection test
        let sub_id3 = "CVH-080.sub-bg-test-3".to_string();
        let sub_id4 = "CVH-080.sub-bg-test-4".to_string();
        let (tx3, rx3) = oneshot::channel::<String>();
        let (tx4, rx4) = oneshot::channel::<String>();

        {
            let mut receivers = SUBAGENT_RECEIVERS.lock().await;
            receivers.insert(sub_id3.clone(), rx3);
            receivers.insert(sub_id4.clone(), rx4);
        }

        // Send reports
        let _ = tx3.send("Report 3 completed".to_string());
        let _ = tx4.send("Report 4 completed".to_string());

        let res_batch = execute_tool(&tool_ctx, "collect_subagent", &serde_json::json!({
            "subagent_ids": [&sub_id3, &sub_id4]
        }))
        .await
        .expect("collect batch subagents");

        assert!(res_batch.contains("Report 3 completed"));
        assert!(res_batch.contains("Report 4 completed"));
        assert!(res_batch.contains("---"));
    }


    #[tokio::test]
    async fn test_collect_subagent_resilient_on_channel_closed() {
        let (tx, rx) = oneshot::channel::<String>();
        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let target_root = std::path::PathBuf::from("target");
        let workdir = std::path::PathBuf::from("target").join(format!("cvh_test_subagent_resilient_{}", uuid::Uuid::new_v4()));

        let ctx = OwnedToolContext {
            repo_root: target_root.clone(),
            worktree_dir: workdir.clone(),
            stage_id: Some("dev".to_string()),
            role: Some("developer".to_string()),
            abort_handle: Arc::clone(&flag),
            tool_policy: policy.clone(),
            task_id: Some("CVH-085".to_string()),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: Some("gemini-3.8-flash-high".to_string()),
            temperature: None,
            reasoning_effort: None,
            call_id: Some("call_resilient_1".to_string()),
        };

        let sub_id = "CVH-085.sub-resilient-1".to_string();
        let session = SubagentSession::new(
            Some(tx),
            SubagentRetryInfo {
                parent_task_id: "CVH-085".to_string(),
                subagent_id: sub_id.clone(),
                tool_call_id: Some("call_resilient_1".to_string()),
                title: "Resilient Task".to_string(),
                role: "developer".to_string(),
                prompt: "Run resilient".to_string(),
                requested_tools: None,
                ctx: ctx.clone(),
                status: "finished".to_string(),
                last_error: None,
                report: Some("Session saved report successfully".to_string()),
                is_bg: true,
                duration_ms: Some(120.0),
                steps: 2,
            },
        );

        {
            let mut receivers = SUBAGENT_RECEIVERS.lock().await;
            receivers.insert(sub_id.clone(), rx);
            let mut sessions = SUBAGENT_SESSIONS.lock().await;
            sessions.insert(sub_id.clone(), session);
        }

        // Drop sender to simulate channel closed (e.g. sender dropped without sending)
        {
            let sessions = SUBAGENT_SESSIONS.lock().await;
            if let Some(s) = sessions.get(&sub_id) {
                let _ = s.sender.lock().await.take();
            }
        }

        let tool_ctx = ToolContext {
            repo_root: &target_root,
            worktree_dir: &workdir,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-085"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        let res = execute_tool(&tool_ctx, "collect_subagent", &serde_json::json!({
            "subagent_id": &sub_id
        }))
        .await
        .expect("must recover report from SUBAGENT_SESSIONS even when channel closed");

        assert_eq!(res, "Session saved report successfully");
    }


    #[tokio::test]
    async fn test_collect_subagent_deterministic_order() {
        let (tx1, rx1) = oneshot::channel::<String>();
        let (tx2, rx2) = oneshot::channel::<String>();
        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let target_root = std::path::PathBuf::from("target");
        let workdir = std::path::PathBuf::from("target").join(format!("cvh_test_subagent_det_{}", uuid::Uuid::new_v4()));

        let ctx = OwnedToolContext {
            repo_root: target_root.clone(),
            worktree_dir: workdir.clone(),
            stage_id: Some("dev".to_string()),
            role: Some("developer".to_string()),
            abort_handle: Arc::clone(&flag),
            tool_policy: policy.clone(),
            task_id: Some("CVH-086".to_string()),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: Some("gemini-3.8-flash-high".to_string()),
            temperature: None,
            reasoning_effort: None,
            call_id: Some("call_det".to_string()),
        };

        let sub_id_a = "CVH-086.sub-det-A".to_string();
        let sub_id_b = "CVH-086.sub-det-B".to_string();

        let session_a = SubagentSession::new(Some(tx1), SubagentRetryInfo {
            parent_task_id: "CVH-086".to_string(),
            subagent_id: sub_id_a.clone(),
            tool_call_id: Some("call_a".to_string()),
            title: "Task A".to_string(),
            role: "developer".to_string(),
            prompt: "Prompt A".to_string(),
            requested_tools: None,
            ctx: ctx.clone(),
            status: "running".to_string(),
            last_error: None,
            report: None,
            is_bg: true,
            duration_ms: None,
            steps: 0,
        });

        let session_b = SubagentSession::new(Some(tx2), SubagentRetryInfo {
            parent_task_id: "CVH-086".to_string(),
            subagent_id: sub_id_b.clone(),
            tool_call_id: Some("call_b".to_string()),
            title: "Task B".to_string(),
            role: "reviewer".to_string(),
            prompt: "Prompt B".to_string(),
            requested_tools: None,
            ctx: ctx.clone(),
            status: "running".to_string(),
            last_error: None,
            report: None,
            is_bg: true,
            duration_ms: None,
            steps: 0,
        });

        {
            let mut receivers = SUBAGENT_RECEIVERS.lock().await;
            receivers.insert(sub_id_a.clone(), rx1);
            receivers.insert(sub_id_b.clone(), rx2);
            let mut sessions = SUBAGENT_SESSIONS.lock().await;
            sessions.insert(sub_id_a.clone(), session_a);
            sessions.insert(sub_id_b.clone(), session_b);
        }

        // Send results: sub_id_b finishes first, then sub_id_a
        {
            let sessions = SUBAGENT_SESSIONS.lock().await;
            if let Some(s) = sessions.get(&sub_id_b) {
                if let Some(snd) = s.sender.lock().await.take() {
                    let _ = snd.send("Report B finished early".to_string());
                }
            }
            if let Some(s) = sessions.get(&sub_id_a) {
                if let Some(snd) = s.sender.lock().await.take() {
                    let _ = snd.send("Report A finished later".to_string());
                }
            }
        }

        let tool_ctx = ToolContext {
            repo_root: &target_root,
            worktree_dir: &workdir,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-086"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Collect in order [sub_id_a, sub_id_b]
        let batch_res = execute_tool(&tool_ctx, "collect_subagent", &serde_json::json!({
            "subagent_ids": [&sub_id_a, &sub_id_b]
        }))
        .await
        .expect("batch collection");

        // Verification: output preserves exact input order of subagent_ids
        let pos_a = batch_res.find("Report A finished later").expect("Report A present");
        let pos_b = batch_res.find("Report B finished early").expect("Report B present");
        assert!(pos_a < pos_b, "Report A must precede Report B matching subagent_ids order");
    }

    // -------------------------------------------------------------------------
    // c) Human-in-the-Loop & Tasks: question / ask_user, todo_write
    // -------------------------------------------------------------------------


    #[test]
    fn test_subagent_prompt_deduplication() {
        // 1. Exact match
        assert!(are_subagent_prompts_duplicate(
            "Fix the bug in agent.rs",
            "Fix the bug in agent.rs"
        ));

        // 2. Whitespace-relaxed match
        assert!(are_subagent_prompts_duplicate(
            "Fix  the   bug \n in agent.rs\n",
            "Fix the bug in agent.rs"
        ));

        // 3. High similarity / prefix match
        assert!(are_subagent_prompts_duplicate(
            "Investigate and fix the compiler error in src-tauri/src/agent.rs right away",
            "Investigate and fix the compiler error in src-tauri/src/agent.rs"
        ));

        // 4. Distinct prompts must NOT match
        assert!(!are_subagent_prompts_duplicate(
            "Run cargo clippy on src-tauri",
            "Design the frontend drawer component in React"
        ));
    }


    #[tokio::test]
    async fn test_exploratory_subagents_different_prompts_not_duplicates() {
        let parent_id = "CVH-EXPLORE-101";

        let (tx1, _rx1) = oneshot::channel();
        let ctx1 = OwnedToolContext {
            repo_root: PathBuf::from("."),
            worktree_dir: PathBuf::from("."),
            stage_id: Some("dev".to_string()),
            role: Some("architect".to_string()),
            abort_handle: Arc::new(AtomicBool::new(false)),
            tool_policy: ToolPolicyConfig::default(),
            task_id: Some(parent_id.to_string()),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: Some("call_arch_1".to_string()),
        };

        let sid1 = format!("{}.sub-call_arch_1", parent_id);
        let prompt1 = "Investigate harness2.0...";
        let info1 = SubagentRetryInfo {
            parent_task_id: parent_id.to_string(),
            subagent_id: sid1.clone(),
            tool_call_id: Some("call_arch_1".to_string()),
            title: "Investigate harness2.0".to_string(),
            role: "architect".to_string(),
            prompt: prompt1.to_string(),
            requested_tools: None,
            ctx: ctx1,
            status: "running".to_string(),
            last_error: None,
            report: None,
            is_bg: true,
            duration_ms: None,
            steps: 0,
        };

        let session1 = SubagentSession::new(Some(tx1), info1);
        {
            let mut sessions = SUBAGENT_SESSIONS.lock().await;
            sessions.insert(sid1.clone(), session1);
        }

        // Subagents #2 and #3 have the SAME role ("architect"), but DIFFERENT prompts:
        // #2: "Investigate backend rules and AGENTS.md..."
        // #3: "Investigate frontend rules..."
        let prompt2 = "Investigate backend rules and AGENTS.md...";
        let prompt3 = "Investigate frontend rules...";

        assert!(!are_subagent_prompts_duplicate(prompt1, prompt2));
        assert!(!are_subagent_prompts_duplicate(prompt1, prompt3));
        assert!(!are_subagent_prompts_duplicate(prompt2, prompt3));

        // Deduplication check across sessions for subagent #2:
        let mut is_dup_2 = false;
        let mut reused_id_2 = None;
        {
            let sessions = SUBAGENT_SESSIONS.lock().await;
            for (sid, sess) in sessions.iter() {
                let info = sess.retry_info.lock().await;
                if info.parent_task_id == parent_id
                    && (info.status == "running" || info.status == "finished")
                {
                    if info.role.eq_ignore_ascii_case("architect")
                        && are_subagent_prompts_duplicate(&info.prompt, prompt2)
                    {
                        is_dup_2 = true;
                        reused_id_2 = Some(sid.clone());
                        break;
                    }
                }
            }
        }

        assert!(!is_dup_2, "Architect subagent #2 with different prompt must NOT be considered duplicate");
        assert!(reused_id_2.is_none());

        // Each parallel subagent receives its own distinct ID
        let sid2 = format!("{}.sub-call_arch_2", parent_id);
        let sid3 = format!("{}.sub-call_arch_3", parent_id);
        assert_ne!(sid1, sid2);
        assert_ne!(sid2, sid3);
        assert_ne!(sid1, sid3);

        // Also verify explorer role with distinct prompts
        assert!(!are_subagent_prompts_duplicate(
            "Explore backend database schema",
            "Explore frontend Tailwind styles"
        ));

        // Cleanup test session
        {
            let mut sessions = SUBAGENT_SESSIONS.lock().await;
            sessions.remove(&sid1);
        }
    }


    #[test]
    fn test_is_exploratory_role() {
        assert!(is_exploratory_role("explorer"));
        assert!(is_exploratory_role("architect"));
        assert!(is_exploratory_role("researcher"));
        assert!(is_exploratory_role("scout"));
        assert!(is_exploratory_role("code_explorer"));
        assert!(is_exploratory_role("system_architect"));
        assert!(is_exploratory_role("Project Explorer"));

        // Non-exploratory roles must NOT match
        assert!(!is_exploratory_role("developer"));
        assert!(!is_exploratory_role("reviewer"));
        assert!(!is_exploratory_role("tester"));
        assert!(!is_exploratory_role("cvh-dev"));
    }


    #[test]
    fn test_subagent_session_info_is_bg() {
        let (tx, _rx) = oneshot::channel();
        let ctx = OwnedToolContext {
            repo_root: PathBuf::from("."),
            worktree_dir: PathBuf::from("."),
            stage_id: Some("dev".to_string()),
            role: Some("developer".to_string()),
            abort_handle: Arc::new(AtomicBool::new(false)),
            tool_policy: ToolPolicyConfig::default(),
            task_id: Some("CVH-101".to_string()),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        let info_bg = SubagentRetryInfo {
            parent_task_id: "CVH-101".to_string(),
            subagent_id: "CVH-101.sub-1".to_string(),
            tool_call_id: Some("call-1".to_string()),
            title: "Background worker".to_string(),
            role: "developer".to_string(),
            prompt: "Do work in background".to_string(),
            requested_tools: None,
            ctx,
            status: "running".to_string(),
            last_error: None,
            report: None,
            is_bg: true,
            duration_ms: None,
            steps: 0,
        };

        let session = SubagentSession::new(Some(tx), info_bg);
        assert!(!session.abort_handle.load(Ordering::SeqCst));

        // Test serialization of SubagentSummary with is_bg
        let summary = crate::commands::dialogs::SubagentSummary {
            id: "CVH-101.sub-1".to_string(),
            parent_task_id: "CVH-101".to_string(),
            tool_call_id: Some("call-1".to_string()),
            title: "Background worker".to_string(),
            role: "developer".to_string(),
            prompt: "Do work in background".to_string(),
            status: "running".to_string(),
            created_at: 1000,
            duration_ms: None,
            steps_count: 2,
            last_error: None,
            is_bg: true,
        };

        let serialized = serde_json::to_string(&summary).expect("serialize summary");
        assert!(serialized.contains(r#""isBg":true"#), "got: {}", serialized);

        let deserialized: crate::commands::dialogs::SubagentSummary =
            serde_json::from_str(&serialized).expect("deserialize summary");
        assert!(deserialized.is_bg);
    }


    #[tokio::test]
    async fn test_subagent_list() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_sublist_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-200"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        let sub_id1 = "CVH-200.sub-call_11111".to_string();
        let sub_id2 = "CVH-200.sub-call_22222".to_string();
        let (tx1, _rx1) = oneshot::channel();
        let (tx2, _rx2) = oneshot::channel();

        let session1 = SubagentSession::new(
            Some(tx1),
            SubagentRetryInfo {
                parent_task_id: "CVH-200".to_string(),
                subagent_id: sub_id1.clone(),
                tool_call_id: Some("call_11111".to_string()),
                title: "Backend Architecture Analysis".to_string(),
                role: "developer".to_string(),
                prompt: "Analyze backend".to_string(),
                requested_tools: None,
                ctx: tool_ctx.to_owned(),
                status: "finished".to_string(),
                last_error: None,
                report: Some("All good".to_string()),
                is_bg: true,
                duration_ms: Some(77000.0),
                steps: 5,
            },
        );

        let session2 = SubagentSession::new(
            Some(tx2),
            SubagentRetryInfo {
                parent_task_id: "CVH-200".to_string(),
                subagent_id: sub_id2.clone(),
                tool_call_id: Some("call_22222".to_string()),
                title: "Frontend Styling".to_string(),
                role: "artisan".to_string(),
                prompt: "Style UI".to_string(),
                requested_tools: None,
                ctx: tool_ctx.to_owned(),
                status: "running".to_string(),
                last_error: None,
                report: None,
                is_bg: true,
                duration_ms: None,
                steps: 2,
            },
        );

        {
            let mut sessions = SUBAGENT_SESSIONS.lock().await;
            sessions.insert(sub_id1.clone(), session1);
            sessions.insert(sub_id2.clone(), session2);
        }

        // Test 1: list all
        let all_res = execute_tool(&tool_ctx, "subagent_list", &serde_json::json!({}))
            .await
            .expect("subagent_list all");
        let parsed_all: Vec<serde_json::Value> = serde_json::from_str(&all_res).expect("valid json");
        assert!(parsed_all.iter().any(|s| s["subagent_id"] == sub_id1 && s["status"] == "completed" && s["duration_ms"] == 77000 && s["steps"] == 5));
        assert!(parsed_all.iter().any(|s| s["subagent_id"] == sub_id2 && s["status"] == "running" && s["steps"] == 2));

        // Test 2: filter running
        let running_res = execute_tool(&tool_ctx, "subagent_list", &serde_json::json!({ "status_filter": "running" }))
            .await
            .expect("subagent_list running");
        let parsed_running: Vec<serde_json::Value> = serde_json::from_str(&running_res).expect("valid json");
        assert_eq!(parsed_running.len(), 1);
        assert_eq!(parsed_running[0]["subagent_id"], sub_id2);
        assert_eq!(parsed_running[0]["status"], "running");

        // Test 3: filter completed
        let completed_res = execute_tool(&tool_ctx, "subagent_list", &serde_json::json!({ "status_filter": "completed" }))
            .await
            .expect("subagent_list completed");
        let parsed_completed: Vec<serde_json::Value> = serde_json::from_str(&completed_res).expect("valid json");
        assert_eq!(parsed_completed.len(), 1);
        assert_eq!(parsed_completed[0]["subagent_id"], sub_id1);
        assert_eq!(parsed_completed[0]["status"], "completed");

        // Cleanup in-memory sessions
        {
            let mut sessions = SUBAGENT_SESSIONS.lock().await;
            sessions.remove(&sub_id1);
            sessions.remove(&sub_id2);
        }

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


