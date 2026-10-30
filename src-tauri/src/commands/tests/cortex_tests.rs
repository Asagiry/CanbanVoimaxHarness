use super::*;
use crate::agent::CustomToolConfig;
use std::path::PathBuf;


    #[test]
    fn test_persona_role_validation() {
        for ok in ["developer", "a", "0abc", "my-role-1", "r2-d2"] {
            assert!(is_valid_persona_role(ok), "expected valid: {}", ok);
        }
        for bad in [
            "",
            "-abc",
            "Abc",
            "my_role",
            "my role",
            "роль",
            "a/b",
            "A",
            "dev!",
        ] {
            assert!(!is_valid_persona_role(bad), "expected invalid: {}", bad);
        }
    }


    #[test]
    fn test_persona_placeholder_template_en() {
        let t = persona_placeholder_template("reviewer", "ShieldCheck", "#a855f7");
        assert!(t.contains("name: reviewer"));
        assert!(t.contains("icon: ShieldCheck"));
        assert!(t.contains("color: #a855f7"));
        assert!(t.contains("# Role Persona: reviewer"));
        assert!(t.contains("Identity & Scope:"));
        assert!(t.contains("Guidelines:"));
        // Assert absence of Cyrillic characters (100% English invariant)
        assert!(!t.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)));
    }


    #[test]
    fn test_agent_persona_info_camel_case() {
        let info = AgentPersonaInfo {
            role: "developer".to_string(),
            has_prompt: true,
            is_builtin: false,
            persona_file: "persona.md".to_string(),
            rules: vec!["r1.md".to_string()],
            hooks: vec![],
            icon: Some("Code2".to_string()),
            color: Some("#0284c7".to_string()),
            context_window: Some(400000),
        };
        let v = serde_json::to_value(&info).expect("serialize");
        assert_eq!(v.get("role").and_then(|x| x.as_str()), Some("developer"));
        assert_eq!(v.get("hasPrompt").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("isBuiltin").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(v.get("personaFile").and_then(|x| x.as_str()), Some("persona.md"));
        assert_eq!(v.get("icon").and_then(|x| x.as_str()), Some("Code2"));
        assert_eq!(v.get("color").and_then(|x| x.as_str()), Some("#0284c7"));
        assert_eq!(v.get("contextWindow").and_then(|x| x.as_u64()), Some(400000));
        assert!(v.get("has_prompt").is_none());
        assert!(v.get("is_builtin").is_none());

        let builtin = AgentPersonaInfo {
            role: "cvh-dev".to_string(),
            has_prompt: true,
            is_builtin: true,
            persona_file: "persona.md".to_string(),
            rules: vec!["boards.md".to_string()],
            hooks: vec![],
            icon: Some("Cpu".to_string()),
            color: Some("#6366f1".to_string()),
            context_window: Some(128000),
        };
        let vb = serde_json::to_value(&builtin).expect("serialize");
        assert_eq!(vb.get("isBuiltin").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(vb.get("icon").and_then(|x| x.as_str()), Some("Cpu"));
        assert_eq!(vb.get("color").and_then(|x| x.as_str()), Some("#6366f1"));
    }


    #[tokio::test]
    async fn test_agent_colors_palette_and_forbidden_invariants() {
        // Серый (#64748b) и зеленый (#10b981) не должны быть в AGENT_PALETTE
        for color in AGENT_PALETTE {
            assert_ne!(*color, "#64748b", "Backlog gray is strictly forbidden in agent palette");
            assert_ne!(*color, "#6b7280", "Gray is strictly forbidden in agent palette");
            assert_ne!(*color, "#10b981", "Ready green is strictly forbidden in agent palette");
            assert_ne!(*color, "#22c55e", "Green is strictly forbidden in agent palette");
        }

        let dev_col = deterministic_color_for_role("developer");
        assert!(!FORBIDDEN_AGENT_COLORS.contains(&dev_col.as_str()));

        let text_with_color = "---\nname: foo\ncolor: #f59e0b\n---\n# Body";
        assert_eq!(extract_persona_color_from_text(text_with_color), Some("#f59e0b".to_string()));

        let updated = insert_or_update_frontmatter_color("# Title", "#a855f7");
        assert!(updated.contains("color: #a855f7"));
        assert_eq!(extract_persona_color_from_text(&updated), Some("#a855f7".to_string()));
    }


    #[tokio::test]
    async fn test_agent_svg_icons_generation_and_persistence() {
        assert_eq!(default_column_icon("backlog"), "Inbox");
        assert_eq!(default_column_icon("ready"), "CheckCircle2");
        assert_eq!(default_column_icon("other"), "Layers");

        let text_with_icon = "---\nname: foo\nicon: Rocket\n---\n# Body";
        assert_eq!(extract_persona_icon_from_text(text_with_icon), Some("Rocket".to_string()));

        let text_no_icon = "# Just Title";
        let updated = insert_or_update_frontmatter_icon(text_no_icon, "Flame");
        assert!(updated.contains("icon: Flame"));
        assert_eq!(extract_persona_icon_from_text(&updated), Some("Flame".to_string()));

        let text_update_icon = "---\nname: bar\nicon: Zap\n---\n# Bar";
        let updated2 = insert_or_update_frontmatter_icon(text_update_icon, "Radar");
        assert!(updated2.contains("icon: Radar"));
        assert!(!updated2.contains("icon: Zap"));
        assert_eq!(extract_persona_icon_from_text(&updated2), Some("Radar".to_string()));

        let base = PathBuf::from("target").join(format!("cvh_persona_icon_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        // Создаем персону через create_agent_persona_file — она должна сразу получить иконку
        let rel = create_agent_persona_file(&base, "bot-agent").await.expect("create bot-agent");
        let content = tokio::fs::read_to_string(base.join(&rel)).await.expect("read created");
        let icon_opt = extract_persona_icon_from_text(&content);
        assert!(icon_opt.is_some(), "Созданная персона должна содержать иконку в frontmatter");
        let icon = icon_opt.unwrap();
        assert!(AGENT_ICONS.contains(&icon.as_str()), "Иконка должна быть из канонического пака");

        // Проверяем листинг персон
        let listed = list_agent_personas_in(&base).await;
        let bot_info = listed.iter().find(|p| p.role == "bot-agent").expect("bot-agent in list");
        assert_eq!(bot_info.icon.as_deref(), Some(icon.as_str()));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_agent_setting_icon_color_priority_and_sync() {
        let base = PathBuf::from("target").join(format!("cvh_setting_sync_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let agents_dir = base.join(".cvh").join("agents");
        let test_role_dir = agents_dir.join("test-sync-agent");
        tokio::fs::create_dir_all(&test_role_dir).await.expect("role dir");

        // 1. Initial persona.md with icon "Wrench" and color "#0284c7"
        let persona_content = "---\nname: test-sync-agent\nicon: Wrench\ncolor: #0284c7\n---\n# Test Agent";
        tokio::fs::write(test_role_dir.join("persona.md"), persona_content)
            .await
            .expect("write persona.md");

        // 2. Initial setting with overriding icon "Flame" and color "#f97316"
        let setting_content = r##"{
  "model": "gemini-3.8-flash-high",
  "context_window": 200000,
  "icon": "Flame",
  "color": "#f97316"
}"##;
        tokio::fs::write(test_role_dir.join("setting"), setting_content)
            .await
            .expect("write setting");

        // 3. list_agent_personas_in must prioritize setting over persona.md
        let listed = list_agent_personas_in(&base).await;
        let info = listed.iter().find(|p| p.role == "test-sync-agent").expect("found agent");
        assert_eq!(info.icon.as_deref(), Some("Flame"), "setting icon must override persona.md");
        assert_eq!(info.color.as_deref(), Some("#f97316"), "setting color must override persona.md");

        // 4. save_persona_icon must update persona.md AND sync into setting file
        save_persona_icon(&test_role_dir, "Sparkles").await.expect("save icon");
        let updated_persona = tokio::fs::read_to_string(test_role_dir.join("persona.md"))
            .await
            .expect("read persona");
        assert_eq!(extract_persona_icon_from_text(&updated_persona), Some("Sparkles".to_string()));
        let updated_setting = tokio::fs::read_to_string(test_role_dir.join("setting"))
            .await
            .expect("read setting");
        assert_eq!(extract_persona_icon_from_json(&updated_setting), Some("Sparkles".to_string()));

        // 5. save_persona_color must update persona.md AND sync into setting file
        save_persona_color(&test_role_dir, "#a855f7").await.expect("save color");
        let updated_persona2 = tokio::fs::read_to_string(test_role_dir.join("persona.md"))
            .await
            .expect("read persona");
        assert_eq!(extract_persona_color_from_text(&updated_persona2), Some("#a855f7".to_string()));
        let updated_setting2 = tokio::fs::read_to_string(test_role_dir.join("setting"))
            .await
            .expect("read setting");
        assert_eq!(extract_persona_color_from_json(&updated_setting2), Some("#a855f7".to_string()));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_create_persona_placeholder_in_tmp_dir() {
        let base =
            PathBuf::from("target").join(format!("cvh_persona_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        // invalid role → Err
        let bad = create_agent_persona_file(&base, "Bad_Name").await;
        assert!(bad.is_err());

        // valid create → directory `.cvh/agents/<role>/persona.md`
        let rel = create_agent_persona_file(&base, "my-role")
            .await
            .expect("create failed");
        assert_eq!(rel, ".cvh/agents/my-role/persona.md");
        let agents_dir = base.join(".cvh").join("agents");
        let role_dir = agents_dir.join("my-role");
        let content = tokio::fs::read_to_string(role_dir.join("persona.md"))
            .await
            .expect("read created file");
        assert!(content.contains("# Role Persona: my-role"));
        assert!(role_dir.join("rules").is_dir(), "rules/ must exist");
        assert!(role_dir.join("hooks").is_dir(), "hooks/ must exist");
        let setting = tokio::fs::read_to_string(role_dir.join("setting"))
            .await
            .expect("read setting");
        let parsed_setting: crate::config::AgentSetting =
            serde_json::from_str(&setting).expect("parse setting");
        assert_eq!(parsed_setting.model.as_deref(), Some("gemini-3.8-flash-high"));
        assert_eq!(parsed_setting.context_window, Some(200000));
        assert_eq!(parsed_setting.temperature, Some(0.2));
        assert_eq!(parsed_setting.max_tokens, Some(8192));
        assert!(parsed_setting.icon.is_some(), "created setting must have icon");
        assert!(parsed_setting.color.is_some(), "created setting must have color");

        // duplicate → Err mentioning существование
        let dup = create_agent_persona_file(&base, "my-role").await;
        assert!(dup.is_err());
        assert!(dup.unwrap_err().contains("уже существует"));

        // creating another role in directory
        create_agent_persona_file(&base, "aaa").await.expect("create aaa");
        tokio::fs::write(agents_dir.join("notes.txt"), "ignore me")
            .await
            .expect("seed txt");
        let listed = list_agent_personas_in(&base).await;
        let roles: Vec<String> = listed.iter().map(|p| p.role.clone()).collect();
        assert_eq!(roles, vec!["aaa".to_string(), "cvh-dev".to_string(), "my-role".to_string()]);
        assert!(listed.iter().all(|p| p.has_prompt));
        let cvh_dev_info = listed.iter().find(|p| p.role == "cvh-dev").unwrap();
        assert!(cvh_dev_info.is_builtin);

        // missing folder → still always contains builtin cvh-dev
        let empty_base =
            PathBuf::from("target").join(format!("cvh_persona_empty_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&empty_base).await.expect("empty dir");
        let empty = list_agent_personas_in(&empty_base).await;
        assert_eq!(empty.len(), 1);
        assert_eq!(empty[0].role, "cvh-dev");
        assert!(empty[0].is_builtin);
        assert!(empty[0].has_prompt);

        let _ = tokio::fs::remove_dir_all(&base).await;
        let _ = tokio::fs::remove_dir_all(&empty_base).await;
    }


    #[tokio::test]
    async fn test_list_agent_personas_auto_scaffolds_cvh_dev() {
        let base = PathBuf::from("target").join(format!("cvh_persona_autoseed_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let listed = list_agent_personas_in(&base).await;
        let cvh_dev = listed.iter().find(|p| p.role == "cvh-dev").expect("cvh-dev must exist");
        assert!(cvh_dev.is_builtin);
        assert!(cvh_dev.has_prompt);
        assert!(cvh_dev.rules.contains(&"boards.md".to_string()));
        assert!(cvh_dev.rules.contains(&"agents.md".to_string()));
        assert!(cvh_dev.rules.contains(&"hooks.md".to_string()));

        // Check on disk files
        let dev_dir = base.join(".cvh").join("agents").join("cvh-dev");
        assert!(dev_dir.join("persona.md").is_file());
        assert!(dev_dir.join("setting").is_file());
        assert!(dev_dir.join("rules").join("boards.md").is_file());

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_list_personas_dir_only() {
        // Directory-only structure: subdirectories with persona.md/agent.md are listed.
        // Flat *.md files in agents/ are strictly ignored. cvh-dev is always included.
        let base =
            PathBuf::from("target").join(format!("cvh_persona_mix_{}", uuid::Uuid::new_v4()));
        let agents = base.join(".cvh").join("agents");
        tokio::fs::create_dir_all(&agents).await.expect("test dir");
        // Flat files should be ignored
        tokio::fs::write(agents.join("ignored.md"), "# flat ignored
")
            .await
            .expect("seed flat");
        // Directories with persona.md or agent.md
        let a_dir = agents.join("a-role");
        tokio::fs::create_dir_all(&a_dir).await.expect("a-role dir");
        tokio::fs::write(a_dir.join("persona.md"), "# a-role
")
            .await
            .expect("seed a-role persona.md");

        let b_dir = agents.join("b-role");
        tokio::fs::create_dir_all(&b_dir).await.expect("b-role dir");
        tokio::fs::write(b_dir.join("agent.md"), "# b-role
")
            .await
            .expect("seed b-role agent.md");

        tokio::fs::write(agents.join("junk.txt"), "ignore").await.expect("seed txt");

        let listed = list_agent_personas_in(&base).await;
        let roles: Vec<String> = listed.iter().map(|p| p.role.clone()).collect();
        assert_eq!(roles, vec!["a-role".to_string(), "b-role".to_string(), "cvh-dev".to_string()]);
        assert!(listed.iter().all(|p| p.has_prompt));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_delete_agent_persona_protection() {
        let base =
            PathBuf::from("target").join(format!("cvh_persona_del_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        // Deleting cvh-dev must be strictly forbidden
        let err = delete_agent_persona_file(&base, "cvh-dev").await.unwrap_err();
        assert!(err.contains("Запрещено удалять встроенную персону 'cvh-dev'"));

        // Creating and then deleting a custom persona succeeds
        create_agent_persona_file(&base, "temp-agent").await.expect("create temp-agent");
        let agents_dir = base.join(".cvh").join("agents");
        assert!(agents_dir.join("temp-agent").join("persona.md").is_file());

        delete_agent_persona_file(&base, "temp-agent").await.expect("delete temp-agent");
        assert!(!agents_dir.join("temp-agent").exists());

        // Deleting non-existent persona returns error
        let missing_err = delete_agent_persona_file(&base, "non-existent").await.unwrap_err();
        assert!(missing_err.contains("не найдена"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_rename_agent_persona_and_boards_sync() {
        let base =
            PathBuf::from("target").join(format!("cvh_persona_ren_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        // Renaming cvh-dev must be strictly forbidden
        let err = rename_agent_persona_file(&base, "cvh-dev", "new-dev").await.unwrap_err();
        assert!(err.contains("Запрещено переименовывать встроенную персону 'cvh-dev'"));

        // Invalid name
        let bad_err = rename_agent_persona_file(&base, "agent-a", "Bad_Name!").await.unwrap_err();
        assert!(bad_err.contains("Недопустимое имя роли"));

        // Non-existent persona
        let notfound_err = rename_agent_persona_file(&base, "non-existent", "new-role").await.unwrap_err();
        assert!(notfound_err.contains("не найдена"));

        // Create persona
        create_agent_persona_file(&base, "agent-a").await.expect("create agent-a");
        let agents_dir = base.join(".cvh").join("agents");
        assert!(agents_dir.join("agent-a").join("persona.md").is_file());

        // Create board and pipeline referencing agent-a
        let backlog_col: crate::config::PipelineColumn = serde_json::from_value(serde_json::json!({
            "id": "backlog",
            "title": "Backlog"
        })).unwrap();
        let dev_col: crate::config::PipelineColumn = serde_json::from_value(serde_json::json!({
            "id": "dev",
            "title": "Dev",
            "persona": "agent-a"
        })).unwrap();
        let ready_col: crate::config::PipelineColumn = serde_json::from_value(serde_json::json!({
            "id": "ready",
            "title": "Ready",
            "terminal": true
        })).unwrap();

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "test-board".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "test-board".to_string(),
                name: "Test Board".to_string(),
                columns: vec![backlog_col, dev_col, ready_col],
            }],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Rename agent-a to agent-b
        rename_agent_persona_file(&base, "agent-a", "agent-b").await.expect("rename agent-a to agent-b");
        assert!(!agents_dir.join("agent-a").exists());
        assert!(agents_dir.join("agent-b").join("persona.md").is_file());

        // Check boards.json updated
        let loaded_boards = crate::config::load_boards_config(&base).await.expect("load boards");
        let dev_col = loaded_boards.boards[0].columns.iter().find(|c| c.id == "dev").unwrap();
        assert_eq!(dev_col.persona.as_deref(), Some("agent-b"));

        // Check pipeline.json updated
        let loaded_pipe = crate::config::load_pipeline_config(&base).await.expect("load pipeline");
        let pipe_dev_col = loaded_pipe.columns.iter().find(|c| c.id == "dev").unwrap();
        assert_eq!(pipe_dev_col.persona.as_deref(), Some("agent-b"));

        // Delete agent-b and verify references reset to None
        delete_agent_persona_file(&base, "agent-b").await.expect("delete agent-b");
        assert!(!agents_dir.join("agent-b").exists());

        let loaded_boards_after_del = crate::config::load_boards_config(&base).await.expect("load boards after del");
        let dev_col_del = loaded_boards_after_del.boards[0].columns.iter().find(|c| c.id == "dev").unwrap();
        assert_eq!(dev_col_del.persona, None);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }



    #[tokio::test]
    async fn test_sync_tool_policies_to_personas_frontmatter() {
        // Verification: persona.md must remain untouched (zero file mutation churn)
        // and tool permissions are resolved purely from ToolPolicyConfig.
        let base = PathBuf::from("target").join(format!("cvh_policy_sync_{}", uuid::Uuid::new_v4()));
        let agent_dir = base.join(".cvh").join("agents").join("developer");
        tokio::fs::create_dir_all(&agent_dir).await.expect("test dir");

        let initial_persona = r#"---
name: developer
description: Developer persona
tools: [read_file]
---
# Developer Instructions
Write code.
"#;
        tokio::fs::write(agent_dir.join("persona.md"), initial_persona)
            .await
            .expect("write persona");

        let mut policy = crate::config::ToolPolicyConfig::default();
        let mut dev_map = std::collections::HashMap::new();
        dev_map.insert("read_file".to_string(), true);
        dev_map.insert("edit_file".to_string(), true);
        dev_map.insert("write_file".to_string(), true);
        dev_map.insert("web_search".to_string(), false);
        policy.stage_overrides.insert("dev".to_string(), dev_map);

        crate::config::sync_tool_policies_to_personas(&base, &policy)
            .await
            .expect("sync policies");

        let post_sync = tokio::fs::read_to_string(agent_dir.join("persona.md"))
            .await
            .expect("read post sync");
        // Must be identical: persona.md is NOT mutated by tool policy saves
        assert_eq!(post_sync, initial_persona);

        // Tool permission checks work directly from policy
        assert!(crate::config::is_tool_allowed(&policy, Some("dev"), "read_file"));
        assert!(crate::config::is_tool_allowed(&policy, Some("dev"), "edit_file"));
        assert!(!crate::config::is_tool_allowed(&policy, Some("dev"), "web_search"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_agent_setting_context_window_strict_resolution() {
        let base = PathBuf::from("target").join(format!("cvh_cw_test_{}", uuid::Uuid::new_v4()));
        let dev_dir = base.join(".cvh").join("agents").join("developer");
        tokio::fs::create_dir_all(&dev_dir).await.expect("create dev dir");
        tokio::fs::write(
            dev_dir.join("setting"),
            r#"{"context_window": 400000, "model": "gemini-3.8-flash-high"}"#,
        )
        .await
        .expect("write dev setting");

        let custom_dir = base.join(".cvh").join("agents").join("architect");
        tokio::fs::create_dir_all(&custom_dir).await.expect("create custom dir");
        tokio::fs::write(
            custom_dir.join("setting"),
            r#"{"contextWindow": 300000}"#,
        )
        .await
        .expect("write custom setting");

        // 1. Resolved directly from agent setting
        let dev_setting = crate::config::load_agent_setting(&base, "developer").await;
        assert_eq!(dev_setting.context_window, Some(400000));

        let custom_setting = crate::config::load_agent_setting(&base, "architect").await;
        assert_eq!(custom_setting.context_window, Some(300000));

        // 2. Fallback to 128_000 if not configured
        let missing_setting = crate::config::load_agent_setting(&base, "non-existent").await;
        assert_eq!(missing_setting.context_window.unwrap_or(128_000), 128_000);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_list_global_rules_in() {
        let base = PathBuf::from("target").join(format!("cvh_global_rules_test_{}", uuid::Uuid::new_v4()));
        let rules_dir = base.join(".cvh").join("rules");
        tokio::fs::create_dir_all(&rules_dir).await.expect("rules dir");

        tokio::fs::write(rules_dir.join("b_rule.md"), "# Rule B").await.expect("write b_rule.md");
        tokio::fs::write(rules_dir.join("a_rule.md"), "# Rule A").await.expect("write a_rule.md");
        tokio::fs::write(rules_dir.join("ignored.txt"), "not markdown").await.expect("write ignored.txt");

        let files = list_global_rules_in(&base).await.expect("list_global_rules_in");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "a_rule.md");
        assert_eq!(files[0].path, ".cvh/rules/a_rule.md");
        assert_eq!(files[0].file_type, "rule");

        assert_eq!(files[1].name, "b_rule.md");
        assert_eq!(files[1].path, ".cvh/rules/b_rule.md");
        assert_eq!(files[1].file_type, "rule");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_delete_cortex_file_security_and_traversal_protection() {
        let base = PathBuf::from("target").join(format!("cvh_del_cortex_test_{}", uuid::Uuid::new_v4()));
        let rules_dir = base.join(".cvh").join("rules");
        let agents_dir = base.join(".cvh").join("agents").join("developer").join("rules");
        tokio::fs::create_dir_all(&rules_dir).await.expect("rules dir");
        tokio::fs::create_dir_all(&agents_dir).await.expect("agents dir");

        let global_rule = rules_dir.join("temp.md");
        tokio::fs::write(&global_rule, "# Temp").await.expect("write temp.md");
        let agent_rule = agents_dir.join("agent_rule.md");
        tokio::fs::write(&agent_rule, "# Agent Rule").await.expect("write agent_rule.md");

        // 1. Deleting outside .cvh/rules/ or .cvh/agents/ is forbidden
        let err1 = delete_cortex_file_at(&base, "src/main.rs").await.unwrap_err();
        assert!(err1.contains("Forbidden"));

        // 2. Directory traversal with .. is strictly forbidden
        let err2 = delete_cortex_file_at(&base, ".cvh/rules/../../secret.txt").await.unwrap_err();
        assert!(err2.contains("traversal"));

        // 3. Deleting non-existent file returns error
        let err3 = delete_cortex_file_at(&base, ".cvh/rules/missing.md").await.unwrap_err();
        assert!(err3.contains("does not exist"));

        // 4. Valid global rule deletion succeeds
        assert!(global_rule.exists());
        delete_cortex_file_at(&base, ".cvh/rules/temp.md").await.expect("delete global rule");
        assert!(!global_rule.exists());

        // 5. Valid agent rule deletion succeeds
        assert!(agent_rule.exists());
        delete_cortex_file_at(&base, ".cvh/agents/developer/rules/agent_rule.md").await.expect("delete agent rule");
        assert!(!agent_rule.exists());

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_custom_tools_save_list_delete_roundtrip() {
        let base =
            PathBuf::from("target").join(format!("cvh_tools_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let params = serde_json::json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"]
        });

        // save b_tool first, then a_tool → list must be sorted by name
        let rel_b = save_custom_tool_file(&base, "b_tool", "B desc", params.clone(), "echo b")
            .await
            .expect("save b_tool");
        assert_eq!(rel_b, ".cvh/tools/b_tool.json");
        let rel_a = save_custom_tool_file(&base, "a_tool", "A desc", params.clone(), "echo a")
            .await
            .expect("save a_tool");
        assert_eq!(rel_a, ".cvh/tools/a_tool.json");

        // pretty JSON on disk parses back to the same config
        let raw = tokio::fs::read_to_string(base.join(".cvh").join("tools").join("a_tool.json"))
            .await
            .expect("read a_tool file");
        let parsed: CustomToolConfig = serde_json::from_str(&raw).expect("parse a_tool");
        assert_eq!(parsed.name, "a_tool");
        assert_eq!(parsed.description, "A desc");
        assert_eq!(parsed.command, "echo a");
        assert_eq!(parsed.parameters, params);

        let listed = list_custom_tools_in(&base).await;
        let names: Vec<String> = listed.iter().map(|t| t.name.clone()).collect();
        assert_eq!(names, vec!["a_tool".to_string(), "b_tool".to_string()]);

        delete_custom_tool_file(&base, "a_tool").await.expect("delete a_tool");
        assert!(!base.join(".cvh").join("tools").join("a_tool.json").exists());
        let listed2 = list_custom_tools_in(&base).await;
        let names2: Vec<String> = listed2.iter().map(|t| t.name.clone()).collect();
        assert_eq!(names2, vec!["b_tool".to_string()]);

        delete_custom_tool_file(&base, "b_tool").await.expect("delete b_tool");
        let listed3 = list_custom_tools_in(&base).await;
        assert!(listed3.is_empty());

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_custom_tool_invalid_name_rejected() {
        let base =
            PathBuf::from("target").join(format!("cvh_tools_badname_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");
        let params = serde_json::json!({"type": "object"});

        for bad in ["", "BadName", "with space", "has-dash", "a/b", "имя", "name!"] {
            let res = save_custom_tool_file(&base, bad, "d", params.clone(), "cmd").await;
            assert!(res.is_err(), "expected invalid name error for '{}'", bad);
            let del = delete_custom_tool_file(&base, bad).await;
            assert!(del.is_err(), "expected delete invalid name error for '{}'", bad);
        }

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_custom_tool_parameters_must_be_object() {
        let base =
            PathBuf::from("target").join(format!("cvh_tools_params_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        for non_obj in [
            serde_json::json!([]),
            serde_json::json!("str"),
            serde_json::json!(42),
            serde_json::Value::Null,
        ] {
            let res =
                save_custom_tool_file(&base, "good_name", "d", non_obj, "cmd").await;
            assert!(res.is_err(), "expected parameters-object error");
        }
        // nothing was written
        assert!(list_custom_tools_in(&base).await.is_empty());

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_custom_tool_delete_missing_errors() {
        let base =
            PathBuf::from("target").join(format!("cvh_tools_missing_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let res = delete_custom_tool_file(&base, "no_such_tool").await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("не найден"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }


    #[tokio::test]
    async fn test_custom_tool_broken_json_ignored() {
        let base =
            PathBuf::from("target").join(format!("cvh_tools_broken_{}", uuid::Uuid::new_v4()));
        let tools_dir = base.join(".cvh").join("tools");
        tokio::fs::create_dir_all(&tools_dir).await.expect("test dir");

        save_custom_tool_file(&base, "good_tool", "ok", serde_json::json!({"type": "object"}), "echo ok")
            .await
            .expect("save good");
        // broken json + non-json file must be silently skipped
        tokio::fs::write(tools_dir.join("broken.json"), "{ not valid json {{{")
            .await
            .expect("seed broken");
        tokio::fs::write(tools_dir.join("notes.txt"), "ignore me")
            .await
            .expect("seed txt");

        let listed = list_custom_tools_in(&base).await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "good_tool");

        // missing folder → []
        let empty_base =
            PathBuf::from("target").join(format!("cvh_tools_empty_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&empty_base).await.expect("empty dir");
        assert!(list_custom_tools_in(&empty_base).await.is_empty());

        let _ = tokio::fs::remove_dir_all(&base).await;
        let _ = tokio::fs::remove_dir_all(&empty_base).await;
    }
