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

        // When load_rule is not in active tools (fallback), rules are inlined alphabetically.
        let tools_no_loadrule = vec!["read_file".to_string(), "edit_file".to_string()];
        let prompt = compile_primed_system_prompt_with_tools(&base, "dev", Some(&tools_no_loadrule))
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
    async fn test_system_prompt_jit_rules_with_load_rule() {
        // When load_rule is present, rule bodies are decoupled and JIT modular rules banner is added.
        let base =
            std::path::PathBuf::from("target").join(format!("cvh_jit_rules_{}", uuid::Uuid::new_v4()));
        let agents = base.join(".cvh").join("agents");
        let role_dir = agents.join("dev");
        tokio::fs::create_dir_all(role_dir.join("rules"))
            .await
            .expect("test dirs");
        tokio::fs::write(role_dir.join("persona.md"), "FLAT_PERSONA_MARKER")
            .await
            .expect("persona seed");
        tokio::fs::write(role_dir.join("rules").join("a_first.md"), "RULE_A_BODY_SECRET")
            .await
            .expect("rule a");
        tokio::fs::write(role_dir.join("rules").join("b_second.md"), "RULE_B_BODY_SECRET")
            .await
            .expect("rule b");

        let tools_with_loadrule = vec!["read_file".to_string(), "load_rule".to_string()];
        let prompt = compile_primed_system_prompt_with_tools(&base, "dev", Some(&tools_with_loadrule))
            .await
            .expect("prompt failed");

        assert!(prompt.contains("FLAT_PERSONA_MARKER"));
        assert!(prompt.contains("# Modular Rules (JIT On-Demand)"));
        assert!(prompt.contains("Architectural standards, domain invariants, and tech guidelines are decoupled from the prompt to optimize tokens."));
        assert!(prompt.contains("Use the `load_rule` tool to dynamically load rules when working on specific topics or technologies."));
        assert!(!prompt.contains("RULE_A_BODY_SECRET"), "Rule bodies must NOT be inlined when load_rule is active!");
        assert!(!prompt.contains("RULE_B_BODY_SECRET"), "Rule bodies must NOT be inlined when load_rule is active!");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_create_task_allowed_in_active_tools_with_policy_override() {
        use std::collections::HashMap;
        let mut policy = crate::config::ToolPolicyConfig::default();

        // 1. When overridden to false in stage_overrides, create_task must be inactive
        let mut dev_map = HashMap::new();
        dev_map.insert("create_task".to_string(), false);
        dev_map.insert("complete_task".to_string(), false);
        dev_map.insert("read_file".to_string(), true);
        policy.stage_overrides.insert("dev".to_string(), dev_map);

        assert!(
            !is_tool_active_for_agent("create_task", &policy, "developer", Some("dev"), None),
            "create_task must be inactive when overridden to false"
        );

        // 2. When explicitly enabled by user in policy matrix, create_task must be active!
        let mut dev_map_allowed = HashMap::new();
        dev_map_allowed.insert("create_task".to_string(), true);
        dev_map_allowed.insert("complete_task".to_string(), true);
        dev_map_allowed.insert("read_file".to_string(), true);
        policy.stage_overrides.insert("dev".to_string(), dev_map_allowed);

        assert!(
            is_tool_active_for_agent("create_task", &policy, "developer", Some("dev"), None),
            "create_task must be active for developer when explicitly enabled in tool-policy!"
        );
        assert!(
            is_tool_active_for_agent("complete_task", &policy, "developer", Some("dev"), None),
            "complete_task must be active for developer when explicitly enabled in tool-policy!"
        );
        assert!(
            is_tool_active_for_agent("create_task", &policy, "cvh-dev", Some("dev"), None),
            "create_task must be active for cvh-dev when enabled in dev stage-override"
        );
        assert!(
            is_tool_active_for_agent("complete_task", &policy, "cvh-dev", Some("dev"), None),
            "complete_task must be active for cvh-dev when enabled in dev stage-override"
        );
        assert!(
            is_tool_active_for_agent("complete_task", &policy, "cvh-dev", None, None),
            "complete_task must be active for cvh-dev even without stage_id via fallback convention"
        );

        let mut cvh_denied_policy = crate::config::ToolPolicyConfig::default();
        let mut cvh_denied_map = HashMap::new();
        cvh_denied_map.insert("complete_task".to_string(), false);
        cvh_denied_policy.stage_overrides.insert("cvh-dev".to_string(), cvh_denied_map);
        assert!(
            !is_tool_active_for_agent("complete_task", &cvh_denied_policy, "cvh-dev", None, None),
            "complete_task must be inactive for cvh-dev when role override is false"
        );

        // 3. Orchestrator role is permitted
        let orch_policy = crate::config::ToolPolicyConfig::default();
        assert!(
            is_tool_active_for_agent("create_task", &orch_policy, "orchestrator", Some("orchestrator"), None),
            "orchestrator must be able to create tasks"
        );
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
        let workdir = std::path::PathBuf::from("target").join(format!("cvh_test_agent_{}", uuid::Uuid::new_v4()));
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
            skip_worktree: None,
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

        // Verify on disk that CVH-002 has parent_id = None (standalone root task by default)
        let tasks_on_disk = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let created_child = tasks_on_disk.iter().find(|t| t.id == "CVH-002").expect("CVH-002 on disk");
        assert_eq!(created_child.parent_id.as_deref(), None);
        assert_eq!(created_child.priority.as_deref(), Some("high"));
        assert_eq!(created_child.tags.as_ref().unwrap(), &vec!["rust".to_string(), "tauri".to_string()]);

        // 2. Batch create_tasks call
        let batch_args = serde_json::json!({
            "tasks": [
                {
                    "title": "Frontend UI Matrix",
                    "description": "Integrate into toolPolicyStore.ts",
                    "column_id": "dev",
                    "priority": "medium",
                    "is_subtask": true
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
        assert_eq!(t4.parent_id.as_deref(), None);

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
            skip_worktree: None,
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

        // 1. When allowed by tool-policy (default for dev), calling complete_task succeeds directly without requiring child tasks
        let complete_res = execute_tool(&dev_ctx, "complete_task", &serde_json::json!({
            "summary": "Finished task directly without requiring child subtasks"
        }))
        .await
        .expect("complete_task must succeed when allowed by tool-policy");

        let parsed_res: serde_json::Value = serde_json::from_str(&complete_res).expect("parse complete json");
        assert_eq!(parsed_res["task_id"], "CVH-010");
        assert_eq!(parsed_res["status"], "done");
        assert_eq!(parsed_res["column_id"], "ready");

        // Verify on disk that CVH-010 moved to 'ready', status is 'done', and handover_summary is recorded
        let tasks_final = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t10_done = tasks_final.iter().find(|t| t.id == "CVH-010").unwrap();
        assert_eq!(t10_done.stage_id, "ready");
        assert_eq!(t10_done.status.as_deref(), Some("done"));
        assert_eq!(t10_done.handover_summary.as_deref(), Some("Finished task directly without requiring child subtasks"));

        // 2. When policy explicitly denies complete_task, it is rejected with PermissionDenied
        let mut denied_policy = ToolPolicyConfig::default();
        denied_policy.stage_overrides.get_mut("dev").unwrap().insert("complete_task".to_string(), false);
        let denied_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &denied_policy,
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
        let bypass_err = execute_tool(&denied_ctx, "complete_task", &serde_json::json!({
            "summary": "Should be denied by policy"
        }))
        .await
        .expect_err("complete_task must be rejected when disabled by policy");
        assert!(bypass_err.to_string().contains("PermissionDenied"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_cvh_dev_card_tools_governed_by_policy() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_dev_scope_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let seed_task = make_test_task("CVH-999", "dev", "development");
        crate::commands::tasks::save_tasks_to_disk(&base, &[seed_task]).await.expect("save seed task");

        let flag = Arc::new(AtomicBool::new(false));

        // 1. When policy denies create_task and complete_task for cvh-dev, it must fail with PermissionDenied
        let mut denied_policy = ToolPolicyConfig::default();
        let mut cvh_denied_map = HashMap::new();
        cvh_denied_map.insert("create_task".to_string(), false);
        cvh_denied_map.insert("complete_task".to_string(), false);
        denied_policy.stage_overrides.insert("cvh-dev".to_string(), cvh_denied_map);

        let cvh_dev_denied_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("cvh-dev"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &denied_policy,
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

        let create_err = execute_tool(&cvh_dev_denied_ctx, "create_task", &serde_json::json!({
            "title": "Forbidden task"
        }))
        .await
        .expect_err("cvh-dev must be rejected when policy disables create_task");
        assert!(create_err.to_string().contains("PermissionDenied"));

        let complete_err = execute_tool(&cvh_dev_denied_ctx, "complete_task", &serde_json::json!({
            "summary": "Trying to complete task"
        }))
        .await
        .expect_err("cvh-dev must be rejected when policy disables complete_task");
        assert!(complete_err.to_string().contains("PermissionDenied"));

        // 2. When policy explicitly allows create_task and complete_task for cvh-dev, it must succeed without hardcoded role errors!
        let mut allowed_policy = ToolPolicyConfig::default();
        let mut cvh_allowed_map = HashMap::new();
        cvh_allowed_map.insert("create_task".to_string(), true);
        cvh_allowed_map.insert("complete_task".to_string(), true);
        allowed_policy.stage_overrides.insert("cvh-dev".to_string(), cvh_allowed_map);

        let cvh_dev_allowed_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("cvh-dev"),
            abort_handle: Arc::clone(&flag),
            tool_policy: &allowed_policy,
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

        let create_res = execute_tool(&cvh_dev_allowed_ctx, "create_task", &serde_json::json!({
            "title": "Allowed Task by Policy"
        }))
        .await
        .expect("cvh-dev must succeed in create_task when allowed by policy");
        assert!(create_res.contains("Allowed Task by Policy"));

        let complete_res = execute_tool(&cvh_dev_allowed_ctx, "complete_task", &serde_json::json!({
            "summary": "Task completed by cvh-dev per policy"
        }))
        .await
        .expect("cvh-dev must succeed in complete_task when allowed by policy");
        assert!(complete_res.contains("ready") || complete_res.contains("done"));

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
            skip_worktree: None,
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
        assert_eq!(t.stage_id, "dev");
        assert_eq!(t.status.as_deref(), Some("waiting_subtask"));
        assert_eq!(t.waiting_for_subtask_id.as_deref(), Some("CVH-050.1"));

        let sub = tasks_after_adv.iter().find(|t| t.id == "CVH-050.1").unwrap();
        assert_eq!(sub.stage_id, "review");

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
            skip_worktree: None,
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

        // 2. Advance from dev -> review creates review subtask CVH-060.1 (§ 2.2)
        let res2 = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({ "summary": "Implemented feature" }))
            .await
            .expect("advance dev -> review");
        assert!(res2.contains("УСПЕШНО"), "got: {}", res2);

        let tasks2 = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t2 = tasks2.iter().find(|t| t.id == "CVH-060").expect("find parent task");
        assert_eq!(t2.stage_id, "dev", "Parent must remain in dev");
        assert_eq!(t2.status.as_deref(), Some("waiting_subtask"));
        assert_eq!(t2.waiting_for_subtask_id.as_deref(), Some("CVH-060.1"));

        let sub2 = tasks2.iter().find(|t| t.id == "CVH-060.1").expect("find review subtask");
        assert_eq!(sub2.stage_id, "review", "Subtask must be in review stage");

        // 3. Advance subtask from review -> ready (terminal)
        let res3 = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({ "task_id": "CVH-060.1", "summary": "Review approved" }))
            .await
            .expect("advance review -> ready");
        assert!(res3.contains("УСПЕШНО"), "got: {}", res3);

        let tasks3 = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let sub3 = tasks3.iter().find(|t| t.id == "CVH-060.1").expect("find subtask");
        assert_eq!(sub3.stage_id, "ready");
        assert_eq!(sub3.status.as_deref(), Some("done"));

        // 4. Boundary error: Advance when already in terminal ready stage must fail
        let res_err = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({ "task_id": "CVH-060.1" }))
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
    async fn test_resilient_whitespace_relaxed_matching() {
        // 1. Indentation mismatch: file has 4 spaces, LLM supplies 2 spaces in old_string and new_string
        let file_content = "fn main() {\n    let price = 100;\n    let tax = 20;\n}\n";
        let search = "  let price = 100;\n  let tax = 20;";
        let replace = "  let price = 150;\n  let discount = 10;";
        let result = replace_text_normalized(file_content, search, replace, false).expect("whitespace match");
        assert!(result.contains("    let price = 150;\n    let discount = 10;\n"), "Must adapt indentation to match file");

        // 2. Trailing spaces mismatch in file
        let file_trailing = "fn run() {\r\n    step_one();   \r\n    step_two(); \r\n}\r\n";
        let search_clean = "    step_one();\r\n    step_two();";
        let replace_clean = "    step_three();";
        let res_trailing = replace_text_normalized(file_trailing, search_clean, replace_clean, false).expect("trailing match");
        assert!(res_trailing.contains("fn run() {\r\n    step_three();\r\n}\r\n"));
        assert!(res_trailing.contains("\r\n"), "Must preserve CRLF");

        // 3. Ambiguous match: same block occurs twice with relaxed whitespace
        // File has 2 spaces on each line, search has 4 spaces (exact substring search fails)
        let ambiguous_file = "  val a = 1;\n\n  val a = 1;\n";
        let amb_search = "    val a = 1;";
        let amb_replace = "    val a = 2;";
        let amb_err = replace_text_normalized(ambiguous_file, amb_search, amb_replace, false).unwrap_err();
        assert!(amb_err.contains("Ambiguous match"));
        assert!(amb_err.contains("whitespace-relaxed"));

        // With replace_all: true, both occurrences are replaced
        let all_replaced = replace_text_normalized(ambiguous_file, amb_search, amb_replace, true).expect("replace all");
        assert_eq!(all_replaced, "  val a = 2;\n\n  val a = 2;\n");
    }

    #[tokio::test]
    async fn test_honest_mismatch_diagnostics_no_fake_100_percent() {
        let content = r#"pub fn process_data(input: &str) -> Result<String> {
    let sanitized = input.trim();
    let validated = validate_input(sanitized)?;
    let result = format!("processed: {}", validated);
    Ok(result)
}"#;

        // old_string where line 1 ("pub fn process_data...") matches 100%, but line 2 differs!
        let old_string_partial_match = r#"pub fn process_data(input: &str) -> Result<String> {
    let sanitized = input.to_lowercase();
    let validated = validate_input(sanitized)?;
}"#;

        let err_msg = build_edit_mismatch_error(content, "src/data.rs", old_string_partial_match);

        // Verification 1: NEVER claim 100% similarity when subsequent lines differ!
        assert!(!err_msg.contains("similarity: 100%"), "Must NOT claim 100% similarity when line 2 differs!");

        // Verification 2: Block similarity must be reported honestly (< 100%)
        assert!(err_msg.contains("block similarity:"), "Must report multi-line block similarity");

        // Verification 3: Line-by-line diff must distinguish matches and mismatches
        assert!(err_msg.contains("Line 1 (file line 1): EXACT MATCH"));
        assert!(err_msg.contains("Line 2 (file line 2): MISMATCH"));

        // Verification 4: Token-level diff must pinpoint the exact token discrepancy
        assert!(err_msg.contains("expected 'to_lowercase', found 'trim'"));

        // Verification 5: Must output candidate block with line indicators
        assert!(err_msg.contains(">>    1 | pub fn process_data(input: &str) -> Result<String> {"));
        assert!(err_msg.contains(">>    2 |     let sanitized = input.trim();"));
        assert!(err_msg.contains(">>    3 |     let validated = validate_input(sanitized)?;"));

        // Verification 6: Must include actionable recommendation
        assert!(err_msg.contains("Recommendation: Use the exact candidate lines above or inspect with read_file"));
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

        // 3. Output spilling (§ Directive 1):
        // Generic commands: ceiling is 500 lines / 64KB
        let mut large_output = String::new();
        for i in 1..=520 {
            large_output.push_str(&format!("This is log line {}\n", i));
        }

        let spilled = maybe_spill_output(&base, "execute_command", large_output)
            .await
            .expect("spill output");

        assert!(spilled.contains("[Output truncated: 520 lines"), "got: {}", spilled);
        assert!(spilled.contains("spilled to .cvh/spill/"), "got: {}", spilled);
        assert!(spilled.contains("--- HEAD (first 20 lines) ---"), "got: {}", spilled);
        assert!(spilled.contains("--- TAIL (last 20 lines) ---"), "got: {}", spilled);

        // Verify subagent reports NEVER spill
        let mut subagent_rep = String::new();
        for i in 1..=600 {
            subagent_rep.push_str(&format!("Subagent finding {}\n", i));
        }
        let unspilled_sub = maybe_spill_output(&base, "subagent", subagent_rep.clone())
            .await
            .expect("subagent unspilled");
        assert_eq!(unspilled_sub, subagent_rep, "subagent reports must NEVER be truncated");

        let unspilled_collect = maybe_spill_output(&base, "collect_subagent", subagent_rep.clone())
            .await
            .expect("collect_subagent unspilled");
        assert_eq!(unspilled_collect, subagent_rep, "collect_subagent reports must NEVER be truncated");

        // Verify read_file: 600 lines does NOT spill (safe limit is 2000 lines)
        let unspilled_read = maybe_spill_output(&base, "read_file", subagent_rep)
            .await
            .expect("read_file unspilled at 600 lines");
        assert!(!unspilled_read.contains("[Output truncated"), "read_file must not spill under 2000 lines");

        // read_file with 2100 lines DOES spill
        let mut massive_file = String::new();
        for i in 1..=2100 {
            massive_file.push_str(&format!("File line {}\n", i));
        }
        let spilled_read = maybe_spill_output(&base, "read_file", massive_file)
            .await
            .expect("read_file spilled at 2100 lines");
        assert!(spilled_read.contains("[Output truncated: 2100 lines"), "got: {}", spilled_read);

        // Verify spilled log file actually exists on disk
        let spill_dir = base.join(".cvh").join("spill");
        assert!(spill_dir.is_dir(), "spill dir must exist");
        let mut entries = tokio::fs::read_dir(&spill_dir).await.expect("read spill dir");
        let first_file = entries.next_entry().await.expect("entry").expect("some entry");
        assert!(first_file.file_name().to_string_lossy().contains(".log"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn test_execute_command_windows_quoting_python() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_cmd_quote_{}", uuid::Uuid::new_v4()));
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
            task_id: Some("CVH-QUOTE-TEST"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Verify python -c "print(1)" succeeds with exit code 0 and stdout "1"
        let res = execute_tool(&tool_ctx, "execute_command", &serde_json::json!({
            "command": "python -c \"print(1)\""
        }))
        .await
        .expect("execute_command with quotes");

        assert!(res.contains("[Exit Code: 0]"), "got: {}", res);
        assert!(res.contains("--- STDOUT ---
1"), "stdout should contain 1, got: {}", res);

        // 2. Verify complex nested quotes: python -c "import json; print(json.dumps({'status': 'ok'}))"
        let res_complex = execute_tool(&tool_ctx, "execute_command", &serde_json::json!({
            "command": "python -c \"import json; print(json.dumps({'status': 'ok'}))\""
        }))
        .await
        .expect("execute_command with complex quotes");

        assert!(res_complex.contains("[Exit Code: 0]"), "got: {}", res_complex);
        assert!(res_complex.contains(r#"{"status": "ok"}"#), "stdout should contain json, got: {}", res_complex);

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
    async fn test_cvh_dev_tool_policy_matrix_gating_and_card_invariants() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_dev_gate_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let flag = Arc::new(AtomicBool::new(false));
        let mut policy = ToolPolicyConfig::default();
        let mut cvh_denied = HashMap::new();
        cvh_denied.insert("create_task".to_string(), false);
        cvh_denied.insert("create_tasks".to_string(), false);
        cvh_denied.insert("complete_task".to_string(), false);
        policy.stage_overrides.insert("cvh-dev".to_string(), cvh_denied);

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

        // 1. Tool policy matrix enforcement: when card manipulation tools are disabled by policy, cvh-dev is denied
        let forbidden_card_tools = ["create_task", "create_tasks", "complete_task"];
        for tool in &forbidden_card_tools {
            let err = execute_tool(&cvh_dev_ctx, tool, &serde_json::json!({ "title": "Test" }))
                .await
                .expect_err(&format!("Tool '{}' must be forbidden for cvh-dev when disabled in policy", tool));

            assert!(
                err.to_string().contains("PermissionDenied"),
                "Expected PermissionDenied for '{}', got: {}",
                tool,
                err
            );
        }

        // 1b. When explicitly allowed by tool policy matrix, card tools succeed for cvh-dev!
        let mut allowed_policy = ToolPolicyConfig::default();
        let mut cvh_allowed = HashMap::new();
        cvh_allowed.insert("create_task".to_string(), true);
        cvh_allowed.insert("complete_task".to_string(), true);
        allowed_policy.stage_overrides.insert("cvh-dev".to_string(), cvh_allowed);
        let cvh_dev_allowed_ctx = ToolContext {
            tool_policy: &allowed_policy,
            ..cvh_dev_ctx.clone()
        };
        let create_res = execute_tool(&cvh_dev_allowed_ctx, "create_task", &serde_json::json!({ "title": "Test Task" })).await;
        assert!(create_res.is_ok(), "create_task must succeed for cvh-dev when allowed by policy");

        // 2. Tools allowed by the Tool Policy Matrix (including todo_write) work for cvh-dev
        let todo_res = execute_tool(&cvh_dev_ctx, "todo_write", &serde_json::json!({
            "todos": [{ "content": "Configure pipeline", "status": "in_progress" }]
        })).await;
        assert!(todo_res.is_ok(), "todo_write must succeed for cvh-dev when allowed by policy, got: {:?}", todo_res.err());

        // 3. When policy disables a tool, cvh-dev is denied by the Tool Policy Matrix
        let mut restricted_policy = ToolPolicyConfig::default();
        restricted_policy.global_enabled.insert("web_search".to_string(), false);
        let restricted_ctx = ToolContext {
            tool_policy: &restricted_policy,
            ..cvh_dev_ctx.clone()
        };
        let search_err = execute_tool(&restricted_ctx, "web_search", &serde_json::json!({ "query": "rust" }))
            .await
            .expect_err("web_search must be denied when disabled in tool-policy");
        assert!(search_err.to_string().contains("PermissionDenied"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    // -------------------------------------------------------------------------
    // Agent Loop Clean Natural Termination on Stage Transition
    // -------------------------------------------------------------------------

    async fn start_mock_sse_server(
        sse_payload: &'static str,
        counter: Arc<std::sync::atomic::AtomicUsize>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock listener");
        let port = listener.local_addr().expect("local addr").port();
        let base_url = format!("http://127.0.0.1:{}", port);

        let handle = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                counter.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    let _ = socket.read(&mut buf).await;
                    let body = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n{}",
                        sse_payload
                    );
                    let _ = socket.write_all(body.as_bytes()).await;
                    let _ = socket.flush().await;
                    let _ = socket.shutdown().await;
                });
            }
        });

        (base_url, handle)
    }

    #[tokio::test]
    async fn test_agent_loop_natural_exit_on_advance_task() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_natural_exit_adv_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in dev stage
        let task = make_test_task("CVH-081", "dev", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let sse_stream = "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_adv_01\",\"function\":{\"name\":\"advance_task\",\"arguments\":\"{\\\"summary\\\":\\\"Feature implemented successfully\\\"}\"}}]}}]}

data: [DONE]

";
        let request_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (base_url, server_handle) = start_mock_sse_server(sse_stream, Arc::clone(&request_counter)).await;

        let config = AgentSessionConfig {
            repo_root: base.clone(),
            worktree_path: base.clone(),
            role: "developer".to_string(),
            stage_id: "dev".to_string(),
            task_id: "CVH-081".to_string(),
            task_title: "Implement feature".to_string(),
            task_description: "Do work".to_string(),
            handover_summary: None,
            steer_prompt: None,
            steer_images: None,
            base_url,
            api_key: None,
            model: "test-model".to_string(),
            temperature: None,
            reasoning_effort: None,
            max_turns: 10,
            app_handle: None,
            context_window: None,
        };

        let abort_flag = Arc::new(AtomicBool::new(false));
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&events);

        let result = run_agent_loop(config, abort_flag, move |ev| {
            events_clone.lock().unwrap().push(ev);
        })
        .await
        .expect("run_agent_loop must succeed");

        server_handle.abort();

        // 1. Result should indicate success
        assert!(result.contains("УСПЕШНО"), "got: {}", result);
        assert!(result.contains("ревью") || result.contains("review"), "got: {}", result);

        // 2. Only 1 request was sent to the LLM (zero extra turns)
        assert_eq!(request_counter.load(Ordering::SeqCst), 1, "Expected exactly 1 request to LLM, no extra turns");

        // 3. Emitted events must contain Finished with final_response == result
        {
            let captured = events.lock().unwrap();
            let finished_event = captured.iter().find(|e| matches!(e, AgentEvent::Finished { .. }));
            assert!(finished_event.is_some(), "AgentEvent::Finished must be emitted");
            if let Some(AgentEvent::Finished { final_response }) = finished_event {
                // Emitted final_response preserves rich agent markdown summary
                assert_eq!(final_response, "Feature implemented successfully");
            }
        }

        // 4. Verify Review Subtask Hierarchy (§ 2.2):
        // Parent remains pinned in 'dev' waiting for subtask, while child subtask CVH-081.1 is in 'review'
        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-081").expect("task found");
        assert_eq!(t.stage_id, "dev");
        assert_eq!(t.status.as_deref(), Some("waiting_subtask"));
        assert_eq!(t.waiting_for_subtask_id.as_deref(), Some("CVH-081.1"));

        let sub = tasks.iter().find(|t| t.id == "CVH-081.1").expect("subtask found");
        assert_eq!(sub.stage_id, "review");
        assert_eq!(sub.status.as_deref(), Some("running"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_agent_loop_natural_exit_on_move_task() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_natural_exit_mov_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in dev stage
        let task = make_test_task("CVH-083", "dev", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let sse_stream = "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_mov_01\",\"function\":{\"name\":\"move_task\",\"arguments\":\"{\\\"stage_id\\\":\\\"ready\\\",\\\"summary\\\":\\\"Fast-track to ready\\\"}\"}}]}}]}

data: [DONE]

";
        let request_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (base_url, server_handle) = start_mock_sse_server(sse_stream, Arc::clone(&request_counter)).await;

        let config = AgentSessionConfig {
            repo_root: base.clone(),
            worktree_path: base.clone(),
            role: "developer".to_string(),
            stage_id: "dev".to_string(),
            task_id: "CVH-083".to_string(),
            task_title: "Fast-track task".to_string(),
            task_description: "Move straight to ready".to_string(),
            handover_summary: None,
            steer_prompt: None,
            steer_images: None,
            base_url,
            api_key: None,
            model: "test-model".to_string(),
            temperature: None,
            reasoning_effort: None,
            max_turns: 10,
            app_handle: None,
            context_window: None,
        };

        let abort_flag = Arc::new(AtomicBool::new(false));
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&events);

        let result = run_agent_loop(config, abort_flag, move |ev| {
            events_clone.lock().unwrap().push(ev);
        })
        .await
        .expect("run_agent_loop must succeed");

        server_handle.abort();

        // 1. Result should indicate success and target column ready
        assert!(result.contains("УСПЕШНО"), "got: {}", result);
        assert!(result.contains("ready"), "got: {}", result);

        // 2. Only 1 request was sent to the LLM (zero extra turns)
        assert_eq!(request_counter.load(Ordering::SeqCst), 1, "Expected exactly 1 request to LLM, no extra turns");

        // 3. Emitted events must contain Finished with final_response == summary
        {
            let captured = events.lock().unwrap();
            let finished_event = captured.iter().find(|e| matches!(e, AgentEvent::Finished { .. }));
            assert!(finished_event.is_some(), "AgentEvent::Finished must be emitted");
            if let Some(AgentEvent::Finished { final_response }) = finished_event {
                // Emitted final_response preserves rich agent markdown summary
                assert_eq!(final_response, "Fast-track to ready");
            }
        }

        // 4. Verify task state on disk advanced to "ready"
        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-083").expect("task found");
        assert_eq!(t.stage_id, "ready");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_agent_loop_natural_exit_on_reject_task() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_natural_exit_rej_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task in review stage
        let task = make_test_task("CVH-082", "review", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let sse_stream = "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_rej_01\",\"function\":{\"name\":\"reject_task\",\"arguments\":\"{\\\"reason\\\":\\\"Unit test failure\\\"}\"}}]}}]}

data: [DONE]

";
        let request_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (base_url, server_handle) = start_mock_sse_server(sse_stream, Arc::clone(&request_counter)).await;

        let config = AgentSessionConfig {
            repo_root: base.clone(),
            worktree_path: base.clone(),
            role: "reviewer".to_string(),
            stage_id: "review".to_string(),
            task_id: "CVH-082".to_string(),
            task_title: "Review feature".to_string(),
            task_description: "Audit code".to_string(),
            handover_summary: None,
            steer_prompt: None,
            steer_images: None,
            base_url,
            api_key: None,
            model: "test-model".to_string(),
            temperature: None,
            reasoning_effort: None,
            max_turns: 10,
            app_handle: None,
            context_window: None,
        };

        let abort_flag = Arc::new(AtomicBool::new(false));
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&events);

        let result = run_agent_loop(config, abort_flag, move |ev| {
            events_clone.lock().unwrap().push(ev);
        })
        .await
        .expect("run_agent_loop must succeed");

        server_handle.abort();

        // 1. Result should indicate rejected / returned to dev
        assert!(result.contains("Задача CVH-082 перемещена на этап 'dev'") || result.contains("ОТКЛОНЕНО") || result.contains("возвращена"), "got: {}", result);

        // 2. Only 1 request was sent to the LLM (zero extra turns)
        assert_eq!(request_counter.load(Ordering::SeqCst), 1, "Expected exactly 1 request to LLM, no extra turns");

        // 3. Emitted events must contain Finished with final_response == result
        {
            let captured = events.lock().unwrap();
            let finished_event = captured.iter().find(|e| matches!(e, AgentEvent::Finished { .. }));
            assert!(finished_event.is_some(), "AgentEvent::Finished must be emitted");
            if let Some(AgentEvent::Finished { final_response }) = finished_event {
                // Emitted final_response preserves rich rejection reason
                assert_eq!(final_response, "Unit test failure");
            }
        }

        // 4. Verify task state on disk returned to "dev" with status "idle"
        let tasks = crate::commands::tasks::load_tasks_from_disk(&base).await;
        let t = tasks.iter().find(|t| t.id == "CVH-082").expect("task found");
        assert_eq!(t.stage_id, "dev");
        assert_eq!(t.status.as_deref(), Some("idle"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_standard_tools_descriptions_english_only() {
        let tools = get_standard_tools();
        assert!(!tools.is_empty(), "standard tools must not be empty");

        fn assert_no_cyrillic(text: &str, context: &str) {
            for ch in text.chars() {
                assert!(
                    !(('а'..='я').contains(&ch) || ('А'..='Я').contains(&ch) || ch == 'ё' || ch == 'Ё'),
                    "Found Cyrillic character '{}' in {}: \"{}\"",
                    ch,
                    context,
                    text
                );
            }
        }

        fn check_json_descriptions(val: &serde_json::Value, path: &str) {
            match val {
                serde_json::Value::Object(map) => {
                    for (k, v) in map {
                        let current_path = format!("{}.{}", path, k);
                        if k == "description" {
                            if let Some(desc_str) = v.as_str() {
                                assert_no_cyrillic(desc_str, &current_path);
                            }
                        }
                        check_json_descriptions(v, &current_path);
                    }
                }
                serde_json::Value::Array(arr) => {
                    for (i, v) in arr.iter().enumerate() {
                        check_json_descriptions(v, &format!("{}[{}]", path, i));
                    }
                }
                _ => {}
            }
        }

        for tool in &tools {
            let name = &tool.function.name;
            assert_no_cyrillic(&tool.function.description, &format!("tool '{}' description", name));
            check_json_descriptions(&tool.function.parameters, &format!("tool '{}' parameters", name));
        }

        // Specifically check get_load_rule_tool
        let load_rule = get_load_rule_tool(Path::new(""), None);
        assert_no_cyrillic(&load_rule.function.description, "load_rule description");
        check_json_descriptions(&load_rule.function.parameters, "load_rule parameters");
    }

    #[tokio::test]
    async fn test_advance_task_failure_does_not_abort_loop_and_shows_available_stages() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_adv_fail_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let (boards_cfg, _) = make_test_board(&base);
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        let task = make_test_task("CVH-089", "dev", "test-board");
        crate::commands::tasks::save_tasks_to_disk(&base, &[task]).await.expect("save task");

        let abort_flag = Arc::new(AtomicBool::new(false));
        let policy = ToolPolicyConfig::default();
        let tool_ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&abort_flag),
            tool_policy: &policy,
            task_id: Some("CVH-089"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 1. Direct tool execution with non-existent target stage returns error with available stages
        let err = execute_tool(&tool_ctx, "advance_task", &serde_json::json!({
            "stage_id": "non_existent_stage"
        }))
        .await
        .expect_err("non existent stage must return Err");
        let err_msg = err.to_string();
        assert!(err_msg.contains("Target stage 'non_existent_stage' not found in columns"));
        assert!(err_msg.contains("Available stages:"));
        assert!(err_msg.contains("dev"));
        assert!(err_msg.contains("ready"));

        // 2. In run_agent_loop, a failed advance_task must NOT trip abort_flag and must continue to next turn
        // Turn 1 calls advance_task with invalid stage -> fails
        // Turn 2 outputs self-correction text
        let sse_stream = r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_adv_fail","function":{"name":"advance_task","arguments":"{\"stage_id\":\"non_existent_stage\"}"}}]}}]}

data: [DONE]

data: {"choices":[{"delta":{"content":"Understood, available stages are dev, review, ready. Retrying."}}]}

data: [DONE]

"#;
        let request_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (base_url, server_handle) = start_mock_sse_server(sse_stream, Arc::clone(&request_counter)).await;

        let config = AgentSessionConfig {
            repo_root: base.clone(),
            worktree_path: base.clone(),
            role: "developer".to_string(),
            stage_id: "dev".to_string(),
            task_id: "CVH-089".to_string(),
            task_title: "Test task".to_string(),
            task_description: "Advance failure test".to_string(),
            handover_summary: None,
            steer_prompt: None,
            steer_images: None,
            base_url,
            api_key: None,
            model: "test-model".to_string(),
            temperature: None,
            reasoning_effort: None,
            max_turns: 10,
            app_handle: None,
            context_window: None,
        };

        let loop_abort_flag = Arc::new(AtomicBool::new(false));
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&events);

        let result = run_agent_loop(config, loop_abort_flag, move |ev| {
            events_clone.lock().unwrap().push(ev);
        })
        .await
        .expect("run_agent_loop must succeed on retry");

        server_handle.abort();

        // Must have made 2 requests to LLM (Turn 1 tool call failed, Turn 2 received tool response and responded)
        assert_eq!(request_counter.load(Ordering::SeqCst), 2, "Expected 2 requests to LLM because advance_task error did not kill loop");
        assert!(result.contains("Understood, available stages are dev, review, ready. Retrying."));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    // -------------------------------------------------------------------------
    // e) Steer Agent Context Preservation & Tool Interruption Sanitization
    // -------------------------------------------------------------------------

    #[test]
    fn test_sanitize_unclosed_tool_calls() {
        let mut messages = vec![
            ChatMessage::system("system prompt"),
            ChatMessage::user("user request"),
            ChatMessage::assistant(
                Some("Calling tools".to_string()),
                Some(vec![
                    ToolCall {
                        id: "call_1".to_string(),
                        call_type: "function".to_string(),
                        function: crate::llm::FunctionCall {
                            name: "read_file".to_string(),
                            arguments: "{}".to_string(),
                        },
                    },
                    ToolCall {
                        id: "call_2".to_string(),
                        call_type: "function".to_string(),
                        function: crate::llm::FunctionCall {
                            name: "write_file".to_string(),
                            arguments: "{}".to_string(),
                        },
                    },
                ]),
            ),
            // Only call_1 was answered before abort/steer
            ChatMessage::tool_response("call_1", "file contents"),
        ];

        sanitize_unclosed_tool_calls(&mut messages);

        // Expect 5 messages: call_2 must have a synthesized tool response
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[4].role, Role::Tool);
        assert_eq!(messages[4].tool_call_id.as_deref(), Some("call_2"));
        assert!(messages[4]
            .content
            .as_deref()
            .unwrap()
            .contains("Инструмент прерван указанием пользователя"));

        // Running sanitize again should be idempotent (no extra messages added)
        sanitize_unclosed_tool_calls(&mut messages);
        assert_eq!(messages.len(), 5);
    }

    #[test]
    fn test_reconstruct_messages_from_dialog_preserves_steer_and_tools() {
        let items = vec![
            ActivityItem {
                id: "act-1".to_string(),
                item_type: "output".to_string(),
                timestamp: "12:00:00".to_string(),
                content: Some("I will check the files".to_string()),
                tool: None,
                duration_ms: None,
                images: None,
            },
            ActivityItem {
                id: "call_read_1".to_string(),
                item_type: "tool".to_string(),
                timestamp: "12:00:01".to_string(),
                content: Some("src/main.rs content here".to_string()),
                tool: Some(serde_json::json!({
                    "id": "call_read_1",
                    "name": "read_file",
                    "arguments": "{\"file_path\":\"src/main.rs\"}",
                    "result": "src/main.rs content here"
                })),
                duration_ms: Some(10.0),
                images: None,
            },
            ActivityItem {
                id: "act-steer-1".to_string(),
                item_type: "steer".to_string(),
                timestamp: "12:00:05".to_string(),
                content: Some("Поправь баг в agent.rs вместо main.rs".to_string()),
                tool: None,
                duration_ms: None,
                images: None,
            },
        ];

        let reconstructed = reconstruct_messages_from_dialog(
            "SYSTEM PROMPT",
            "INITIAL OBJECTIVE",
            &items,
        );

        // Expected sequence:
        // [0] System: "SYSTEM PROMPT"
        // [1] User: "INITIAL OBJECTIVE"
        // [2] Assistant: "I will check the files"
        // [3] Assistant with tool_calls: ["call_read_1"]
        // [4] Tool response: "call_read_1"
        // [5] User with prominent steer banner
        assert_eq!(reconstructed.len(), 6);
        assert_eq!(reconstructed[0].role, Role::System);
        assert_eq!(reconstructed[1].role, Role::User);
        assert_eq!(reconstructed[2].role, Role::Assistant);
        assert_eq!(reconstructed[3].role, Role::Assistant);
        assert_eq!(reconstructed[4].role, Role::Tool);
        assert_eq!(reconstructed[5].role, Role::User);

        let steer_content = reconstructed[5].content.as_deref().unwrap();
        assert!(steer_content.contains("### [УКАЗАНИЕ ПОЛЬЗОВАТЕЛЯ / USER STEER]"));
        assert!(steer_content.contains("Поправь баг в agent.rs вместо main.rs"));
        assert!(!steer_content.contains("Примите это указание во внимание"));
    }
    #[tokio::test]
    async fn test_steer_sqlite_persistence_and_context_reconstruction() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_steer_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dirs");

        let task_id = "CVH-STEER-1";

        // 1. Append a steer activity item to SQLite
        crate::commands::dialogs::append_task_dialog_activity(
            &base,
            task_id,
            "steer",
            "Сфокусируйся на оптимизации компилятора",
            None,
        )
        .await
        .expect("append steer to sqlite");

        // 2. Load dialog items and verify item_type == "steer"
        let items = crate::db::load_dialog_items(&base, task_id).expect("load dialog items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_type, "steer");
        assert_eq!(items[0].content.as_deref(), Some("Сфокусируйся на оптимизации компилятора"));

        // 3. Reconstruct messages from dialog
        let reconstructed = reconstruct_messages_from_dialog(
            "SYSTEM PROMPT",
            "INITIAL OBJECTIVE",
            &items,
        );

        assert_eq!(reconstructed.len(), 3);
        assert_eq!(reconstructed[0].role, Role::System);
        assert_eq!(reconstructed[1].role, Role::User);
        assert_eq!(reconstructed[2].role, Role::User);

        let steer_text = reconstructed[2].content.as_deref().unwrap();
        assert!(steer_text.contains("### [УКАЗАНИЕ ПОЛЬЗОВАТЕЛЯ / USER STEER]"));
        assert!(steer_text.contains("Сфокусируйся на оптимизации компилятора"));
        assert!(!steer_text.contains("Примите это указание во внимание"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_system_prompt_tool_leak_prevention() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_tool_leak_{}", uuid::Uuid::new_v4()));
        let dev_dir = base.join(".cvh").join("agents").join("developer");
        tokio::fs::create_dir_all(&dev_dir).await.expect("test dirs");
        tokio::fs::write(
            dev_dir.join("persona.md"),
            "---\nname: developer\ntools: [read_file, write_file, edit_file, advance_task]\n---\nYou are a developer.",
        )
        .await
        .expect("persona seed");

        // Case 1: Only advance_task is allowed (user turned off all other tools)
        let tools_advance_only = vec!["advance_task".to_string()];
        let prompt = compile_primed_system_prompt_with_tools(&base, "developer", Some(&tools_advance_only))
            .await
            .expect("compile prompt");

        assert!(prompt.contains("tools: [advance_task]"), "frontmatter must contain only allowed tools");
        assert!(!prompt.contains("read_file"), "disabled read_file must NOT leak in prompt");
        assert!(!prompt.contains("write_file"), "disabled write_file must NOT leak in prompt");
        assert!(!prompt.contains("Scoped Tool Execution Mandate"), "mandate for fs tools must NOT appear");
        assert!(!prompt.contains("Subagent Delegation Mandate"), "mandate for subagent must NOT appear");

        // Case 2: Only read_file and advance_task
        let tools_fs = vec!["read_file".to_string(), "advance_task".to_string()];
        let prompt_fs = compile_primed_system_prompt_with_tools(&base, "developer", Some(&tools_fs))
            .await
            .expect("compile prompt");

        assert!(prompt_fs.contains("tools: [read_file, advance_task]"));
        assert!(prompt_fs.contains("Scoped Tool Execution Mandate"));
        assert!(!prompt_fs.contains("Subagent Delegation Mandate"));
        assert!(!prompt_fs.contains("`write_file`"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

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
    async fn test_tool_list_tasks() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_list_tasks_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dirs");
        crate::db::init_db(&base).expect("init db");

        let task1 = Task {
            id: "CVH-101".to_string(),
            title: "Optimize compiler sensor".to_string(),
            description: "Deep compiler optimizations".to_string(),
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
            handover_summary: Some("Compiler partially optimized".to_string()),
            cover_image: None,
            attachments: None,
            todos: None,
            priority: Some("high".to_string()),
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 1000,
            updated_at: 2000,
        };

        let task2 = Task {
            id: "CVH-102".to_string(),
            title: "Audit security invariants".to_string(),
            description: "Review token leaks".to_string(),
            stage_id: "review".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: Some("APPROVED".to_string()),
            custom_flow: None,
            status: Some("idle".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: Some("Security invariants verified".to_string()),
            cover_image: None,
            attachments: None,
            todos: None,
            priority: Some("medium".to_string()),
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 1000,
            updated_at: 3000,
        };

        crate::db::db_upsert_task(&base, &task1).expect("upsert task1");
        crate::db::db_upsert_task(&base, &task2).expect("upsert task2");

        let policy = ToolPolicyConfig::default();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&abort_flag),
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

        // 1. List all tasks (default limit, sorted newest updated_at first)
        let res_str = execute_tool(&ctx, "list_tasks", &serde_json::json!({})).await.expect("list_tasks");
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&res_str).expect("parse json");
        assert_eq!(parsed.len(), 2);
        // CVH-102 has updated_at 3000, CVH-101 has 2000 -> CVH-102 should be first
        assert_eq!(parsed[0]["id"], "CVH-102");
        assert_eq!(parsed[0]["stage_id"], "review");
        assert_eq!(parsed[0]["status"], "idle");
        assert_eq!(parsed[0]["role"], "reviewer");
        assert_eq!(parsed[1]["id"], "CVH-101");
        assert_eq!(parsed[1]["stage_id"], "dev");
        assert_eq!(parsed[1]["role"], "developer");

        // 2. Filter by stage_id
        let res_dev = execute_tool(&ctx, "list_tasks", &serde_json::json!({ "stage_id": "dev" })).await.expect("list_tasks stage");
        let parsed_dev: Vec<serde_json::Value> = serde_json::from_str(&res_dev).expect("parse json");
        assert_eq!(parsed_dev.len(), 1);
        assert_eq!(parsed_dev[0]["id"], "CVH-101");

        // 3. Filter by query
        let res_q = execute_tool(&ctx, "list_tasks", &serde_json::json!({ "query": "security" })).await.expect("list_tasks query");
        let parsed_q: Vec<serde_json::Value> = serde_json::from_str(&res_q).expect("parse json");
        assert_eq!(parsed_q.len(), 1);
        assert_eq!(parsed_q[0]["id"], "CVH-102");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_tool_get_task_history() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_task_hist_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dirs");
        crate::db::init_db(&base).expect("init db");

        let task = Task {
            id: "CVH-201".to_string(),
            title: "Implement task inspection tools".to_string(),
            description: "Add list_tasks, get_task_history, and load_rule tools for agents.".to_string(),
            stage_id: "review".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: Some("APPROVED".to_string()),
            custom_flow: None,
            status: Some("idle".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: Some("Tools implemented and verified with sensors.".to_string()),
            cover_image: None,
            attachments: None,
            todos: None,
            priority: Some("high".to_string()),
            tags: None,
            current_tokens: None,
            skip_worktree: None,
            created_at: 1000,
            updated_at: 2000,
        };
        crate::db::db_upsert_task(&base, &task).expect("upsert task");

        let dialog_items = vec![
            ActivityItem {
                id: "item-1".to_string(),
                item_type: "steer".to_string(),
                timestamp: "10:00:00".to_string(),
                content: Some("Follow AGENTS.md invariants strictly.".to_string()),
                tool: None,
                duration_ms: None,
                images: None,
            },
            ActivityItem {
                id: "item-2".to_string(),
                item_type: "tool".to_string(),
                timestamp: "10:00:05".to_string(),
                content: None,
                tool: Some(serde_json::json!({
                    "id": "tc-1",
                    "toolName": "read_file",
                    "params": { "file_path": "src-tauri/src/agent.rs" },
                    "status": "success",
                    "durationMs": 42.0,
                    "stdout": "pub fn get_standard_tools() -> Vec<ToolDefinition> { ... }"
                })),
                duration_ms: Some(42.0),
                images: None,
            },
            ActivityItem {
                id: "item-3".to_string(),
                item_type: "tool".to_string(),
                timestamp: "10:00:10".to_string(),
                content: None,
                tool: Some(serde_json::json!({
                    "id": "tc-2",
                    "toolName": "execute_command",
                    "params": { "command": "cargo check" },
                    "status": "failed",
                    "durationMs": 150.0,
                    "stderr": "error[E0425]: cannot find value 'load_rule'"
                })),
                duration_ms: Some(150.0),
                images: None,
            },
            ActivityItem {
                id: "item-4".to_string(),
                item_type: "output".to_string(),
                timestamp: "10:00:15".to_string(),
                content: Some("Fixed load_rule and verified check successfully.".to_string()),
                tool: None,
                duration_ms: None,
                images: None,
            },
        ];
        crate::db::append_dialog_items(&base, "CVH-201", &dialog_items).expect("append dialog");

        let policy = ToolPolicyConfig::default();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("review"),
            role: Some("reviewer"),
            abort_handle: Arc::clone(&abort_flag),
            tool_policy: &policy,
            task_id: Some("CVH-201"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        let report = execute_tool(
            &ctx,
            "get_task_history",
            &serde_json::json!({ "task_id": "CVH-201", "include_tool_outputs": true }),
        )
        .await
        .expect("get_task_history");
        assert!(report.contains("# Task History: CVH-201 - Implement task inspection tools (Этап: review, Статус: idle, Роль: reviewer)"));
        assert!(report.contains("## Initial Objective:\nAdd list_tasks, get_task_history, and load_rule tools for agents."));
        assert!(report.contains("## Execution Steps:"));
        assert!(report.contains("Шаг 1 [User Steer / System]: Follow AGENTS.md invariants strictly."));
        assert!(report.contains(r#"Шаг 2 [Tool read_file]: params: {"file_path":"src-tauri/src/agent.rs"}, status: success, duration: 42ms, error: none."#));
        assert!(report.contains("Output: pub fn get_standard_tools()"));
        assert!(report.contains(r#"Шаг 3 [Tool execute_command]: params: {"command":"cargo check"}, status: failed, duration: 150ms, error: error[E0425]: cannot find value 'load_rule'."#));
        assert!(report.contains("## Handover & Terminal Response:"));
        assert!(report.contains("- Handover Summary: Tools implemented and verified with sensors."));
        assert!(report.contains("- Final Output / Verdict: [APPROVED] Fixed load_rule and verified check successfully."));
        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_tool_load_rule() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_load_rule_{}", uuid::Uuid::new_v4()));
        let dev_rules = base.join(".cvh").join("agents").join("developer").join("rules");
        let shared_rules = base.join(".cvh").join("rules");
        tokio::fs::create_dir_all(&dev_rules).await.expect("dev rules dir");
        tokio::fs::create_dir_all(&shared_rules).await.expect("shared rules dir");

        tokio::fs::write(
            dev_rules.join("rust.md"),
            "# Rust Guidelines
Rule: Never use naked unwrap.",
        )
        .await
        .expect("write rust rule");

        tokio::fs::write(
            shared_rules.join("architecture.md"),
            "# Architecture Invariants
Rule: Strictly isolated worktrees.",
        )
        .await
        .expect("write arch rule");

        // 1. Check get_available_rules collects both
        let available = get_available_rules(&base, Some("developer"));
        assert!(available.contains(&"rust".to_string()));
        assert!(available.contains(&"architecture".to_string()));

        // 2. Check get_load_rule_tool contains enum
        let tool_def = get_load_rule_tool(&base, Some("developer"));
        let enum_vals = tool_def.function.parameters["properties"]["rule"]["enum"].as_array().expect("enum array");
        let enum_strs: Vec<&str> = enum_vals.iter().filter_map(|v| v.as_str()).collect();
        assert!(enum_strs.contains(&"rust"));
        assert!(enum_strs.contains(&"architecture"));

        let policy = ToolPolicyConfig::default();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let ctx = ToolContext {
            repo_root: &base,
            worktree_dir: &base,
            stage_id: Some("dev"),
            role: Some("developer"),
            abort_handle: Arc::clone(&abort_flag),
            tool_policy: &policy,
            task_id: Some("CVH-301"),
            agent_allowed_tools: None,
            app_handle: None,
            base_url: None,
            api_key: None,
            model: None,
            temperature: None,
            reasoning_effort: None,
            call_id: None,
        };

        // 3. Load role-specific rule
        let rust_content = execute_tool(&ctx, "load_rule", &serde_json::json!({ "rule": "rust" }))
            .await
            .expect("load rust rule");
        assert!(rust_content.contains("Never use naked unwrap"));

        // 4. Load shared rule
        let arch_content = execute_tool(&ctx, "load_rule", &serde_json::json!({ "rule": "architecture" }))
            .await
            .expect("load arch rule");
        assert!(arch_content.contains("Strictly isolated worktrees"));

        // 5. Non-existent rule returns error
        let err = execute_tool(&ctx, "load_rule", &serde_json::json!({ "rule": "non_existent" }))
            .await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("not found"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_read_file_offset_out_of_range() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_read_oob_{}", uuid::Uuid::new_v4()));
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

        // Write a short file with 3 lines
        let file_path = base.join("short.txt");
        tokio::fs::write(&file_path, "line 1\nline 2\nline 3\n").await.expect("write");

        // 1. Offset beyond line count (e.g. offset = 250, lines = 3)
        // Must NOT panic, and must return informative string
        let res = execute_tool(&tool_ctx, "read_file", &serde_json::json!({
            "file_path": "short.txt",
            "offset": 250,
            "limit": 10
        }))
        .await
        .expect("read_file beyond length must succeed without panic");

        assert_eq!(res, "(File is empty or offset out of range)\n");

        // 2. Empty file
        let empty_path = base.join("empty.txt");
        tokio::fs::write(&empty_path, "").await.expect("write empty");
        let res_empty = execute_tool(&tool_ctx, "read_file", &serde_json::json!({
            "file_path": "empty.txt",
            "offset": 1
        }))
        .await
        .expect("read_file empty must succeed without panic");

        assert_eq!(res_empty, "(File is empty or offset out of range)\n");

        // 3. Normal read
        let res_normal = execute_tool(&tool_ctx, "read_file", &serde_json::json!({
            "file_path": "short.txt",
            "offset": 2,
            "limit": 2
        }))
        .await
        .expect("read normal");

        assert!(res_normal.contains("2: line 2\n3: line 3\n"));

        let _ = tokio::fs::remove_dir_all(&base).await;
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

    #[test]
    fn test_thinking_parser_ignores_backtick_tags() {
        let mut parser = ThinkingStreamParser::new();
        let input = "Here is an explanation of `<thinking>` and `</thinking>` tags in markdown.";
        let outputs = parser.push(input);
        let mut flushed = parser.flush();
        let mut all = outputs;
        all.append(&mut flushed);

        assert!(!all.is_empty(), "Parser must produce output");
        for (is_thought, text) in &all {
            assert!(
                !is_thought,
                "Backtick-wrapped tags must NEVER be classified as thought! Found is_thought=true for: '{}'",
                text
            );
        }

        let full_text: String = all.into_iter().map(|(_, t)| t).collect();
        assert_eq!(full_text, input, "Full output text must match input without loss");
    }

    #[test]
    fn test_thinking_parser_actual_reasoning() {
        let mut parser = ThinkingStreamParser::new();
        let input = "<thinking>This is actual reasoning</thinking>Final answer";
        let outputs = parser.push(input);
        let mut flushed = parser.flush();
        let mut all = outputs;
        all.append(&mut flushed);

        let thoughts: Vec<String> = all.iter().filter(|(th, _)| *th).map(|(_, t)| t.clone()).collect();
        let regular: Vec<String> = all.iter().filter(|(th, _)| !*th).map(|(_, t)| t.clone()).collect();

        let joined_thought = thoughts.join("");
        let joined_regular = regular.join("");

        assert_eq!(joined_thought, "This is actual reasoning");
        assert_eq!(joined_regular, "Final answer");
    }

    #[test]
    fn test_thinking_parser_think_tag_and_streamed_chunks() {
        // Test chunk-by-chunk streaming with backticks
        let mut parser = ThinkingStreamParser::new();
        let chunks = vec![
            "Here is some text with `",
            "<think>",
            "` and then real reasoning: 
",
            "<think>",
            "Deep thoughts here",
            "</think>",
            " Done!",
        ];

        let mut all = Vec::new();
        for chunk in chunks {
            all.extend(parser.push(chunk));
        }
        all.extend(parser.flush());

        let thoughts: Vec<String> = all.iter().filter(|(th, _)| *th).map(|(_, t)| t.clone()).collect();
        let regular: Vec<String> = all.iter().filter(|(th, _)| !*th).map(|(_, t)| t.clone()).collect();

        assert_eq!(thoughts.join(""), "Deep thoughts here");
        let regular_text = regular.join("");
        assert!(regular_text.contains("`<think>`"));
        assert!(regular_text.contains("Done!"));
    }


