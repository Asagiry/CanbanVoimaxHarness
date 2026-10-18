    use crate::config::{BoardDef, BoardsConfig, PipelineColumn, ToolPolicyConfig};
use super::*;

    #[tokio::test]
    async fn test_system_prompt_is_lean_cvh_only() {
        // Regression: only `.cvh` persona + tool instructions reach the model.
        // Root `AGENTS.md` and `.agents/skills` must NOT be injected even when
        // present on disk (user mandate: send the model ONLY .cvh + persona).
        let base =
            std::path::PathBuf::from("target").join(format!("cvh_prompt_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh").join("agents").join("developer"))
            .await
            .expect("test dirs");
        tokio::fs::write(
            base.join(".cvh").join("agents").join("developer").join("persona.md"),
            "PERSONA_MARKER_ALPHA",
        )
        .await
        .expect("persona seed");
        tokio::fs::write(base.join("AGENTS.md"), "AGENTS_MARKER_BETA")
            .await
            .expect("agents seed");
        let skill_dir = base.join(".agents").join("skills").join("demo");
        tokio::fs::create_dir_all(&skill_dir)
            .await
            .expect("skill dirs");
        tokio::fs::write(skill_dir.join("SKILL.md"), "SKILL_MARKER_GAMMA")
            .await
            .expect("skill seed");

        let prompt = compile_primed_system_prompt(&base, "developer")
            .await
            .expect("prompt failed");

        assert!(
            prompt.contains("PERSONA_MARKER_ALPHA"),
            "persona from .cvh/agents must be present"
        );
        assert!(
            !prompt.contains("AGENTS_MARKER_BETA"),
            "root AGENTS.md must NOT leak into the system prompt"
        );
        assert!(
            !prompt.contains("SKILL_MARKER_GAMMA"),
            ".agents/skills must NOT leak into the system prompt"
        );
        assert!(
            !prompt.contains("Active Agent Skills"),
            "skills section must be gone"
        );
        assert!(
            prompt.contains("Scoped Tool Execution Mandate"),
            "tool instructions must be kept"
        );

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_system_prompt_flat_persona_rules_alphabetical() {
        // Single flat persona `.cvh/agents/<role>.md` + rules/*.md (alphabetical)
        // + tool instructions.
        let base =
            std::path::PathBuf::from("target").join(format!("cvh_rules_{}", uuid::Uuid::new_v4()));
        let agents = base.join(".cvh").join("agents");
        let role_dir = agents.join("dev");
        tokio::fs::create_dir_all(role_dir.join("rules"))
            .await
            .expect("test dirs");
        tokio::fs::write(role_dir.join("persona.md"), "FLAT_PERSONA_MARKER")
            .await
            .expect("persona seed");
        // Seed out of order: b before a — prompt must order a, b.
        tokio::fs::write(role_dir.join("rules").join("b_second.md"), "RULE_B_MARKER")
            .await
            .expect("rule b");
        tokio::fs::write(role_dir.join("rules").join("a_first.md"), "RULE_A_MARKER")
            .await
            .expect("rule a");
        tokio::fs::write(role_dir.join("rules").join("notes.txt"), "IGNORED")
            .await
            .expect("non-md seed");

        let prompt = compile_primed_system_prompt(&base, "dev")
            .await
            .expect("prompt failed");

        assert!(prompt.contains("FLAT_PERSONA_MARKER"));
        assert!(prompt.contains("RULE_A_MARKER"));
        assert!(prompt.contains("RULE_B_MARKER"));
        assert!(!prompt.contains("IGNORED"), "non-md rules must be skipped");
        let pos_a = prompt.find("RULE_A_MARKER").expect("rule a pos");
        let pos_b = prompt.find("RULE_B_MARKER").expect("rule b pos");
        assert!(pos_a < pos_b, "rules must be alphabetical: a_first before b_second");
        assert!(prompt.contains("Scoped Tool Execution Mandate"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_system_prompt_flat_persona_and_builtin_fallback() {
        // Directory-only persona path: <role>/persona.md or <role>/agent.md.
        // Flat .md files are strictly ignored.
        let base =
            std::path::PathBuf::from("target").join(format!("cvh_fb_{}", uuid::Uuid::new_v4()));
        let agents = base.join(".cvh").join("agents");
        tokio::fs::create_dir_all(&agents)
            .await
            .expect("test dirs");

        // 1. persona.md in role directory
        let custom_dir = agents.join("custom");
        tokio::fs::create_dir_all(&custom_dir).await.expect("custom dir");
        tokio::fs::write(custom_dir.join("persona.md"), "PERSONAMARKER")
            .await
            .expect("persona seed");
        let with_persona = compile_primed_system_prompt(&base, "custom")
            .await
            .expect("prompt failed");
        assert!(with_persona.contains("PERSONAMARKER"), "custom persona.md must be used");

        // 2. agent.md in role directory
        let alt_dir = agents.join("alt");
        tokio::fs::create_dir_all(&alt_dir).await.expect("alt dir");
        tokio::fs::write(alt_dir.join("agent.md"), "AGENTMARKER")
            .await
            .expect("agent seed");
        let with_agent = compile_primed_system_prompt(&base, "alt")
            .await
            .expect("prompt failed");
        assert!(with_agent.contains("AGENTMARKER"), "alt agent.md must be used");

        // 3. Flat dev.md must NOT be picked up (directory-only structure)
        tokio::fs::write(agents.join("dev.md"), "FLATMARKER")
            .await
            .expect("flat seed");
        let fallback_dev = compile_primed_system_prompt(&base, "dev")
            .await
            .expect("prompt failed");
        assert!(!fallback_dev.contains("FLATMARKER"), "flat dev.md must be ignored");

        // 4. developer role without persona on disk → rich built-in developer fallback
        let dev_builtin = compile_primed_system_prompt(&base, "developer")
            .await
            .expect("prompt failed");
        assert!(
            dev_builtin.contains("developer (Software Engineer)") && dev_builtin.contains("Ground Truth Sensors"),
            "rich developer fallback must fire, got: {}",
            &dev_builtin[..dev_builtin.len().min(200)]
        );

        // 5. ghost role without persona on disk → generic expert AI agent
        let builtin = compile_primed_system_prompt(&base, "ghost")
            .await
            .expect("prompt failed");
        assert!(
            builtin.contains("You are an expert AI agent in ghost."),
            "built-in fallback must fire, got: {}",
            &builtin[..builtin.len().min(200)]
        );

        // cvh-dev without file on disk → rich builtin prompt
        let cvh_dev = compile_primed_system_prompt(&base, "cvh-dev")
            .await
            .expect("prompt failed");
        assert!(cvh_dev.contains("cvh-dev"));
        assert!(cvh_dev.contains("Columns = Agents"));
        assert!(cvh_dev.contains("boards.json"));
        assert!(cvh_dev.contains("pipeline.json"));
        assert!(cvh_dev.contains("APPROVED_WITH_CAVEATS"));
        assert!(cvh_dev.contains("Instructions for Empty & New Projects"));
        // Lean-router invariants (≤150 lines, frontmatter, paths, skeletons, guards).
        assert!(
            BUILTIN_CVH_DEV_PROMPT.lines().count() <= 150,
            "router must stay ≤150 lines, got {}",
            BUILTIN_CVH_DEV_PROMPT.lines().count()
        );
        for marker in [
            "name: <role>",
            "description: Use PROACTIVELY when",
            "model: inherit",
            "tools:",
            "scope:",
            "activation:",
            ".cvh/boards.json {activeBoardId",
            ".cvh/pipeline.json {id,title,columns[",
            ".cvh/settings/tool-policy.json {globalEnabled, stageOverrides}",
            ".cvh/agents/<role>.md",
            ".cvh/tools/<tool>.json",
            ".cvh/hooks/",
            "worktrees/<slug>",
            "activeBoardId",
            "contextWindow",
            "gatekeeperSensor",
            "terminal",
            "globalEnabled && stageOverrides",
            "commands.rs:47",
            "resolve_safe_path",
            "Verification[evidence]",
            "Red Flags",
            "30 lines",
        ] {
            assert!(
                cvh_dev.contains(marker),
                "router prompt must contain marker: {}",
                marker
            );
        }

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_agent_tool_intersection_with_policy() {
        // Effective tool set = policy lookup ∩ agent allowed_tools.
        use crate::config::{is_tool_allowed, ToolPolicyConfig};
        let policy = ToolPolicyConfig::default();

        // Agent restricts to read-only: schema filter must drop the rest
        // even though the matrix allows them.
        let agent_allowed = Some(vec!["read_file".to_string(), "glob".to_string()]);
        let all = get_all_tools(&std::path::PathBuf::from("target")).await;
        assert!(!all.is_empty());
        let active: Vec<String> = all
            .iter()
            .filter(|t| {
                is_tool_allowed(&policy, Some("dev"), &t.function.name)
                    && agent_allows_tool(agent_allowed.as_deref(), &t.function.name)
            })
            .map(|t| t.function.name.clone())
            .collect();
        assert!(active.contains(&"read_file".to_string()));
        assert!(active.contains(&"glob".to_string()));
        assert!(!active.contains(&"execute_command".to_string()));
        assert!(!active.contains(&"write_file".to_string()));

        // No agent restriction → matrix alone decides.
        let open: Vec<String> = all
            .iter()
            .filter(|t| {
                is_tool_allowed(&policy, Some("dev"), &t.function.name)
                    && agent_allows_tool(None, &t.function.name)
            })
            .map(|t| t.function.name.clone())
            .collect();
        assert!(open.len() > active.len());

        // Zero-trust gate: agent-denied tool errors even when policy allows.
        let flag = Arc::new(AtomicBool::new(false));
        let workdir = std::env::temp_dir();
        let target_root = std::path::PathBuf::from("target");
        let tool_ctx = ToolContext {
            repo_root: &target_root,
            worktree_dir: &workdir,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: None,
            agent_allowed_tools: agent_allowed.as_deref(),
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };
        let err = execute_tool(
            &tool_ctx,
            "write_file",
            &serde_json::json!({"file_path": "x.txt", "content": "y"}),
        )
        .await
        .expect_err("agent-denied tool must be rejected");
        assert!(err.to_string().contains("PermissionDenied"), "unexpected: {}", err);
    }
    #[tokio::test]
    async fn test_subagent_session_and_receiver_lookup() {
        let (tx1, rx1) = oneshot::channel::<String>();
        let flag = Arc::new(AtomicBool::new(false));
        let workdir = std::env::temp_dir();
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
        };

        let session = SubagentSession {
            sender: Arc::new(tokio::sync::Mutex::new(Some(tx1))),
            retry_info: Arc::new(tokio::sync::Mutex::new(retry_info)),
        };

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

        let _ = tokio::fs::remove_dir_all(&base).await;
    }
    #[tokio::test]
    async fn test_orchestrator_create_task_and_create_tasks() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_orch_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        // Seed parent orchestrator task CVH-001
        let parent_task = Task {
            id: "CVH-001".to_string(),
            title: "Parent Orchestrator Task".to_string(),
            description: "Decomposing feature".to_string(),
            stage_id: "backlog".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: Some("high".to_string()),
            tags: Some(vec!["orchestrator".to_string()]),
            current_tokens: None,
            created_at: 0,
            updated_at: 0,
        };
        crate::commands::tasks::save_tasks_to_disk(&base, &[parent_task]).await.expect("save parent");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("backlog"),
            role: Some("orchestrator"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-001"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Single create_task call
        let single_args = serde_json::json!({
            "title": "Backend Rust Implementation",
            "description": "Implement create_task and complete_task tools",
            "column_id": "dev",
            "priority": "high",
            "tags": ["rust", "tauri"]
        });
        let single_res = execute_tool(&tool_ctx, "create_task", &single_args)
            .await
            .expect("create_task execution");

        let parsed_single: serde_json::Value = serde_json::from_str(&single_res).expect("parse single json");
        assert_eq!(parsed_single["status"], "created");
        assert_eq!(parsed_single["task_id"], "CVH-002");
        assert_eq!(parsed_single["title"], "Backend Rust Implementation");
        assert_eq!(parsed_single["column_id"], "dev");

        // Verify on disk that CVH-002 has parent_id = "CVH-001"
        let tasks_on_disk = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let created_child = tasks_on_disk.iter().find(|t| t.id == "CVH-002").expect("CVH-002 on disk");
        assert_eq!(created_child.parent_id.as_deref(), Some("CVH-001"));
        assert_eq!(created_child.priority.as_deref(), Some("high"));
        assert_eq!(created_child.tags.as_ref().unwrap(), &vec!["rust".to_string(), "tauri".to_string()]);

        // 2. Batch create_tasks call
        let batch_args = serde_json::json!({
            "tasks": [
                {
                    "title": "Frontend UI Matrix",
                    "description": "Integrate into toolPolicyStore.ts",
                    "column_id": "dev",
                    "priority": "medium"
                },
                {
                    "title": "End-to-End Test Suite",
                    "description": "Run cargo check and vitest",
                    "column_id": "dev",
                    "priority": "high"
                }
            ]
        });
        let batch_res = execute_tool(&tool_ctx, "create_tasks", &batch_args)
            .await
            .expect("create_tasks execution");

        let parsed_batch: serde_json::Value = serde_json::from_str(&batch_res).expect("parse batch json");
        assert_eq!(parsed_batch["status"], "created");
        assert_eq!(parsed_batch["count"], 2);

        let tasks_after_batch = crate::commands::tasks::load_tasks_from_disk(&base).await;
        assert_eq!(tasks_after_batch.len(), 4); // CVH-001, CVH-002, CVH-003, CVH-004

        let t3 = tasks_after_batch.iter().find(|t| t.id == "CVH-003").expect("CVH-003");
        let t4 = tasks_after_batch.iter().find(|t| t.id == "CVH-004").expect("CVH-004");
        assert_eq!(t3.parent_id.as_deref(), Some("CVH-001"));
        assert_eq!(t4.parent_id.as_deref(), Some("CVH-001"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_complete_task_anti_bypass_invariant() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_antibypass_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        // Seed a regular developer task CVH-010 that has NO child tasks
        let regular_task = Task {
            id: "CVH-010".to_string(),
            title: "Regular Developer Task".to_string(),
            description: "Working on code".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            created_at: 0,
            updated_at: 0,
        };
        crate::commands::tasks::save_tasks_to_disk(&base, &[regular_task]).await.expect("save regular task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let dev_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-010"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Regular dev agent attempting to bypass pipeline with complete_task MUST BE REJECTED
        let bypass_err = execute_tool(&dev_ctx, "complete_task", &serde_json::json!({
            "summary": "Trying to skip review and retro"
        }))
        .await
        .expect_err("Regular dev agent must be rejected from complete_task");

        assert!(
            bypass_err.to_string().contains("complete_task is restricted to orchestrator tasks that created subtasks."),
            "Error must match Anti-Bypass Invariant string, got: {}",
            bypass_err
        );

        // Verify task is still in 'dev' with status 'running'
        let tasks_check1 = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t10 = tasks_check1.iter().find(|t| t.id == "CVH-010").unwrap();
        assert_eq!(t10.stage_id, "dev");
        assert_eq!(t10.status.as_deref(), Some("running"));

        // 2. Now let's spawn a child task linked to CVH-010
        let child_task = Task {
            id: "CVH-011".to_string(),
            title: "Child Subtask".to_string(),
            description: "Subtask 1".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
            model: None,
            parent_id: Some("CVH-010".to_string()),
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            created_at: 0,
            updated_at: 0,
        };
        crate::commands::tasks::save_tasks_to_disk(&base, &[t10.clone(), child_task]).await.expect("save with child");

        // Now CVH-010 has spawned child tasks -> complete_task MUST SUCCEED
        let complete_res = execute_tool(&dev_ctx, "complete_task", &serde_json::json!({
            "summary": "Decomposition finished and subtasks spawned"
        }))
        .await
        .expect("complete_task must succeed for task with child subtasks");

        let parsed_res: serde_json::Value = serde_json::from_str(&complete_res).expect("parse complete json");
        assert_eq!(parsed_res["task_id"], "CVH-010");
        assert_eq!(parsed_res["status"], "done");
        assert_eq!(parsed_res["column_id"], "ready");

        // Verify on disk that CVH-010 moved to 'ready', status is 'done', and handover_summary is recorded
        let tasks_final = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t10_done = tasks_final.iter().find(|t| t.id == "CVH-010").unwrap();
        assert_eq!(t10_done.stage_id, "ready");
        assert_eq!(t10_done.status.as_deref(), Some("done"));
        assert_eq!(t10_done.handover_summary.as_deref(), Some("Decomposition finished and subtasks spawned"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_cvh_dev_cannot_create_or_complete_tasks() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_dev_scope_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let cvh_dev_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("cvh-dev"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-999"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Calling create_task as cvh-dev must fail
        let create_err = execute_tool(&cvh_dev_ctx, "create_task", &serde_json::json!({
            "title": "Forbidden task"
        }))
        .await
        .expect_err("cvh-dev must NOT be allowed to call create_task");
        assert!(create_err.to_string().contains("cvh-dev"));

        // 2. Calling complete_task as cvh-dev must fail
        let complete_err = execute_tool(&cvh_dev_ctx, "complete_task", &serde_json::json!({
            "summary": "Trying to complete task"
        }))
        .await
        .expect_err("cvh-dev must NOT be allowed to call complete_task");
        assert!(complete_err.to_string().contains("cvh-dev"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[test]
    fn test_canonical_tool_name_aliasing() {
        assert_eq!(canonical_tool_name("move_task"), "advance_task");
        assert_eq!(canonical_tool_name("move_task_stage"), "advance_task");
        assert_eq!(canonical_tool_name("advance_stage"), "advance_task");
        assert_eq!(canonical_tool_name("next_stage"), "advance_task");
        assert_eq!(canonical_tool_name("advance_task"), "advance_task");
        assert_eq!(canonical_tool_name("bash"), "execute_command");
        assert_eq!(canonical_tool_name("str_replace"), "edit_file");
        assert_eq!(canonical_tool_name("ask_user"), "question");
        assert_eq!(canonical_tool_name("create_tasks"), "create_task");
        assert_eq!(canonical_tool_name("read_file"), "read_file");
    }

    #[tokio::test]
    async fn test_advance_and_move_task_tool_dispatch() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_adv_tool_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let b_cols = vec![
            crate::config::PipelineColumn {
                id: "backlog".to_string(),
                title: "Backlog".to_string(),
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
            },
            crate::config::PipelineColumn {
                id: "dev".to_string(),
                title: "Dev".to_string(),
                color: None,
                icon: None,
                persona: Some("developer".to_string()),
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
            },
            crate::config::PipelineColumn {
                id: "review".to_string(),
                title: "Review".to_string(),
                color: None,
                icon: None,
                persona: Some("reviewer".to_string()),
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
            },
            crate::config::PipelineColumn {
                id: "ready".to_string(),
                title: "Ready".to_string(),
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
                terminal: Some(true),
            },
        ];

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "dev-board".to_string(),
            boards: vec![
                crate::config::BoardDef {
                    id: "dev-board".to_string(),
                    name: "Development".to_string(),
                    columns: b_cols,
                },
            ],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        let task = Task {
            id: "CVH-050".to_string(),
            title: "Move Task Test".to_string(),
            description: "Testing move task aliasing and target_stage".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("dev-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            created_at: 0,
            updated_at: 0,
            handover_summary: None,
            attachments: None,
            cover_image: None,
        };
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-050"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. advance_task without summary -> uses default summary and advances to next column (review)
        let adv_res = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({}))
            .await
            .expect("advance_task with default summary");
        assert!(adv_res.contains("УСПЕШНО"));

        let tasks_after_adv = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks_after_adv.iter().find(|t| t.id == "CVH-050").unwrap();
        assert_eq!(t.stage_id, "review");

        // 2. move_task with target_stage: "ready" and explicit summary
        let move_res = execute_tool(&tool_ctx, "move_task", &serde_json::json!({
            "target_stage": "ready",
            "summary": "Completed code review, verified clean build."
        }))
        .await
        .expect("move_task to ready");
        assert!(move_res.contains("УСПЕШНО"));

        let tasks_after_move = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t_ready = tasks_after_move.iter().find(|t| t.id == "CVH-050").unwrap();
        assert_eq!(t_ready.stage_id, "ready");
        assert_eq!(t_ready.status.as_deref(), Some("done"));
        assert_eq!(t_ready.handover_summary.as_deref(), Some("Completed code review, verified clean build."));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    static QUESTION_TEST_MUTEX: std::sync::LazyLock<tokio::sync::Mutex<()>> = std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

    // =========================================================================
    // COMPREHENSIVE AUTOMATED TEST SUITE: TOOLS, SUBAGENTS & PIPELINE (CVH)
    // =========================================================================

    fn make_test_board(_base: &Path) -> (BoardsConfig, Vec<PipelineColumn>) {
        let b_cols = vec![
            PipelineColumn {
                id: "backlog".to_string(),
                title: "Backlog".to_string(),
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
            },
            PipelineColumn {
                id: "dev".to_string(),
                title: "Dev".to_string(),
                color: None,
                icon: None,
                persona: Some("developer".to_string()),
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
            },
            PipelineColumn {
                id: "review".to_string(),
                title: "Review".to_string(),
                color: None,
                icon: None,
                persona: Some("reviewer".to_string()),
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
            },
            PipelineColumn {
                id: "ready".to_string(),
                title: "Ready".to_string(),
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
                terminal: Some(true),
            },
        ];

        let boards_cfg = BoardsConfig {
            active_board_id: "test-board".to_string(),
            boards: vec![BoardDef {
                id: "test-board".to_string(),
                name: "Test Board".to_string(),
                columns: b_cols.clone(),
            }],
        };

        (boards_cfg, b_cols)
    }

    fn make_test_task(id: &str, stage_id: &str, board_id: &str) -> Task {
        Task {
            id: id.to_string(),
            title: format!("Task {}", id),
            description: format!("Description for {}", id),
            stage_id: stage_id.to_string(),
            board_id: Some(board_id.to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            created_at: 0,
            updated_at: 0,
            handover_summary: None,
            attachments: None,
            cover_image: None,
        }
    }

    // -------------------------------------------------------------------------
    // a) Pipeline Transitions: advance_task, move_task, reject_task
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_pipeline_advance_task_sequential_and_boundary_error() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_adv_seq_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in backlog
        let task = make_test_task("CVH-060", "backlog", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("backlog"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-060"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Advance from backlog -> dev
        let res1 = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({ "summary": "Started dev work" }))
            .await
            .expect("advance backlog -> dev");
        assert!(res1.contains("УСПЕШНО"), "got: {}", res1);

        let tasks1 = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t1 = tasks1.iter().find(|t| t.id == "CVH-060").expect("find task");
        assert_eq!(t1.stage_id, "dev");

        // 2. Advance from dev -> review
        let res2 = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({ "summary": "Implemented feature" }))
            .await
            .expect("advance dev -> review");
        assert!(res2.contains("УСПЕШНО"), "got: {}", res2);

        let tasks2 = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t2 = tasks2.iter().find(|t| t.id == "CVH-060").expect("find task");
        assert_eq!(t2.stage_id, "review");

        // 3. Advance from review -> ready (terminal)
        let res3 = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({ "summary": "Review approved" }))
            .await
            .expect("advance review -> ready");
        assert!(res3.contains("УСПЕШНО"), "got: {}", res3);

        let tasks3 = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t3 = tasks3.iter().find(|t| t.id == "CVH-060").expect("find task");
        assert_eq!(t3.stage_id, "ready");
        assert_eq!(t3.status.as_deref(), Some("done"));

        // 4. Boundary error: Advance when already in terminal ready stage must fail
        let res_err = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({}))
            .await
            .expect_err("advance from terminal column must fail");
        assert!(res_err.to_string().contains("уже находится на последнем этапе пайплайна"), "got: {}", res_err);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_pipeline_move_task_explicit_target_stage() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_move_tgt_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in dev
        let task = make_test_task("CVH-061", "dev", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-061"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Move directly from dev to ready via target_stage
        let res = execute_tool(&tool_ctx, "move_task", &serde_json::json!({
            "target_stage": "ready",
            "summary": "Fast-tracked verification"
        }))
        .await
        .expect("move_task with target_stage");
        assert!(res.contains("УСПЕШНО"));

        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-061").expect("find task");
        assert_eq!(t.stage_id, "ready");
        assert_eq!(t.status.as_deref(), Some("done"));
        assert_eq!(t.handover_summary.as_deref(), Some("Fast-tracked verification"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_pipeline_reject_task_moves_back_and_sets_changes_requested() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_reject_ok_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in 'review' (stage index 2: backlog=0, dev=1, review=2)
        let task = make_test_task("CVH-062", "review", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("review"),
            role: Some("reviewer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-062"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Reject task with reason
        let res = execute_tool(&tool_ctx, "reject_task", &serde_json::json!({
            "reason": "Clippy sensor emitted 3 warnings"
        }))
        .await
        .expect("reject_task from review");

        assert!(res.contains("ОТКЛОНЕНО"), "got: {}", res);
        assert!(res.contains("dev"), "got: {}", res);

        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-062").expect("find task");
        assert_eq!(t.stage_id, "dev", "Must move back to dev");
        assert_eq!(t.status.as_deref(), Some("idle"), "Status must be reset to idle");
        assert_eq!(t.verdict.as_deref(), Some("CHANGES_REQUESTED"), "Verdict must be CHANGES_REQUESTED");
        assert_eq!(t.handover_summary.as_deref(), Some("ОТКЛОНЕНО РЕВЬЮ: Clippy sensor emitted 3 warnings"));

        // Verify reject with explicit task_id in args when tool_ctx.task_id is None
        let tool_ctx_no_id = ToolContext {
            task_id: None,
            ..tool_ctx
        };
        // Move task to review again
        let mut tasks_mut = tasks.clone();
        tasks_mut[0].stage_id = "review".to_string();
        crate::commands::tasks::save_tasks_to_disk(&base, &tasks_mut).await.expect("reset to review");

        let res_explicit = execute_tool(&tool_ctx_no_id, "reject_task", &serde_json::json!({
            "task_id": "CVH-062",
            "reason": "Unit tests failing"
        }))
        .await
        .expect("reject_task with explicit task_id");
        assert!(res_explicit.contains("ОТКЛОНЕНО"));

        let tasks_explicit = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t_exp = tasks_explicit.iter().find(|t| t.id == "CVH-062").expect("find task");
        assert_eq!(t_exp.stage_id, "dev");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_pipeline_reject_task_blocks_first_working_stage_to_backlog() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_reject_inv_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in 'dev' (first working column right after backlog)
        let task = make_test_task("CVH-063", "dev", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-063"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Attempting to reject from 'dev' (first working stage) back to backlog must be BLOCKED
        let err = execute_tool(&tool_ctx, "reject_task", &serde_json::json!({
            "reason": "Cannot proceed"
        }))
        .await
        .expect_err("Rejecting from first working stage back to backlog must be rejected");

        assert!(
            err.to_string().contains("Запрещено возвращать задачу из первой рабочей стадии назад в бэклог. Задача должна быть доработана или закрыта."),
            "Expected ironclad invariant error, got: {}",
            err
        );

        // Verify task did not move and remains in 'dev'
        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-063").expect("find task");
        assert_eq!(t.stage_id, "dev");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_pipeline_reject_task_from_initial_column_fails() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_reject_init_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in 'backlog' (index 0)
        let task = make_test_task("CVH-064", "backlog", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("backlog"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-064"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        let err = execute_tool(&tool_ctx, "reject_task", &serde_json::json!({ "reason": "No previous" }))
            .await
            .expect_err("Reject from initial column must fail");
        assert!(err.to_string().contains("уже находится в начальной колонке"), "got: {}", err);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    // -------------------------------------------------------------------------
    // b) Subagents & Delegation: synchronous subagent, subagent_bg & collect
    // -------------------------------------------------------------------------

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
        let workdir = std::env::temp_dir();

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

        let session1 = SubagentSession {
            sender: Arc::new(tokio::sync::Mutex::new(Some(tx1))),
            retry_info: Arc::new(tokio::sync::Mutex::new(SubagentRetryInfo {
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
            })),
        };

        let session2 = SubagentSession {
            sender: Arc::new(tokio::sync::Mutex::new(Some(tx2))),
            retry_info: Arc::new(tokio::sync::Mutex::new(SubagentRetryInfo {
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
            })),
        };

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

    // -------------------------------------------------------------------------
    // c) Human-in-the-Loop & Tasks: question / ask_user, todo_write
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_question_tool_schema_and_pause_unblock() {
        let _lock = QUESTION_TEST_MUTEX.lock().await;
        let base = std::path::PathBuf::from("target").join(format!("cvh_q_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &policy,
            task_id: Some("CVH-090"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Spawn responder task which polls QUESTION_REGISTRY and delivers answer
        let responder = tokio::spawn(async {
            for _ in 0..60 {
                tokio::time::sleep(Duration::from_millis(50)).await;
                let mut reg = QUESTION_REGISTRY.lock().await;
                if !reg.is_empty() {
                    let q_id = reg.keys().next().cloned().unwrap();
                    if let Some(sender) = reg.remove(&q_id) {
                        let _ = sender.send("SQLite".to_string());
                        return true;
                    }
                }
            }
            false
        });

        let res = execute_tool(&tool_ctx, "question", &serde_json::json!({
            "text": "Which database engine?",
            "options": [
                { "label": "SQLite", "description": "Local embedded" },
                { "label": "PostgreSQL", "description": "Client-server" }
            ],
            "multiple": false
        }))
        .await
        .expect("question execution");

        let did_answer = responder.await.expect("join responder");
        assert!(did_answer, "Question was not registered in QUESTION_REGISTRY within timeout");
        assert_eq!(res, "SQLite");

        // Verify SQLite persisted question activity item with selectedAnswer
        let dialog_items = crate::db::load_dialog_items(&base, "CVH-090").expect("load items");
        assert!(!dialog_items.is_empty(), "Question must be persisted in SQLite");
        let q_item = dialog_items.iter().find(|i| i.item_type == "question").expect("question item");
        assert!(q_item.content.as_deref().unwrap().contains("Which database engine?"));
        let tool_meta = q_item.tool.as_ref().expect("tool meta");
        assert_eq!(tool_meta["status"], "success");
        assert_eq!(tool_meta["selectedAnswer"], "SQLite");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_question_tool_abort_flag_cleanup() {
        let _lock = QUESTION_TEST_MUTEX.lock().await;
        let base = std::path::PathBuf::from("target").join(format!("cvh_q_abort_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&flag);
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-091"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Spawn task to trigger abort flag after brief pause
        let canceller = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            flag_clone.store(true, Ordering::SeqCst);
        });

        let err = execute_tool(&tool_ctx, "ask_user", &serde_json::json!({
            "text": "Will be aborted?"
        }))
        .await
        .expect_err("Must error on abort");

        canceller.await.expect("join canceller");
        assert!(err.to_string().contains("abort") || err.to_string().contains("Operation aborted"), "got: {}", err);

        // Verify registry is cleaned up
        let reg = QUESTION_REGISTRY.lock().await;
        assert!(reg.is_empty(), "Registry must be empty after abort");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_todo_write_checklist_updates_and_status_transitions() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_todo_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        // Seed task in tasks.json
        let task = make_test_task("CVH-095", "dev", "development");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-095"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Call todo_write with varied statuses (including 'done' and 'running' which normalize)
        let todo_args = serde_json::json!({
            "todos": [
                { "content": "Scaffold backend", "status": "done" },
                { "content": "Execute unit tests", "status": "running" },
                { "content": "Review PR", "status": "pending" },
                { "content": "Write docs", "status": "completed" }
            ]
        });

        let res = execute_tool(&tool_ctx, "todo_write", &todo_args)
            .await
            .expect("todo_write execution");

        assert_eq!(res, "Updated todo list: 2 completed, 1 in progress, 1 pending.");

        // Check on-disk persistence
        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-095").expect("task exists");
        let todos = t.todos.as_ref().expect("todos must be persisted");
        assert_eq!(todos.len(), 4);
        assert_eq!(todos[0].status, "completed");
        assert_eq!(todos[1].status, "in_progress");
        assert_eq!(todos[2].status, "pending");
        assert_eq!(todos[3].status, "completed");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    // -------------------------------------------------------------------------
    // d) Worktree Filesystem & Shell: read, write, edit, glob, grep, execute
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_worktree_filesystem_read_write_edit_and_path_security() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_fs_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-100"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. write_file
        let write_res = execute_tool(&tool_ctx, "write_file", &serde_json::json!({
            "file_path": "src/demo.txt",
            "content": "line 1
line 2
line 3
line 2 again"
        }))
        .await
        .expect("write_file");
        assert!(write_res.contains("Successfully wrote"), "got: {}", write_res);

        // 2. read_file with offset and limit
        let read_res = execute_tool(&tool_ctx, "read_file", &serde_json::json!({
            "file_path": "src/demo.txt",
            "offset": 2,
            "limit": 2
        }))
        .await
        .expect("read_file");
        assert!(read_res.contains("2: line 2"), "got: {}", read_res);
        assert!(read_res.contains("3: line 3"), "got: {}", read_res);
        assert!(!read_res.contains("1: line 1"));

        // 3. edit_file with single replacement
        let edit_res = execute_tool(&tool_ctx, "edit_file", &serde_json::json!({
            "file_path": "src/demo.txt",
            "old_string": "line 3",
            "new_string": "LINE_THREE"
        }))
        .await
        .expect("edit_file single");
        assert!(edit_res.contains("Successfully updated"));

        // 4. edit_file ambiguous match when replace_all is false
        let edit_ambig = execute_tool(&tool_ctx, "edit_file", &serde_json::json!({
            "file_path": "src/demo.txt",
            "old_string": "line 2",
            "new_string": "LINE_TWO",
            "replace_all": false
        }))
        .await
        .expect_err("Ambiguous match must fail without replace_all");
        assert!(edit_ambig.to_string().contains("Ambiguous match"), "got: {}", edit_ambig);

        // 5. edit_file with replace_all: true
        let edit_all = execute_tool(&tool_ctx, "edit_file", &serde_json::json!({
            "file_path": "src/demo.txt",
            "old_string": "line 2",
            "new_string": "LINE_TWO",
            "replace_all": true
        }))
        .await
        .expect("edit_file replace_all");
        assert!(edit_all.contains("Successfully updated"));

        // 6. edit_file mismatch error
        let edit_mismatch = execute_tool(&tool_ctx, "edit_file", &serde_json::json!({
            "file_path": "src/demo.txt",
            "old_string": "nonexistent text",
            "new_string": "something"
        }))
        .await
        .expect_err("Mismatch must fail");
        assert!(edit_mismatch.to_string().contains("not found in 'src/demo.txt'"), "got: {}", edit_mismatch);

        // 7. Path security: Directory traversal must be blocked by resolve_safe_path
        let sec_err = execute_tool(&tool_ctx, "read_file", &serde_json::json!({
            "file_path": "../../../secret.txt"
        }))
        .await
        .expect_err("Directory traversal must fail");
        assert!(sec_err.to_string().contains("escape worktree") || sec_err.to_string().contains("Path traversal denied"), "got: {}", sec_err);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_crlf_vs_lf_normalization_edit_file() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_crlf_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dirs");

        let policy_json = r#"{"globalEnabled":{"edit_file":true,"read_file":true,"write_file":true}}"#;
        let tool_policy: ToolPolicyConfig = serde_json::from_str(policy_json).unwrap();
        let abort_flag = Arc::new(AtomicBool::new(false));

        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&abort_flag),
            tool_policy: &tool_policy,
            task_id: None,
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: Some(0.0),
            reasoning_effort: None,
            call_id: None,
        };

        // 1. File on disk with Windows CRLF (\r\n)
        let crlf_content = "line 1\r\nline 2\r\nline 3\r\nline 4\r\n";
        let file_path = base.join("style.css");
        tokio::fs::write(&file_path, crlf_content).await.unwrap();

        // LLM sends old_string with LF (\n) - user screenshot 4 root cause
        let old_str_lf = "line 2\nline 3\n";
        let new_str_lf = "line 2_modified\nline 3_modified\n";

        let edit_res = execute_tool(&tool_ctx, "edit_file", &serde_json::json!({
            "file_path": "style.css",
            "old_string": old_str_lf,
            "new_string": new_str_lf
        }))
        .await
        .expect("edit_file with CRLF file and LF old_string must succeed");

        assert!(edit_res.contains("Successfully updated"));

        // Verify the file on disk retains CRLF line endings
        let updated_content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(updated_content.contains("\r\n"), "File must retain CRLF line endings");
        assert!(updated_content.contains("line 2_modified\r\nline 3_modified\r\n"));

        // 2. Pure helper function unit tests for replace_text_normalized
        let sample_crlf = "@layer base {\r\n  :root {\r\n    --bg: #000;\r\n  }\r\n}\r\n";
        let search_lf = "  :root {\n    --bg: #000;\n  }";
        let replace_lf = "  :root {\n    --bg: #111;\n  }";

        let res = replace_text_normalized(sample_crlf, search_lf, replace_lf, false).unwrap();
        assert!(res.contains("\r\n"), "Result must preserve CRLF");
        assert!(res.contains("--bg: #111;\r\n"));

        // 3. Reverse case: file has LF, LLM sends CRLF
        let sample_lf = "first line\nsecond line\nthird line\n";
        let search_crlf = "second line\r\n";
        let replace_crlf = "2nd line\r\n";
        let res_lf = replace_text_normalized(sample_lf, search_crlf, replace_crlf, false).unwrap();
        assert!(!res_lf.contains("\r\n"), "Result must preserve LF");
        assert!(res_lf.contains("first line\n2nd line\nthird line\n"));

        // 4. Multiline string replacement with CRLF
        let multiline_crlf = "header\r\nbody_part_1\r\nbody_part_2\r\nfooter\r\n";
        let multi_search = "body_part_1\nbody_part_2";
        let multi_replace = "new_body_content";
        let res_multi = replace_text_normalized(multiline_crlf, multi_search, multi_replace, false).unwrap();
        assert!(res_multi.contains("header\r\nnew_body_content\r\nfooter\r\n"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_worktree_glob_and_grep() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_glob_grep_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join("src").join("sub")).await.expect("test dirs");

        tokio::fs::write(
            base.join("src").join("main.rs"),
            r#"fn main() { println!("HELLO_GREP_MARKER"); }"#,
        )
        .await
        .expect("write main");
        tokio::fs::write(base.join("src").join("lib.rs"), "pub fn test_fn() {}
")
            .await
            .expect("write lib");
        tokio::fs::write(base.join("src").join("sub").join("mod.rs"), "// sub module
")
            .await
            .expect("write mod");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-101"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. glob test
        let glob_res = execute_tool(&tool_ctx, "glob", &serde_json::json!({
            "pattern": "**/*.rs"
        }))
        .await
        .expect("glob execution");
        assert!(glob_res.contains("main.rs"), "got: {}", glob_res);
        assert!(glob_res.contains("lib.rs"), "got: {}", glob_res);
        assert!(glob_res.contains("mod.rs"), "got: {}", glob_res);

        // 2. grep test
        let grep_res = execute_tool(&tool_ctx, "grep", &serde_json::json!({
            "pattern": "HELLO_GREP_MARKER"
        }))
        .await
        .expect("grep execution");
        assert!(grep_res.contains("main.rs:1:"), "got: {}", grep_res);
        assert!(grep_res.contains("HELLO_GREP_MARKER"), "got: {}", grep_res);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_execute_command_timeout_cleanup_and_log_spilling() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_cmd_spill_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-102"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Successful command execution
        #[cfg(target_os = "windows")]
        let cmd = "echo EXEC_SUCCESS";
        #[cfg(not(target_os = "windows"))]
        let cmd = "echo EXEC_SUCCESS";

        let res = execute_tool(&tool_ctx, "execute_command", &serde_json::json!({
            "command": cmd
        }))
        .await
        .expect("execute_command");

        assert!(res.contains("[Exit Code: 0]"), "got: {}", res);
        assert!(res.contains("EXEC_SUCCESS"), "got: {}", res);

        // 2. Command timeout test
        #[cfg(target_os = "windows")]
        let sleep_cmd = "ping 127.0.0.1 -n 5 > nul";
        #[cfg(not(target_os = "windows"))]
        let sleep_cmd = "sleep 5";

        let timeout_err = execute_tool(&tool_ctx, "execute_command", &serde_json::json!({
            "command": sleep_cmd,
            "timeout_seconds": 1
        }))
        .await
        .expect_err("Command must time out");

        assert!(timeout_err.to_string().contains("timed out"), "got: {}", timeout_err);

        // 3. Output spilling: >100 lines must be spilled to .cvh/spill/
        let mut large_output = String::new();
        for i in 1..=120 {
            large_output.push_str(&format!("This is log line {}\n", i));
        }

        let spilled = maybe_spill_output(&base, "execute_command", large_output)
            .await
            .expect("spill output");

        assert!(spilled.contains("[Output truncated: 120 lines"), "got: {}", spilled);
        assert!(spilled.contains("spilled to .cvh/spill/"), "got: {}", spilled);
        assert!(spilled.contains("--- HEAD (first 20 lines) ---"), "got: {}", spilled);
        assert!(spilled.contains("--- TAIL (last 20 lines) ---"), "got: {}", spilled);

        // Verify spilled log file actually exists on disk
        let spill_dir = base.join(".cvh").join("spill");
        assert!(spill_dir.is_dir(), "spill dir must exist");
        let mut entries = tokio::fs::read_dir(&spill_dir).await.expect("read spill dir");
        let first_file = entries.next_entry().await.expect("entry").expect("some entry");
        assert!(first_file.file_name().to_string_lossy().contains("execute_command.log"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    // -------------------------------------------------------------------------
    // e) Orchestration & Invariants: monotonic ID with gaps, complete_task, cvh-dev gating
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_orchestrator_monotonic_id_with_gaps() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_gaps_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        // Seed CVH-002 and CVH-007 (gap between 2 and 7)
        let t1 = make_test_task("CVH-002", "dev", "development");
        let t2 = make_test_task("CVH-007", "dev", "development");
        crate::commands::tasks::save_tasks_to_disk(&base, &[t1, t2]).await.expect("save tasks");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("orchestrator"),
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-007"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // Creating a task must assign CVH-008 (monotonic max+1, never reuses 001, 003, etc.)
        let res = execute_tool(&tool_ctx, "create_task", &serde_json::json!({
            "title": "Next In Sequence Task"
        }))
        .await
        .expect("create_task");

        let parsed: serde_json::Value = serde_json::from_str(&res).expect("parse json");
        assert_eq!(parsed["task_id"], "CVH-008", "Must assign CVH-008, not fill gaps");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_complete_task_in_orchestrator_role_without_children() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_orch_comp_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in 'dev' with role 'orchestrator'
        let task = make_test_task("CVH-110", "dev", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("orchestrator"), // Orchestrator role bypasses child subtask requirement
            abort_handle: flag,
            tool_policy: &policy,
            task_id: Some("CVH-110"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        let res = execute_tool(&tool_ctx, "complete_task", &serde_json::json!({
            "summary": "High level orchestration concluded"
        }))
        .await
        .expect("complete_task in orchestrator role");

        let parsed: serde_json::Value = serde_json::from_str(&res).expect("parse json");
        assert_eq!(parsed["task_id"], "CVH-110");
        assert_eq!(parsed["status"], "done");
        assert_eq!(parsed["column_id"], "ready");

        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-110").unwrap();
        assert_eq!(t.stage_id, "ready");
        assert_eq!(t.status.as_deref(), Some("done"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_cvh_dev_seven_tools_gating_comprehensive() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_dev_gate_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let cvh_dev_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("cvh-dev"),
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

        // Allowed 7 configuration tools
        for tool in CVH_DEV_TOOLS {
            assert!(
                CVH_DEV_TOOLS.contains(tool),
                "CVH_DEV_TOOLS must contain {}",
                tool
            );
        }
        assert_eq!(CVH_DEV_TOOLS.len(), 7, "Strictly 7 tools allowed for cvh-dev");

        // Any non-configuration tool must be rejected with PermissionDenied
        let forbidden_for_cvh_dev = [
            "subagent",
            "subagent_bg",
            "collect_subagent",
            "todo_write",
            "advance_task",
            "move_task",
            "reject_task",
            "question",
            "create_task",
            "create_tasks",
            "complete_task",
            "web_fetch",
        ];

        for tool in &forbidden_for_cvh_dev {
            let err = execute_tool(&cvh_dev_ctx, tool, &serde_json::json!({}))
                .await
                .expect_err(&format!("Tool '{}' must be forbidden for cvh-dev", tool));

            assert!(
                err.to_string().contains("PermissionDenied"),
                "Expected PermissionDenied for '{}', got: {}",
                tool,
                err
            );
            assert!(
                err.to_string().contains("cvh-dev"),
                "Error should mention role 'cvh-dev', got: {}",
                err
            );
        }

        let _ = tokio::fs::remove_dir_all(&base).await;
    }
