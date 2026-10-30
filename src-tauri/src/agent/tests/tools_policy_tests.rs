use super::*;
use crate::config::ToolPolicyConfig;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

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
        restricted_policy.stage_overrides.entry("dev".to_string()).or_default().insert("web_search".to_string(), false);
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
