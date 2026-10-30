use super::*;
use crate::config::ToolPolicyConfig;
use std::sync::atomic::Ordering;
use std::sync::Arc;

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
            "stageOverrides",
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


    #[tokio::test]
    async fn test_role_specific_isolated_rules() {
        println!("current_dir: {:?}", std::env::current_dir());
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.parent().unwrap();
        println!("repo_root: {:?}", repo_root);

        let planner_rules = get_available_rules(repo_root, Some("planner"));
        println!("planner_rules: {:?}", planner_rules);
        assert_eq!(
            planner_rules,
            vec!["anti-sycophancy", "architecture", "backend_map", "be-ambitious", "decomposition", "frontend_map", "scope-hygiene"]
        );

        let dev_rules = get_available_rules(repo_root, Some("developer"));
        assert_eq!(
            dev_rules,
            vec!["anti-sycophancy", "backend", "backend_map", "frontend", "frontend_map", "ponytail", "rules-sync", "sensors"]
        );

        let reviewer_rules = get_available_rules(repo_root, Some("reviewer"));
        assert_eq!(
            reviewer_rules,
            vec!["anti-sycophancy", "backend-invariants", "backend_map", "frontend-invariants", "frontend_map", "review-checklist", "verdict-routing"]
        );
        assert!(!reviewer_rules.contains(&"boards".to_string()), "Reviewer must NOT have boards rule");
        assert!(!reviewer_rules.contains(&"retro".to_string()), "Reviewer must NOT have retro rule");

        let retro_rules = get_available_rules(repo_root, Some("retro"));
        assert_eq!(
            retro_rules,
            vec!["anti-sycophancy", "backend_map", "frontend_map", "reflection-methodology", "rule-evolution"]
        );

        let cvh_dev_rules = get_available_rules(repo_root, Some("cvh-dev"));
        assert_eq!(
            cvh_dev_rules,
            vec![
                "agents",
                "anti-sycophancy",
                "backend_map",
                "boards",
                "frontend_map",
                "hooks",
                "pipeline-setup",
                "role-developer",
                "role-planner",
                "role-retro",
                "role-reviewer",
                "tools",
            ]
        );
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
            task_attachments: None,
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
        assert!(result.to_lowercase().contains("review") || result.contains("ревью"), "got: {}", result);

        // 2. Only 1 request was sent to the LLM (zero extra turns)
        assert_eq!(request_counter.load(Ordering::SeqCst), 1, "Expected exactly 1 request to LLM, no extra turns");

        // 3. Emitted events must contain Finished with final_response == result and Output
        {
            let captured = events.lock().unwrap();
            let output_event = captured.iter().find(|e| matches!(e, AgentEvent::Output(ref out) if out == "Feature implemented successfully"));
            assert!(output_event.is_some(), "AgentEvent::Output must be emitted for advance_task summary");

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
            task_attachments: None,
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
            task_attachments: None,
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
            task_attachments: None,
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


