use super::*;
use crate::agent::{CustomToolConfig, Task};
use crate::config::{normalize_pipeline_columns, PipelineColumn};
use std::collections::HashMap;
use std::path::PathBuf;
    fn test_col(id: &str) -> PipelineColumn {
        PipelineColumn {
            id: id.to_string(),
            title: id.to_string(),
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
            is_intake: None,
        }
    }

    fn ids(cols: &[PipelineColumn]) -> Vec<String> {
        cols.iter().map(|c| c.id.clone()).collect()
    }

    #[test]
    fn test_normalize_preserves_custom_column_order() {
        let cols = vec![
            test_col("dev"),
            test_col("ready"),
            test_col("review"),
            test_col("backlog"),
            test_col("qa"),
        ];
        let out = normalize_pipeline_columns(cols).expect("normalize failed");
        assert_eq!(ids(&out), vec!["dev", "ready", "review", "backlog", "qa"]);
        assert_eq!(out[0].is_intake, Some(true));
        assert_eq!(out.last().unwrap().terminal, Some(true));
    }

    #[test]
    fn test_normalize_already_ordered_keeps_order_and_sets_intake_terminal() {
        let cols = vec![
            test_col("backlog"),
            test_col("a"),
            test_col("b"),
            test_col("ready"),
        ];
        let out = normalize_pipeline_columns(cols).expect("normalize failed");
        assert_eq!(ids(&out), vec!["backlog", "a", "b", "ready"]);
        assert_eq!(out[0].is_intake, Some(true));
        assert_eq!(out.last().unwrap().terminal, Some(true));
    }

    #[test]
    fn test_normalize_without_specials_preserves_custom_columns_without_forced_injection() {
        let cols = vec![test_col("a"), test_col("b"), test_col("c")];
        let out = normalize_pipeline_columns(cols).expect("normalize failed");
        assert_eq!(ids(&out), vec!["a", "b", "c"]);
        assert_eq!(out[0].is_intake, Some(true));
        assert_eq!(out.last().unwrap().terminal, Some(true));
    }

    #[test]
    fn test_normalize_empty_returns_canonical() {
        let out = normalize_pipeline_columns(vec![]).expect("normalize failed");
        assert_eq!(ids(&out), vec!["backlog", "dev", "ready"]);
        assert_eq!(out[0].is_intake, Some(true));
        assert_eq!(out.last().unwrap().terminal, Some(true));
    }

    #[test]
    fn test_normalize_duplicate_ids_error() {
        let cols = vec![test_col("dev"), test_col("dev")];
        let err = normalize_pipeline_columns(cols).expect_err("expected duplicate error");
        assert!(err.contains("dev"), "unexpected error text: {}", err);
    }

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


    #[test]
    fn test_next_task_num_never_reuses_ids() {
        // Gap after deletion: len is 2 but max suffix is 3 -> next must be 4,
        // never CVH-003 again (stale dialog history) and never len+1=3.
        let tasks: Vec<Task> = serde_json::from_value(serde_json::json!([
            {"id": "CVH-001", "title": "a", "stageId": "backlog"},
            {"id": "CVH-003", "title": "b", "stageId": "backlog"},
            {"id": "CVH-003.2", "title": "sub", "stageId": "dev", "parentId": "CVH-003", "subtaskIndex": 2}
        ]))
        .expect("deserialize tasks");
        assert_eq!(next_task_num(&tasks), 4);

        // Empty disk -> first id.
        let empty: Vec<Task> = vec![];
        assert_eq!(next_task_num(&empty), 1);

        // Subtask-only suffixes must not inflate the top-level counter.
        let subs: Vec<Task> = serde_json::from_value(serde_json::json!([
            {"id": "CVH-002", "title": "a", "stageId": "backlog"},
            {"id": "CVH-002.9", "title": "sub", "stageId": "dev", "parentId": "CVH-002", "subtaskIndex": 9}
        ]))
        .expect("deserialize tasks");
        assert_eq!(next_task_num(&subs), 3);
    }

    #[tokio::test]
    async fn test_persistent_monotonic_sequence_prevent_deleted_id_reuse() {
        let tmp = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_seq_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        let mut tasks: Vec<Task> = serde_json::from_value(serde_json::json!([
            {"id": "CVH-001", "title": "t1", "stageId": "backlog"},
            {"id": "CVH-002", "title": "t2", "stageId": "backlog"},
            {"id": "CVH-003", "title": "t3", "stageId": "backlog"}
        ])).unwrap();

        // 1. Initial allocation on existing tasks -> must be CVH-004
        let num1 = allocate_next_task_num(&tmp, &tasks).await;
        assert_eq!(num1, 4);

        // Verify sequence was persisted to disk in .cvh/settings/sequence.json
        let seq_file = tmp.join(".cvh").join("settings").join("sequence.json");
        assert!(seq_file.is_file());
        let seq_on_disk = load_task_sequence(&tmp).await;
        assert_eq!(seq_on_disk, 4);

        // 2. User deletes CVH-004 and CVH-003 from tasks
        tasks.retain(|t| t.id != "CVH-003");
        // tasks now only has CVH-001 and CVH-002 (max_on_disk = 2).
        // BUT sequence counter on disk is 4!
        // Next allocation MUST NOT re-issue CVH-003 or CVH-004! It must be CVH-005.
        let num2 = allocate_next_task_num(&tmp, &tasks).await;
        assert_eq!(num2, 5, "Deleted task IDs (CVH-003, CVH-004) must NEVER be reissued!");

        // 3. User deletes ALL tasks (empty tasks list)
        tasks.clear();
        // Even when all tasks are gone, the sequence counter must increment monotonically
        let num3 = allocate_next_task_num(&tmp, &tasks).await;
        assert_eq!(num3, 6, "Empty tasks list must continue monotonically from stored sequence!");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[test]
    fn test_sanitize_task_id() {
        assert!(sanitize_task_id("CVH-001").is_ok());
        assert!(sanitize_task_id("task-123").is_ok());
        assert!(sanitize_task_id("feature_test").is_ok());

        assert!(sanitize_task_id("").is_err());
        assert!(sanitize_task_id("   ").is_err());
        assert!(sanitize_task_id("../escape").is_err());
        assert!(sanitize_task_id("a/b").is_err());
        assert!(sanitize_task_id("a\\b").is_err());
        assert!(sanitize_task_id("task:1").is_err());
        assert!(sanitize_task_id("task*1").is_err());
        assert!(sanitize_task_id("task?1").is_err());
    }

    #[test]
    fn test_activity_item_serialization_camel_case() {
        let item = ActivityItem {
            id: "th-123".to_string(),
            item_type: "thought".to_string(),
            timestamp: "12:00:00".to_string(),
            content: Some("Thinking about the architecture...".to_string()),
            tool: None,
            duration_ms: Some(1500.0),
            images: None,
        };

        let json = serde_json::to_string(&item).expect("serialize");
        assert!(json.contains(r#""type":"thought""#));
        assert!(json.contains(r#""durationMs":1500"#));
        assert!(!json.contains("item_type"));
        assert!(!json.contains("duration_ms"));

        let deserialized: ActivityItem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.id, "th-123");
        assert_eq!(deserialized.item_type, "thought");
        assert_eq!(deserialized.content.as_deref(), Some("Thinking about the architecture..."));
        assert_eq!(deserialized.duration_ms, Some(1500.0));
    }

    #[tokio::test]
    async fn test_task_dialog_sqlite_roundtrip() {
        let base = PathBuf::from("target").join(format!("cvh_dialog_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let items = vec![
            ActivityItem {
                id: "user-1".to_string(),
                item_type: "user".to_string(),
                timestamp: "10:00:00".to_string(),
                content: Some("Почини сохранение диалогов".to_string()),
                tool: None,
                duration_ms: None,
                images: None,
            },
            ActivityItem {
                id: "th-1".to_string(),
                item_type: "thought".to_string(),
                timestamp: "10:00:01".to_string(),
                content: Some("Анализирую требования к персистентности...".to_string()),
                tool: None,
                duration_ms: Some(2500.0),
                images: None,
            },
            ActivityItem {
                id: "call-1".to_string(),
                item_type: "tool".to_string(),
                timestamp: "10:00:03".to_string(),
                content: None,
                tool: Some(serde_json::json!({
                    "id": "call-1",
                    "toolName": "read",
                    "status": "success",
                    "exitCode": 0
                })),
                duration_ms: Some(120.0),
                images: None,
            },
            ActivityItem {
                id: "out-1".to_string(),
                item_type: "output".to_string(),
                timestamp: "10:00:05".to_string(),
                content: Some("Диалоги успешно сохраняются в SQLite.".to_string()),
                tool: None,
                duration_ms: None,
                images: None,
            },
        ];

        crate::db::append_dialog_items(&base, "CVH-001", &items).expect("write dialog to sqlite");
        let loaded = crate::db::load_dialog_items(&base, "CVH-001").expect("load dialog from sqlite");

        assert_eq!(loaded.len(), 4);
        assert_eq!(loaded[0].item_type, "user");
        assert_eq!(loaded[1].item_type, "thought");
        assert_eq!(loaded[2].item_type, "tool");
        assert_eq!(loaded[3].item_type, "output");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[test]
    fn test_slugify_board_name_cyrillic_and_latin() {
        assert_eq!(slugify_board_name("3D Моделирование"), "3d-modelirovanie");
        assert_eq!(slugify_board_name("Разработка"), "razrabotka");
        assert_eq!(slugify_board_name("Dev Pipeline"), "dev-pipeline");
        assert_eq!(slugify_board_name("Engineering Core 2"), "engineering-core-2");
    }

    #[test]
    fn test_task_board_id_serialization() {
        let json_with_board = r#"{
            "id": "CVH-100",
            "title": "3D asset task",
            "stageId": "concept",
            "boardId": "modeler"
        }"#;

        let task: Task = serde_json::from_str(json_with_board).expect("deserialize task");
        assert_eq!(task.board_id.as_deref(), Some("modeler"));

        let json_without_board = r#"{
            "id": "CVH-101",
            "title": "Legacy task",
            "stageId": "dev"
        }"#;

        let task_without_board: Task = serde_json::from_str(json_without_board).expect("deserialize task without board");
        assert!(task_without_board.board_id.is_none());

        let default_cfg = crate::config::default_boards_config();
        let fixed = ensure_task_board_id(vec![task_without_board], &default_cfg);
        assert_eq!(fixed[0].board_id.as_deref(), Some("development"));
    }

    #[test]
    fn test_self_healing_orphaned_board_and_column() {
        use crate::commands::tasks::{heal_task_board_and_column, heal_tasks};
        use crate::config::{BoardDef, BoardsConfig, PipelineColumn};

        let custom_cfg = BoardsConfig {
            active_board_id: "feature-board".to_string(),
            boards: vec![
                BoardDef {
                    id: "feature-board".to_string(),
                    name: "Feature Board".to_string(),
                    columns: vec![
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
                            hooks: std::collections::HashMap::new(),
                            routes: None,
                            auto_run: None,
                            terminal: None,
                            is_intake: Some(true),
                        },
                        PipelineColumn {
                            id: "in_progress".to_string(),
                            title: "In Progress".to_string(),
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
                            hooks: std::collections::HashMap::new(),
                            routes: None,
                            auto_run: None,
                            terminal: None,
                            is_intake: None,
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
                            hooks: std::collections::HashMap::new(),
                            routes: None,
                            auto_run: None,
                            terminal: Some(true),
                            is_intake: None,
                        },
                    ],
                },
            ],
        };

        // 1. Orphaned board_id should be remapped to active_board_id ("feature-board")
        // and non-existent column "old_unknown_stage" should be healed to "backlog"
        let mut orphaned_task = Task {
            id: "CVH-200".to_string(),
            title: "Orphaned Task".to_string(),
            description: String::new(),
            stage_id: "old_unknown_stage".to_string(),
            board_id: Some("deleted_board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: None,
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
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        let modified = heal_task_board_and_column(&mut orphaned_task, &custom_cfg);
        assert!(modified);
        assert_eq!(orphaned_task.board_id.as_deref(), Some("feature-board"));
        assert_eq!(orphaned_task.stage_id, "backlog");

        // 2. Valid task should remain unchanged
        let mut valid_task = Task {
            id: "CVH-201".to_string(),
            title: "Valid Task".to_string(),
            description: String::new(),
            stage_id: "in_progress".to_string(),
            board_id: Some("feature-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: None,
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
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        let modified_valid = heal_task_board_and_column(&mut valid_task, &custom_cfg);
        assert!(!modified_valid);
        assert_eq!(valid_task.board_id.as_deref(), Some("feature-board"));
        assert_eq!(valid_task.stage_id, "in_progress");

        // 3. heal_tasks on vector
        let mut tasks = vec![orphaned_task.clone(), valid_task.clone()];
        tasks[0].board_id = None;
        let any_mod = heal_tasks(&mut tasks, &custom_cfg);
        assert!(any_mod);
        assert_eq!(tasks[0].board_id.as_deref(), Some("feature-board"));
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

    #[allow(dead_code)]
    fn hook_col(id: &str, on_enter: Option<&str>) -> PipelineColumn {
        let mut col = test_col(id);
        if let Some(v) = on_enter {
            col.hooks.insert("on_enter".to_string(), v.into());
        }
        col
    }

    fn task_with_wt(id: &str, wt: Option<&str>) -> Task {
        let mut t: Task = serde_json::from_value(serde_json::json!({
            "id": id, "title": "t", "stageId": "backlog"
        }))
        .expect("deserialize task");
        t.worktree_path = wt.map(|s| s.to_string());
        t
    }

    #[test]
    fn test_apply_worktree_from_hook_stdout() {
        let mut task = task_with_wt("CVH-001", None);
        apply_worktree_from_hook_stdout(
            &mut task,
            "some log\nCVH_WORKTREE=/tmp/wt-cvh-001\nCVH_BRANCH=feat/cvh-001\n",
        );
        assert_eq!(task.worktree_path.as_deref(), Some("/tmp/wt-cvh-001"));
        assert_eq!(task.branch.as_deref(), Some("feat/cvh-001"));

        // CVH_BRANCH does not overwrite existing non-empty branch
        let mut task2 = task_with_wt("CVH-002", None);
        task2.branch = Some("feat/keep".to_string());
        apply_worktree_from_hook_stdout(&mut task2, "CVH_BRANCH=feat/other");
        assert_eq!(task2.branch.as_deref(), Some("feat/keep"));

        // Plain stdout without markers leaves fields intact
        let mut task3 = task_with_wt("CVH-003", None);
        apply_worktree_from_hook_stdout(&mut task3, "plain log, no markers\n");
        assert!(task3.worktree_path.is_none());

        // Windows backslashes in CVH_WORKTREE are normalized to forward slashes
        let mut task4 = task_with_wt("CVH-004", None);
        apply_worktree_from_hook_stdout(&mut task4, "CVH_WORKTREE=worktrees\\CVH-004\n");
        assert_eq!(task4.worktree_path.as_deref(), Some("worktrees/CVH-004"));
    }

    #[tokio::test]
    async fn test_scaffold_backlog_hook_with_and_without_git() {
        // Scaffold creates clean minimal pipeline with empty hooks (pure script governance)
        let with_git =
            PathBuf::from("target").join(format!("cvh_scaffold_git_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(with_git.join(".git")).await.expect("git dir");
        crate::config::scaffold_cvh_project(&with_git).await.expect("scaffold");
        let raw = tokio::fs::read_to_string(with_git.join(".cvh").join("pipeline.json"))
            .await
            .expect("read pipeline");
        let pipe: crate::config::PipelineConfig =
            serde_json::from_str(&raw).expect("parse pipeline");
        let backlog = pipe.columns.iter().find(|c| c.id == "backlog").expect("backlog");
        assert!(backlog.hooks.is_empty());

        // Repeated scaffold preserves existing pipeline.json
        crate::config::scaffold_cvh_project(&with_git).await.expect("rescaffold");
        let raw2 = tokio::fs::read_to_string(with_git.join(".cvh").join("pipeline.json"))
            .await
            .expect("reread");
        assert_eq!(raw, raw2);

        // Without .git -> hooks empty
        let no_git =
            PathBuf::from("target").join(format!("cvh_scaffold_nogit_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&no_git).await.expect("plain dir");
        crate::config::scaffold_cvh_project(&no_git).await.expect("scaffold");
        let raw_n = tokio::fs::read_to_string(no_git.join(".cvh").join("pipeline.json"))
            .await
            .expect("read pipeline");
        let pipe_n: crate::config::PipelineConfig =
            serde_json::from_str(&raw_n).expect("parse pipeline");
        let backlog_n = pipe_n.columns.iter().find(|c| c.id == "backlog").expect("backlog");
        assert!(backlog_n.hooks.is_empty());

        let _ = tokio::fs::remove_dir_all(&with_git).await;
        let _ = tokio::fs::remove_dir_all(&no_git).await;
    }
    #[tokio::test]
    async fn test_advance_task_and_board_switch_finalization() {
        let base = PathBuf::from("target").join(format!("cvh_adv_switch_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let b1_cols = vec![
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
                is_intake: Some(true),
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
                is_intake: None,
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
                is_intake: None,
            },
        ];

        let b2_cols = vec![
            crate::config::PipelineColumn {
                id: "todo".to_string(),
                title: "Todo".to_string(),
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
                is_intake: Some(true),
            },
            crate::config::PipelineColumn {
                id: "in_progress".to_string(),
                title: "In Progress".to_string(),
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
                is_intake: None,
            },
            crate::config::PipelineColumn {
                id: "done".to_string(),
                title: "Done".to_string(),
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
                is_intake: None,
            },
        ];

        let mut boards_cfg = crate::config::BoardsConfig {
            active_board_id: "dev-board".to_string(),
            boards: vec![
                crate::config::BoardDef {
                    id: "dev-board".to_string(),
                    name: "Development".to_string(),
                    columns: b1_cols,
                },
                crate::config::BoardDef {
                    id: "kanban-board".to_string(),
                    name: "Kanban".to_string(),
                    columns: b2_cols,
                },
            ],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task on dev-board in stage dev
        let task1 = Task {
            id: "CVH-001".to_string(),
            title: "Task 1".to_string(),
            description: "Desc 1".to_string(),
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
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[task1]).await.expect("save task1");

        // 1. Normal advance on same board -> transitions to 'ready' with status 'done'
        let adv_res = internal_advance_task(&base, "CVH-001", "Summary for task 1", None).await.expect("advance task1");
        assert!(adv_res.contains("УСПЕШНО"));
        let loaded = load_tasks_from_disk(&base).await;
        let t1 = loaded.iter().find(|t| t.id == "CVH-001").unwrap();
        assert_eq!(t1.stage_id, "ready");
        assert_eq!(t1.status.as_deref(), Some("done"));

        // 2. Board switch scenario (Ironclad Board Preservation Invariant):
        // Seed task 2 on dev-board in stage dev
        let task2 = Task {
            id: "CVH-002".to_string(),
            title: "Task 2".to_string(),
            description: "Desc 2".to_string(),
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
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[t1.clone(), task2]).await.expect("save task2");

        // Now active board switches to kanban-board, but task 2 belongs to dev-board.
        boards_cfg.active_board_id = "kanban-board".to_string();
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards switch");

        // Advance task 2: active board changed -> task MUST strictly preserve its board_id ('dev-board')
        // and advance to terminal column ('ready') on dev-board without hopping to kanban-board!
        let adv_switch_res = internal_advance_task(&base, "CVH-002", "Summary after board switch", None).await.expect("advance task2 switch");
        assert!(adv_switch_res.contains("УСПЕШНО"));
        assert!(adv_switch_res.contains("завершена"));

        let loaded2 = load_tasks_from_disk(&base).await;
        let t2 = loaded2.iter().find(|t| t.id == "CVH-002").unwrap();
        assert_eq!(t2.board_id.as_deref(), Some("dev-board"), "Task must strictly preserve dev-board despite active board switch");
        assert_eq!(t2.stage_id, "ready");
        assert_eq!(t2.status.as_deref(), Some("done"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_move_task_to_backlog_resets_idle_status() {
        let base = PathBuf::from("target").join(format!("cvh_backlog_abort_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let cols = vec![
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
                is_intake: Some(true),
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
                terminal: Some(true),
                is_intake: None,
            },
        ];

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "default-board".to_string(),
            boards: vec![
                crate::config::BoardDef {
                    id: "default-board".to_string(),
                    name: "Default".to_string(),
                    columns: cols,
                },
            ],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed a running task in dev
        let task = Task {
            id: "CVH-ABORT-1".to_string(),
            title: "Running Task".to_string(),
            description: "Desc".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("default-board".to_string()),
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
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[task]).await.expect("save task");

        // Move to backlog
        let moved = internal_move_task_stage(
            &base,
            "CVH-ABORT-1",
            "backlog",
            None,
            None,
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move to backlog");

        assert_eq!(moved.stage_id, "backlog");
        assert_eq!(moved.status.as_deref(), Some("idle"));

        let loaded = load_tasks_from_disk(&base).await;
        assert_eq!(loaded[0].stage_id, "backlog");
        assert_eq!(loaded[0].status.as_deref(), Some("idle"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_resolve_task_active_agent_model_strict_task_override() {
        let base = PathBuf::from("target").join(format!("cvh_model_res_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let llm_settings = crate::config::LlmSettings {
            model: "global-default-model".to_string(),
            ..Default::default()
        };

        let mut task = Task {
            id: "CVH-001".to_string(),
            title: "Task".to_string(),
            description: String::new(),
            stage_id: "dev".to_string(),
            board_id: None,
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: None,
            model: Some("user-custom-model".to_string()),
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
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };

        // 1. When task.model is set, it strictly overrides global model and column model!
        let resolved = resolve_task_active_agent_model(&base, Some(&task), &llm_settings).await;
        assert_eq!(resolved, "user-custom-model");

        // 2. When task.model is None or empty, it falls back to global model
        task.model = None;
        let fallback_global = resolve_task_active_agent_model(&base, Some(&task), &llm_settings).await;
        assert_eq!(fallback_global, "global-default-model");

        task.model = Some("   ".to_string());
        let fallback_whitespace = resolve_task_active_agent_model(&base, Some(&task), &llm_settings).await;
        assert_eq!(fallback_whitespace, "global-default-model");

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

    #[test]
    fn test_is_matching_subagent_key() {
        use crate::commands::tasks::is_matching_subagent_key;

        // Exact match
        assert!(is_matching_subagent_key("CVH-001.sub-abc123", "CVH-001.sub-abc123"));

        // Strip :sub: prefix
        assert!(is_matching_subagent_key("CVH-001.sub-abc123", "CVH-001:sub:CVH-001.sub-abc123"));
        assert!(is_matching_subagent_key("CVH-001:sub:CVH-001.sub-abc123", "CVH-001.sub-abc123"));

        // Short suffix match
        assert!(is_matching_subagent_key("CVH-001.sub-abc123", "CVH-001:sub:abc123"));
        assert!(is_matching_subagent_key("CVH-001.sub-abc123", "abc123"));

        // Sibling subagents must NEVER match
        assert!(!is_matching_subagent_key("CVH-001.sub-xyz999", "CVH-001.sub-abc123"));

        // Parent task must NEVER match child subagent
        assert!(!is_matching_subagent_key("CVH-001", "CVH-001.sub-abc123"));
        assert!(!is_matching_subagent_key("CVH-001", "abc123"));
    }

    #[test]
    fn test_skip_worktree_flag_serialization_and_defaults() {
        // Deserialization with skipWorktree = true
        let t1: Task = serde_json::from_value(serde_json::json!({
            "id": "CVH-101",
            "title": "Skip wt task",
            "stageId": "dev",
            "skipWorktree": true
        })).expect("deserialize t1");
        assert_eq!(t1.skip_worktree, Some(true));

        // Deserialization with alias skip_worktree = false
        let t2: Task = serde_json::from_value(serde_json::json!({
            "id": "CVH-102",
            "title": "Don't skip wt task",
            "stageId": "dev",
            "skip_worktree": false
        })).expect("deserialize t2");
        assert_eq!(t2.skip_worktree, Some(false));

        // Deserialization with alias skipBacklogHook = true
        let t_sbh: Task = serde_json::from_value(serde_json::json!({
            "id": "CVH-102b",
            "title": "Skip backlog hook task",
            "stageId": "backlog",
            "skipBacklogHook": true
        })).expect("deserialize t_sbh");
        assert_eq!(t_sbh.skip_worktree, Some(true));

        // Deserialization with alias skip_backlog_hook = true
        let t_sbh2: Task = serde_json::from_value(serde_json::json!({
            "id": "CVH-102c",
            "title": "Skip backlog hook task snake",
            "stageId": "backlog",
            "skip_backlog_hook": true
        })).expect("deserialize t_sbh2");
        assert_eq!(t_sbh2.skip_worktree, Some(true));

        // Deserialization with default when omitted
        let t3: Task = serde_json::from_value(serde_json::json!({
            "id": "CVH-103",
            "title": "Default wt task",
            "stageId": "dev"
        })).expect("deserialize t3");
        assert_eq!(t3.skip_worktree, None);
    }

    #[tokio::test]
    async fn test_skip_worktree_prevents_builtin_worktree_in_move_stage() {
        let base = PathBuf::from("target").join(format!("cvh_skip_wt_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("test dir");

        let pipe = crate::config::default_pipeline_config();
        crate::config::save_pipeline_config(&base, &pipe).await.expect("save pipeline");

        let task = Task {
            id: "CVH-SKIP-1".to_string(),
            title: "No Worktree Task".to_string(),
            description: "Root only".to_string(),
            stage_id: "backlog".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[task]).await.expect("save task");

        // Move to dev stage
        let moved = internal_move_task_stage(
            &base,
            "CVH-SKIP-1",
            "dev",
            None,
            None,
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move task stage");

        // When skip_worktree is true, NO worktree or branch must be created!
        assert_eq!(moved.stage_id, "dev");
        assert_eq!(moved.worktree_path, None);
        assert_eq!(moved.branch, None);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[test]
    fn test_active_agents_duplicate_run_check() {
        let active_map = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        let task_id = "CVH-042".to_string();

        // 1. Not in map -> not running
        let is_running_initial = active_map.lock().map(|map| map.contains_key(&task_id)).unwrap_or(false);
        assert!(!is_running_initial);

        // 2. Insert dummy handle
        let abort_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let join_slot = std::sync::Arc::new(tokio::sync::Mutex::new(None));
        active_map.lock().unwrap().insert(
            task_id.clone(),
            crate::commands::ActiveAgentHandle {
                abort_flag,
                join_handle: join_slot,
            },
        );

        // 3. Now in map -> is_running is true, reject duplicate run
        let is_running_after = active_map.lock().map(|map| map.contains_key(&task_id)).unwrap_or(false);
        assert!(is_running_after);
    }

    #[tokio::test]
    async fn test_append_llm_debug_log() {
        let base = PathBuf::from("target").join(format!("cvh_log_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        crate::agent::append_llm_debug_log(&base, "[TEST] Turn 1 Request
").await;
        crate::agent::append_llm_debug_log(&base, "[TEST] Turn 1 Response
").await;

        let log_file = base.join(".cvh").join("llm_debug.log");
        assert!(log_file.exists());
        let content = tokio::fs::read_to_string(log_file).await.expect("read log");
        assert!(content.contains("[TEST] Turn 1 Request"));
        assert!(content.contains("[TEST] Turn 1 Response"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_internal_advance_task_creates_review_subtask_and_no_self_abort() {
        let base = PathBuf::from("target").join(format!("cvh_review_subtask_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let cols = vec![
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
                is_intake: Some(true),
            },
            crate::config::PipelineColumn {
                id: "dev".to_string(),
                title: "Development".to_string(),
                color: None,
                icon: None,
                persona: Some("cvh-dev".to_string()),
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: Some(true),
                terminal: None,
                is_intake: None,
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
                auto_run: Some(true),
                terminal: None,
                is_intake: None,
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
                is_intake: None,
            },
        ];

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "dev-board".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "dev-board".to_string(),
                name: "Development".to_string(),
                columns: cols,
            }],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed parent task in 'dev' stage with running status and worktree details
        let parent_task = Task {
            id: "CVH-100".to_string(),
            title: "Implement Review Subtask".to_string(),
            description: "Implement § 2.2 Review Subtask Hierarchy".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("dev-board".to_string()),
            worktree_path: Some("C:\\fake\\worktree".to_string()),
            branch: Some("feat/CVH-100".to_string()),
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: Some("custom-model".to_string()),
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            priority: Some("high".to_string()),
            tags: None,
            current_tokens: None,
            skip_worktree: Some(false),
            created_at: 1000,
            updated_at: 1000,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[parent_task]).await.expect("save parent task");

        // 1. Advance task targeting review stage:
        // Must create child subtask CVH-100.1 in 'review' column and set parent to 'waiting_subtask'
        let adv_res = internal_advance_task(
            &base,
            "CVH-100",
            "Handover: implemented feature and tests",
            None,
        )
        .await
        .expect("internal_advance_task must succeed");

        assert!(adv_res.contains("УСПЕШНО: Создана подзадача CVH-100.1"));

        let loaded = load_tasks_from_disk(&base).await;
        assert_eq!(loaded.len(), 2, "Must contain parent and child subtask");

        let parent = loaded.iter().find(|t| t.id == "CVH-100").unwrap();
        assert_eq!(parent.stage_id, "dev", "Parent must remain pinned in 'dev' stage (§ 2.2)");
        assert_eq!(parent.status.as_deref(), Some("waiting_subtask"), "Parent status must be 'waiting_subtask'");
        assert_eq!(parent.waiting_for_subtask_id.as_deref(), Some("CVH-100.1"), "Parent must be waiting for CVH-100.1");

        let subtask = loaded.iter().find(|t| t.id == "CVH-100.1").unwrap();
        assert_eq!(subtask.parent_id.as_deref(), Some("CVH-100"));
        assert_eq!(subtask.subtask_index, Some(1));
        assert_eq!(subtask.stage_id, "review", "Subtask must be in target review column");
        assert_eq!(subtask.status.as_deref(), Some("running"), "Target has persona 'reviewer', status must be 'running'");
        assert_eq!(subtask.title, "Implement Review Subtask (Review)");
        assert_eq!(subtask.description, "Handover: implemented feature and tests");
        assert_eq!(subtask.handover_summary.as_deref(), Some("Handover: implemented feature and tests"));
        assert_eq!(subtask.worktree_path.as_deref(), Some("C:\\fake\\worktree"));
        assert_eq!(subtask.branch.as_deref(), Some("feat/CVH-100"));
        assert_eq!(subtask.skip_worktree, Some(false));
        assert_eq!(subtask.model.as_deref(), Some("custom-model"));
        assert_eq!(subtask.priority.as_deref(), Some("high"));

        // Verify SQLite dialog items contain handover summary on parent (deduplicated on subtask dialog to prevent phantom bubbles)
        let parent_dialog = crate::db::load_dialog_items(&base, "CVH-100").expect("load parent dialog");
        assert!(parent_dialog.iter().any(|d| d.content.as_deref().unwrap_or("").contains("Handover: implemented feature and tests")));
        assert_eq!(subtask.handover_summary.as_deref(), Some("Handover: implemented feature and tests"));

        // 1b. Verify subtask deduplication protection (§ 1):
        // Calling advance_task a second time must NOT create CVH-100.2!
        let dup_attempt = internal_advance_task_to_stage(
            &base,
            "CVH-100",
            None,
            "Second handover attempt",
            None,
        )
        .await
        .expect("dedup check should succeed with guard message");
        assert!(dup_attempt.contains("уже активна"), "Must indicate subtask is already active: {}", dup_attempt);
        let loaded_dedup = load_tasks_from_disk(&base).await;
        assert_eq!(loaded_dedup.len(), 2, "Must NOT create duplicate subtask (still 2 tasks)");

        // 2. Verify zero self-abort invariant:
        // Advancing when is_autonomous_advance is true must never abort running tasks
        let moved_autonomous = internal_move_task_stage(
            &base,
            "CVH-100.1",
            "ready",
            Some("APPROVED".to_string()),
            Some("done".to_string()),
            Some("Review approved".to_string()),
            None,
            true, // is_autonomous_advance = true!
            None,
        )
        .await
        .expect("autonomous move to ready must succeed");

        assert_eq!(moved_autonomous.stage_id, "ready");
        assert_eq!(moved_autonomous.status.as_deref(), Some("done"));

        // Verify relational symmetry (§ 2.2, § 2.7 & § 2.22):
        // Parent task CVH-100 must also be automatically advanced to 'ready' with status 'done' and verdict 'APPROVED'
        let loaded_after = load_tasks_from_disk(&base).await;
        let parent_after = loaded_after.iter().find(|t| t.id == "CVH-100").unwrap();
        assert_eq!(parent_after.stage_id, "ready", "Parent must be advanced to 'ready' alongside approved child subtask");
        assert_eq!(parent_after.status.as_deref(), Some("done"), "Parent status must be 'done'");
        assert_eq!(parent_after.verdict.as_deref(), Some("APPROVED"), "Parent verdict must be 'APPROVED'");
        assert_eq!(parent_after.waiting_for_subtask_id, None, "Parent waiting_for_subtask_id must be cleared");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_subtask_advancement_5_columns_and_nesting() {
        let base = PathBuf::from("target").join(format!("cvh_5col_nesting_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let cols = vec![
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
                is_intake: Some(true),
            },
            crate::config::PipelineColumn {
                id: "dev".to_string(),
                title: "Development".to_string(),
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
                auto_run: Some(true),
                terminal: None,
                is_intake: None,
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
                auto_run: Some(true),
                terminal: None,
                is_intake: None,
            },
            crate::config::PipelineColumn {
                id: "retro".to_string(),
                title: "Retrospective".to_string(),
                color: None,
                icon: None,
                persona: Some("retro-verifier".to_string()),
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
                auto_run: Some(true),
                terminal: None,
                is_intake: None,
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
                is_intake: None,
            },
        ];

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "5col-board".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "5col-board".to_string(),
                name: "5-Column Board".to_string(),
                columns: cols,
            }],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed parent task in dev
        let parent = Task {
            id: "CVH-200".to_string(),
            title: "Task in 5-stage pipeline".to_string(),
            description: "Desc".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("5col-board".to_string()),
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
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[parent]).await.expect("save parent");

        // 1. Advance parent: creates review subtask CVH-200.1
        let adv_res = internal_advance_task(&base, "CVH-200", "Handover to review", None)
            .await
            .expect("advance to review subtask");
        assert!(adv_res.contains("УСПЕШНО: Создана подзадача CVH-200.1"));

        let loaded = load_tasks_from_disk(&base).await;
        let p1 = loaded.iter().find(|t| t.id == "CVH-200").unwrap();
        assert_eq!(p1.status.as_deref(), Some("waiting_subtask"));
        assert_eq!(p1.waiting_for_subtask_id.as_deref(), Some("CVH-200.1"));

        // 2. Advance subtask CVH-200.1 to ready (terminal column):
        // In 5-column pipeline [backlog, dev, review, retro, ready]:
        // When review subtask finishes, parent must advance past review into 'retro'!
        let move_sub = internal_move_task_stage(
            &base,
            "CVH-200.1",
            "ready",
            Some("APPROVED".to_string()),
            Some("done".to_string()),
            Some("Review approved".to_string()),
            None,
            true,
            None,
        )
        .await
        .expect("move subtask to ready");
        assert_eq!(move_sub.stage_id, "ready");
        assert_eq!(move_sub.status.as_deref(), Some("done"));

        let loaded2 = load_tasks_from_disk(&base).await;
        let p2 = loaded2.iter().find(|t| t.id == "CVH-200").unwrap();
        assert_eq!(p2.stage_id, "dev", "Parent stays pinned in dev stage");
        assert_eq!(p2.status.as_deref(), Some("waiting_subtask"), "Parent status is waiting_subtask");
        assert_eq!(p2.waiting_for_subtask_id.as_deref(), Some("CVH-200.2"), "Parent waits for CVH-200.2 in retro");

        let s2 = loaded2.iter().find(|t| t.id == "CVH-200.2").unwrap();
        assert_eq!(s2.stage_id, "retro", "Subtask CVH-200.2 is in retro stage");
        assert_eq!(s2.status.as_deref(), Some("running"));

        // 3. Now advance subtask CVH-200.2 from retro to ready:
        let move_retro_ready = internal_move_task_stage(
            &base,
            "CVH-200.2",
            "ready",
            Some("APPROVED".to_string()),
            Some("done".to_string()),
            Some("Retro completed".to_string()),
            None,
            true,
            None,
        )
        .await
        .expect("move retro subtask to ready");
        assert_eq!(move_retro_ready.stage_id, "ready");
        assert_eq!(move_retro_ready.status.as_deref(), Some("done"));

        let loaded3 = load_tasks_from_disk(&base).await;
        let p3 = loaded3.iter().find(|t| t.id == "CVH-200").unwrap();
        let s3 = loaded3.iter().find(|t| t.id == "CVH-200.1").unwrap();
        assert_eq!(p3.stage_id, "ready");
        assert_eq!(p3.status.as_deref(), Some("done"));
        assert_eq!(s3.stage_id, "ready");
        assert_eq!(s3.status.as_deref(), Some("done"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_nested_subtasks_advancement() {
        let base = PathBuf::from("target").join(format!("cvh_nested_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let cols = vec![
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
                is_intake: Some(true),
            },
            crate::config::PipelineColumn {
                id: "dev".to_string(),
                title: "Development".to_string(),
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
                is_intake: None,
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
                is_intake: None,
            },
        ];

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "nest-board".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "nest-board".to_string(),
                name: "Nest Board".to_string(),
                columns: cols,
            }],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Grandparent CVH-300 waiting on Parent CVH-300.1
        let gp = Task {
            id: "CVH-300".to_string(),
            title: "Grandparent Task".to_string(),
            description: "".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("nest-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("waiting_subtask".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: Some("CVH-300.1".to_string()),
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };

        // Parent CVH-300.1 waiting on Child CVH-300.1.1
        let parent = Task {
            id: "CVH-300.1".to_string(),
            title: "Child Subtask".to_string(),
            description: "".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("nest-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("waiting_subtask".to_string()),
            model: None,
            parent_id: Some("CVH-300".to_string()),
            subtask_index: Some(1),
            waiting_for_subtask_id: Some("CVH-300.1.1".to_string()),
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };

        // Child CVH-300.1.1 running in dev
        let child = Task {
            id: "CVH-300.1.1".to_string(),
            title: "Grandchild Subtask".to_string(),
            description: "".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("nest-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: Some("CVH-300.1".to_string()),
            subtask_index: Some(1),
            waiting_for_subtask_id: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            handover_summary: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };

        save_tasks_to_disk(&base, &[gp, parent, child]).await.expect("save tasks");

        // Advance grandchild CVH-300.1.1 to ready:
        // Must recursively advance CVH-300.1 to ready, AND CVH-300 to ready!
        let _ = internal_move_task_stage(
            &base,
            "CVH-300.1.1",
            "ready",
            Some("APPROVED".to_string()),
            Some("done".to_string()),
            None,
            None,
            true,
            None,
        )
        .await
        .expect("move grandchild to ready");

        let loaded = load_tasks_from_disk(&base).await;
        let gp_after = loaded.iter().find(|t| t.id == "CVH-300").unwrap();
        let p_after = loaded.iter().find(|t| t.id == "CVH-300.1").unwrap();
        let c_after = loaded.iter().find(|t| t.id == "CVH-300.1.1").unwrap();

        assert_eq!(c_after.stage_id, "ready");
        assert_eq!(c_after.status.as_deref(), Some("done"));

        assert_eq!(p_after.stage_id, "ready", "Parent must be advanced to ready recursively");
        assert_eq!(p_after.status.as_deref(), Some("done"));
        assert_eq!(p_after.waiting_for_subtask_id, None);

        assert_eq!(gp_after.stage_id, "ready", "Grandparent must be advanced to ready recursively");
        assert_eq!(gp_after.status.as_deref(), Some("done"));
        assert_eq!(gp_after.waiting_for_subtask_id, None);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_multi_board_move_task_board_preservation() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_test_multiboard_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("create .cvh");

        // Boards: 'development' (3 cols) and 'complex' (4 cols)
        let boards_json = serde_json::json!({
            "activeBoardId": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Development",
                    "columns": [
                        { "id": "backlog", "title": "Backlog", "color": "#71717a", "icon": "Inbox", "persona": null, "hooks": {} },
                        { "id": "dev", "title": "Dev", "color": "#3b82f6", "icon": "Code2", "persona": "cvh-dev", "hooks": {} },
                        { "id": "ready", "title": "Ready", "color": "#22c55e", "icon": "CheckCircle2", "persona": null, "terminal": true, "hooks": {} }
                    ]
                },
                {
                    "id": "complex",
                    "name": "Complex",
                    "columns": [
                        { "id": "backlog", "title": "Backlog", "color": "#71717a", "icon": "Inbox", "persona": null, "hooks": {} },
                        { "id": "dev", "title": "Dev", "color": "#3b82f6", "icon": "Code2", "persona": "cvh-dev", "hooks": {} },
                        { "id": "review", "title": "Review", "color": "#f59e0b", "icon": "ShieldCheck", "persona": "reviewer", "hooks": {} },
                        { "id": "ready", "title": "Ready", "color": "#22c55e", "icon": "CheckCircle2", "persona": null, "terminal": true, "hooks": {} }
                    ]
                }
            ]
        });
        tokio::fs::write(base.join(".cvh").join("boards.json"), serde_json::to_string(&boards_json).unwrap())
            .await
            .expect("write boards.json");

        let task = Task {
            id: "TASK-MB-1".to_string(),
            title: "Multi-board task".to_string(),
            description: "Test board preservation".to_string(),
            stage_id: "backlog".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            handover_summary: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            todos: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[task]).await.expect("save task");

        // 1. Move within 'development' board to 'dev' with explicit board_id_override:
        let m1 = internal_move_task_stage(
            &base,
            "TASK-MB-1",
            "dev",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move to dev");
        assert_eq!(m1.stage_id, "dev");
        assert_eq!(m1.board_id.as_deref(), Some("development"), "Must strictly preserve active board_id");

        // 2. Move to 'ready' without override (task's current_board is development, which has ready):
        let m2 = internal_move_task_stage(
            &base,
            "TASK-MB-1",
            "ready",
            None,
            None,
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move to ready");
        assert_eq!(m2.stage_id, "ready");
        assert_eq!(m2.board_id.as_deref(), Some("development"), "Must stay on development board");

        // 3. Move to 'review' which ONLY exists on 'complex' board:
        let m3 = internal_move_task_stage(
            &base,
            "TASK-MB-1",
            "review",
            None,
            None,
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move to review");
        assert_eq!(m3.stage_id, "review");
        assert_eq!(m3.board_id.as_deref(), Some("complex"), "Must cleanly transition to board containing stage");

        // 4. Move from 'review' back to 'dev' without override (current_board is now complex, which has dev):
        let m4 = internal_move_task_stage(
            &base,
            "TASK-MB-1",
            "dev",
            None,
            None,
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move to dev on complex");
        assert_eq!(m4.stage_id, "dev");
        assert_eq!(m4.board_id.as_deref(), Some("complex"), "Must stay on complex board");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_multi_board_pipeline_routing_and_board_preservation() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_test_routing_preservation_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(base.join(".cvh")).await.expect("create .cvh");

        // Boards: 'development' (active) and 'nastroyka' (Setup)
        let boards_json = serde_json::json!({
            "activeBoardId": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "CanbanVoimaxHarness",
                    "columns": [
                        { "id": "backlog", "title": "Backlog", "persona": null, "hooks": {} },
                        { "id": "dev", "title": "Dev", "persona": "cvh-dev", "hooks": {} },
                        { "id": "ready", "title": "Ready", "persona": null, "terminal": true, "hooks": {} }
                    ]
                },
                {
                    "id": "nastroyka",
                    "name": "Setup",
                    "columns": [
                        { "id": "backlog", "title": "Backlog", "persona": null, "hooks": {} },
                        { "id": "dev", "title": "CVH Development", "persona": "developer", "hooks": {} },
                        { "id": "ready", "title": "Ready", "persona": null, "terminal": true, "hooks": {} }
                    ]
                }
            ]
        });
        tokio::fs::write(base.join(".cvh").join("boards.json"), serde_json::to_string(&boards_json).unwrap())
            .await
            .expect("write boards.json");

        // Seed task CVH-011 on board 'nastroyka' in stage 'dev'
        let task = Task {
            id: "CVH-011".to_string(),
            title: "Setup Task".to_string(),
            description: "Test board preservation during advance".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("nastroyka".to_string()),
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
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            todos: None,
            attachments: None,
            cover_image: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&base, &[task]).await.expect("save task");

        // Advance task CVH-011: active board is 'development', but task is on 'nastroyka'
        let adv_res = internal_advance_task(&base, "CVH-011", "Task completed on nastroyka board", None)
            .await
            .expect("advance task");
        assert!(adv_res.contains("УСПЕШНО"));

        let loaded = load_tasks_from_disk(&base).await;
        let t = loaded.iter().find(|t| t.id == "CVH-011").unwrap();
        // Task MUST preserve its board 'nastroyka' and MUST NOT hop to 'development'
        assert_eq!(t.board_id.as_deref(), Some("nastroyka"), "Task must stay on 'nastroyka' board!");
        assert_eq!(t.stage_id, "ready");
        assert_eq!(t.status.as_deref(), Some("done"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_race_condition_create_task_early_persistence_and_move() {
        // Simulates the exact user issue:
        // 1. Task is created and persisted immediately to disk and memory (Point 1)
        // 2. While worktree creation is still underway, user moves the task to 'dev' (Point 2)
        // 3. internal_move_task_stage succeeds immediately without 'Task not found'
        // 4. Worktree completes and updates worktree_path/branch WITHOUT overwriting stage back to 'backlog'!
        let base = PathBuf::from("target").join(format!("cvh_race_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        // Seed basic board config
        let dev_board = crate::config::BoardsConfig {
            active_board_id: "development".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "development".to_string(),
                name: "Development".to_string(),
                columns: vec![
                    test_col("backlog"),
                    test_col("dev"),
                    test_col("ready"),
                ],
            }],
        };
        crate::config::save_boards_config(&base, &dev_board).await.expect("save boards");

        let task_id = "CVH-101".to_string();
        let mut new_task = Task {
            id: task_id.clone(),
            title: "Quick move task".to_string(),
            description: "Fast click to dev".to_string(),
            stage_id: "backlog".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
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
            skip_worktree: Some(false),
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        // Step 1: Early persistence (create_task fix)
        save_tasks_to_disk(&base, &[new_task.clone()]).await.expect("early save");

        // Step 2: User moves task to 'dev' concurrently (internal_move_task_stage)
        let moved = internal_move_task_stage(
            &base,
            &task_id,
            "dev",
            None,
            Some("running".to_string()),
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move task to dev must succeed immediately!");

        assert_eq!(moved.stage_id, "dev");
        assert_eq!(moved.status.as_deref(), Some("running"));

        // Step 3: Worktree completion completes late and updates worktree_path / branch
        new_task.worktree_path = Some("worktrees/cvh-101".to_string());
        new_task.branch = Some("feat/cvh-101".to_string());

        let mut current_tasks = load_tasks_from_disk(&base).await;
        if let Some(t) = current_tasks.iter_mut().find(|t| t.id == new_task.id) {
            t.worktree_path = new_task.worktree_path.clone();
            t.branch = new_task.branch.clone();
            t.updated_at = chrono::Utc::now().timestamp_millis();
        }
        save_tasks_to_disk(&base, &current_tasks).await.expect("save post worktree");

        // Verify: The task retained stage 'dev' and status 'running' and received the worktree!
        let final_loaded = load_tasks_from_disk(&base).await;
        let final_task = final_loaded.iter().find(|t| t.id == task_id).expect("task must exist on disk");
        assert_eq!(final_task.stage_id, "dev", "Stage must NOT be reverted to backlog!");
        assert_eq!(final_task.status.as_deref(), Some("running"));
        assert_eq!(final_task.worktree_path.as_deref(), Some("worktrees/cvh-101"));
        assert_eq!(final_task.branch.as_deref(), Some("feat/cvh-101"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_run_agent_preserves_in_memory_tasks_logic() {
        // Simulates Point 3: Disk has task A, in-memory AppState has task A and task B.
        // run_agent must merge in-memory tasks so task B is NOT wiped by disk_tasks snapshot!
        let base = PathBuf::from("target").join(format!("cvh_run_agent_mem_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let task_a = Task {
            id: "CVH-001".to_string(),
            title: "Task A".to_string(),
            description: "A".to_string(),
            stage_id: "backlog".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
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
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        let task_b = Task {
            id: "CVH-002".to_string(),
            title: "Task B (In-Memory)".to_string(),
            description: "B".to_string(),
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
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        // Disk has only Task A
        save_tasks_to_disk(&base, &[task_a.clone()]).await.expect("save task_a to disk");

        // Memory has Task A and Task B
        let mem_tasks = vec![task_a.clone(), task_b.clone()];

        // run_agent task resolution logic:
        let mut tasks_to_use = load_tasks_from_disk(&base).await;
        let initial_disk_count = tasks_to_use.len();
        for mem_task in &mem_tasks {
            if !tasks_to_use.iter().any(|t| t.id == mem_task.id) {
                tasks_to_use.push(mem_task.clone());
            }
        }

        // Verify task B is resolved
        let found_b = tasks_to_use.iter().find(|t| t.id == "CVH-002");
        assert!(found_b.is_some(), "Task B must be found in merged task list!");

        // Verify disk is updated with in-memory task
        if tasks_to_use.len() > initial_disk_count {
            save_tasks_to_disk(&base, &tasks_to_use).await.expect("flush memory tasks to disk");
        }

        let disk_after = load_tasks_from_disk(&base).await;
        assert_eq!(disk_after.len(), 2, "Task B must now be safely persisted to disk!");
        assert!(disk_after.iter().any(|t| t.id == "CVH-002"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_create_task_with_explicit_task_id_and_sequence_sync() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_explicit_id_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        // Scaffold minimal .cvh settings with sequence = 1
        save_task_sequence(&tmp, 1).await.expect("save initial sequence");
        assert_eq!(load_task_sequence(&tmp).await, 1);

        // Step 1: Create task with explicit task_id "CVH-005" (as frontend optimisticId)
        let created = internal_create_task(
            &tmp,
            "Optimistic Task".to_string(),
            "Created with frontend optimistic ID".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true), // skip_worktree for unit test speed
            None,
            Some("CVH-005".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("internal_create_task must succeed with explicit task_id");

        // Verify task used the exact explicit ID
        assert_eq!(created.id, "CVH-005");

        // Verify sequence on disk was synchronized to 5
        let seq_after_explicit = load_task_sequence(&tmp).await;
        assert_eq!(
            seq_after_explicit, 5,
            "Sequence on disk must be synchronized to the explicit task number (5)!"
        );

        // Step 2: Create next task without task_id (task_id: None)
        let created_next = internal_create_task(
            &tmp,
            "Subsequent Task".to_string(),
            "Created without explicit ID".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("internal_create_task without task_id must allocate next monotonic ID");

        // Must allocate monotonically after 5 -> CVH-006!
        assert_eq!(created_next.id, "CVH-006");
        let seq_after_next = load_task_sequence(&tmp).await;
        assert_eq!(seq_after_next, 6);

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_move_task_to_dev_autorun_and_running_status() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_dev_autorun_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        // Scaffold minimal boards.json with development board containing backlog and dev (persona cvh-dev)
        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "dev", "title": "В работе", "persona": "cvh-dev", "auto_run": true }
                    ]
                }
            ]
        });
        let boards_path = tmp.join(".cvh").join("boards.json");
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;
        tokio::fs::write(&boards_path, serde_json::to_string_pretty(&boards_cfg).unwrap())
            .await
            .expect("write boards.json");

        // Create task in backlog
        let task = internal_create_task(
            &tmp,
            "Dev Task".to_string(),
            "Move to dev test".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-005".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        assert_eq!(task.stage_id, "backlog");
        assert_ne!(task.status.as_deref(), Some("running"));

        // Move task to 'dev' column
        let moved = internal_move_task_stage(
            &tmp,
            "CVH-005",
            "dev",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move task to dev must succeed");

        assert_eq!(moved.stage_id, "dev");
        assert_eq!(
            moved.status.as_deref(),
            Some("running"),
            "Task moving to stage with persona 'cvh-dev' must automatically enter 'running' status!"
        );

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[test]
    fn test_task_id_matches_case_and_whitespace() {
        assert!(task_id_matches("CVH-001", "cvh-001"));
        assert!(task_id_matches("cvh-001", "CVH-001"));
        assert!(task_id_matches("  CVH-001  ", "cvh-001"));
        assert!(task_id_matches("CVH-001", "  cvh-001  "));
        assert!(task_id_matches("CVH-042.1", "cvh-042.1"));
        assert!(!task_id_matches("CVH-001", "CVH-002"));
    }

    #[tokio::test]
    async fn test_move_task_stage_case_insensitive_and_trimmed() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_case_insensitive_test_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        let dev_board = crate::config::BoardsConfig {
            active_board_id: "development".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "development".to_string(),
                name: "Development".to_string(),
                columns: vec![
                    test_col("backlog"),
                    test_col("dev"),
                    test_col("ready"),
                ],
            }],
        };
        crate::config::save_boards_config(&tmp, &dev_board).await.expect("save boards");

        let task = Task {
            id: "CVH-001".to_string(),
            title: "Case Insensitive Task".to_string(),
            description: "Test".to_string(),
            stage_id: "backlog".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("idle".to_string()),
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
            skip_worktree: Some(true),
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&tmp, &[task]).await.expect("save task");

        // Move using lowercase with spaces "  cvh-001  "
        let moved = internal_move_task_stage(
            &tmp,
            "  cvh-001  ",
            "dev",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move task with lowercase and whitespace must succeed!");

        assert_eq!(moved.id, "CVH-001");
        assert_eq!(moved.stage_id, "dev");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_terminal_task_clears_waiting_for_subtask_and_heal_tasks_cleans_stale_subtasks() {
        let tmp = PathBuf::from("target").join(format!("cvh_heal_subtasks_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&tmp).await.expect("test dir");

        let bcfg = crate::config::BoardsConfig {
            active_board_id: "development".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "development".to_string(),
                name: "Development".to_string(),
                columns: vec![
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
                        is_intake: Some(true),
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
                        auto_run: Some(true),
                        terminal: None,
                        is_intake: None,
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
                        is_intake: None,
                    },
                ],
            }],
        };

        let mut tasks = vec![
            Task {
                id: "CVH-500".to_string(),
                title: "Parent in ready".to_string(),
                description: String::new(),
                stage_id: "ready".to_string(),
                board_id: Some("development".to_string()),
                worktree_path: None,
                branch: None,
                verdict: None,
                custom_flow: None,
                status: Some("done".to_string()),
                model: None,
                parent_id: None,
                subtask_index: None,
                waiting_for_subtask_id: Some("CVH-500.1".to_string()),
                handover_summary: None,
                cover_image: None,
                attachments: None,
                todos: None,
                priority: None,
                tags: None,
                current_tokens: None,
                skip_worktree: Some(true),
                created_at: 0,
                updated_at: 0,
                stage_entered_at: None,
            completed_at: None,
                position: None,
                hook_states: None,
            },
            Task {
                id: "CVH-500.1".to_string(),
                title: "Completed subtask".to_string(),
                description: String::new(),
                stage_id: "dev".to_string(),
                board_id: Some("development".to_string()),
                worktree_path: None,
                branch: None,
                verdict: None,
                custom_flow: None,
                status: Some("done".to_string()),
                model: None,
                parent_id: Some("CVH-500".to_string()),
                subtask_index: Some(1),
                waiting_for_subtask_id: None,
                handover_summary: None,
                cover_image: None,
                attachments: None,
                todos: None,
                priority: None,
                tags: None,
                current_tokens: None,
                skip_worktree: Some(true),
                created_at: 0,
                updated_at: 0,
                stage_entered_at: None,
            completed_at: None,
                position: None,
                hook_states: None,
            },
            Task {
                id: "CVH-501".to_string(),
                title: "Parent with completed child".to_string(),
                description: String::new(),
                stage_id: "dev".to_string(),
                board_id: Some("development".to_string()),
                worktree_path: None,
                branch: None,
                verdict: None,
                custom_flow: None,
                status: Some("waiting_subtask".to_string()),
                model: None,
                parent_id: None,
                subtask_index: None,
                waiting_for_subtask_id: Some("CVH-501.1".to_string()),
                handover_summary: None,
                cover_image: None,
                attachments: None,
                todos: None,
                priority: None,
                tags: None,
                current_tokens: None,
                skip_worktree: Some(true),
                created_at: 0,
                updated_at: 0,
                stage_entered_at: None,
            completed_at: None,
                position: None,
                hook_states: None,
            },
            Task {
                id: "CVH-501.1".to_string(),
                title: "Subtask 501.1".to_string(),
                description: String::new(),
                stage_id: "dev".to_string(),
                board_id: Some("development".to_string()),
                worktree_path: None,
                branch: None,
                verdict: None,
                custom_flow: None,
                status: Some("done".to_string()),
                model: None,
                parent_id: Some("CVH-501".to_string()),
                subtask_index: Some(1),
                waiting_for_subtask_id: None,
                handover_summary: None,
                cover_image: None,
                attachments: None,
                todos: None,
                priority: None,
                tags: None,
                current_tokens: None,
                skip_worktree: Some(true),
                created_at: 0,
                updated_at: 0,
                stage_entered_at: None,
            completed_at: None,
                position: None,
                hook_states: None,
            },
        ];

        // 1. Verify heal_tasks cleans up waiting_for_subtask_id
        let healed = heal_tasks(&mut tasks, &bcfg);
        assert!(healed, "heal_tasks must return true when tasks are modified");

        assert_eq!(tasks[0].waiting_for_subtask_id, None, "Terminal parent must have waiting_for_subtask_id None");
        assert_eq!(tasks[0].status.as_deref(), Some("done"));

        assert_eq!(tasks[2].waiting_for_subtask_id, None, "Parent with completed child must have waiting_for_subtask_id cleared");
        assert_eq!(tasks[2].status.as_deref(), Some("idle"), "Status must revert from waiting_subtask to idle");

        // 2. Verify subtask advance to terminal via internal_advance_task_to_stage clears parent waiting_for_subtask_id
        crate::config::save_boards_config(&tmp, &bcfg).await.expect("save boards");
        let parent_active = Task {
            id: "CVH-600".to_string(),
            title: "Active Parent".to_string(),
            description: String::new(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("waiting_subtask".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: Some("CVH-600.1".to_string()),
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };
        let child_active = Task {
            id: "CVH-600.1".to_string(),
            title: "Child in dev".to_string(),
            description: String::new(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: Some("CVH-600".to_string()),
            subtask_index: Some(1),
            waiting_for_subtask_id: None,
            handover_summary: None,
            cover_image: None,
            attachments: None,
            todos: None,
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: 0,
            updated_at: 0,
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&tmp, &[parent_active, child_active]).await.expect("save active tasks");

        let adv_res = internal_advance_task_to_stage(&tmp, "CVH-600.1", Some("ready"), "Child finished", None)
            .await
            .expect("advance child to ready");
        assert!(adv_res.contains("УСПЕШНО"));

        let loaded = load_tasks_from_disk(&tmp).await;
        let parent_after = loaded.iter().find(|t| t.id == "CVH-600").expect("parent CVH-600");
        assert_eq!(parent_after.stage_id, "ready");
        assert_eq!(parent_after.status.as_deref(), Some("done"));
        assert!(parent_after.completed_at.is_some(), "Parent completed_at must be Some when completed");
        assert_eq!(parent_after.waiting_for_subtask_id, None, "Parent waiting_for_subtask_id must be None after advance to ready");

        let child_after = loaded.iter().find(|t| t.id == "CVH-600.1").expect("child CVH-600.1");
        assert_eq!(child_after.status.as_deref(), Some("done"));
        assert!(child_after.completed_at.is_some(), "Child completed_at must be Some when completed");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_move_task_to_plan_stage_autorun_and_running_status() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_plan_autorun_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        // Scaffold minimal boards.json with development board containing backlog, plan (persona planner), dev, and ready
        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "plan", "title": "Планирование", "persona": "planner", "auto_run": true },
                        { "id": "dev", "title": "В работе", "persona": "cvh-dev", "auto_run": true },
                        { "id": "ready", "title": "Готово", "terminal": true }
                    ]
                }
            ]
        });
        let boards_path = tmp.join(".cvh").join("boards.json");
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;
        tokio::fs::write(&boards_path, serde_json::to_string_pretty(&boards_cfg).unwrap())
            .await
            .expect("write boards.json");

        // Create task in backlog
        let task = internal_create_task(
            &tmp,
            "Plan Task".to_string(),
            "Move to plan test".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-101".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        assert_eq!(task.stage_id, "backlog");
        assert_ne!(task.status.as_deref(), Some("running"));

        // Move task to 'plan' column
        let moved = internal_move_task_stage(
            &tmp,
            "CVH-101",
            "plan",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move task to plan must succeed");

        assert_eq!(moved.stage_id, "plan", "Task must move to 'plan'");
        assert_ne!(moved.stage_id, "ready", "Task must NEVER falsely move to 'ready'");
        assert_eq!(
            moved.status.as_deref(),
            Some("running"),
            "Task moving to stage with persona 'planner' and auto_run=true must enter 'running' status!"
        );
        assert_ne!(moved.status.as_deref(), Some("done"), "Task status must NOT be 'done'");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_move_task_to_plan_from_pipeline_when_not_in_board() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_plan_pipeline_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        // Boards config does NOT contain 'plan' column
        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "dev", "title": "В работе", "persona": "cvh-dev", "auto_run": true },
                        { "id": "ready", "title": "Готово", "terminal": true }
                    ]
                }
            ]
        });
        tokio::fs::write(
            tmp.join(".cvh").join("boards.json"),
            serde_json::to_string_pretty(&boards_cfg).unwrap(),
        )
        .await
        .expect("write boards.json");

        // Pipeline config DOES contain 'plan' column
        let pipeline_cfg = serde_json::json!({
            "id": "development",
            "title": "Pipeline",
            "columns": [
                { "id": "backlog", "title": "Бэклог" },
                { "id": "plan", "title": "Планирование", "persona": "planner", "auto_run": true },
                { "id": "dev", "title": "В работе", "persona": "cvh-dev", "auto_run": true },
                { "id": "ready", "title": "Готово", "terminal": true }
            ]
        });
        tokio::fs::write(
            tmp.join(".cvh").join("pipeline.json"),
            serde_json::to_string_pretty(&pipeline_cfg).unwrap(),
        )
        .await
        .expect("write pipeline.json");

        let _task = internal_create_task(
            &tmp,
            "Pipeline Plan Task".to_string(),
            "Move to plan from pipeline".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-102".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        // Move task to 'plan' column which only exists in pipeline.json
        let moved = internal_move_task_stage(
            &tmp,
            "CVH-102",
            "plan",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move task to plan from pipeline must succeed");

        assert_eq!(moved.stage_id, "plan", "Task must move to 'plan' from pipeline");
        assert_ne!(moved.stage_id, "ready", "Task must NEVER falsely fall back to 'ready'");
        assert_eq!(moved.status.as_deref(), Some("running"));

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_move_task_to_non_existent_stage_returns_error_not_ready() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_non_existent_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог" },
                        { "id": "dev", "title": "В работе" },
                        { "id": "ready", "title": "Готово", "terminal": true }
                    ]
                }
            ]
        });
        tokio::fs::write(
            tmp.join(".cvh").join("boards.json"),
            serde_json::to_string_pretty(&boards_cfg).unwrap(),
        )
        .await
        .expect("write boards.json");

        let _task = internal_create_task(
            &tmp,
            "NonExistent Stage Task".to_string(),
            "Move to non_existent".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-103".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        // Attempt move to 'non_existent' column
        let res = internal_move_task_stage(
            &tmp,
            "CVH-103",
            "non_existent",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await;

        assert!(res.is_err(), "Moving to non-existent stage must fail!");
        let err = res.unwrap_err();
        assert!(
            err.contains("Target stage 'non_existent' not found in active board or pipeline columns. Available:"),
            "Error message must specify available stages and not silently fall back to ready! Got: {}",
            err
        );

        // Verify task on disk is still in backlog and NOT in ready / done
        let loaded = load_tasks_from_disk(&tmp).await;
        let t = loaded.iter().find(|t| t.id == "CVH-103").expect("task exists");
        assert_eq!(t.stage_id, "backlog", "Task must remain in backlog");
        assert_ne!(t.stage_id, "ready", "Task must NEVER be placed into ready");
        assert_ne!(t.status.as_deref(), Some("done"), "Task status must NOT be done");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_get_task_git_diff_subtask_inherits_parent_worktree() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_diff_subtask_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        // 1. Create a fake worktree directory on disk for parent task CVH-999
        let wt_dir = tmp.join("worktrees").join("CVH-999");
        tokio::fs::create_dir_all(&wt_dir).await.expect("create fake worktree dir");

        // 2. Parent task has worktree_path and branch set
        let parent = Task {
            id: "CVH-999".to_string(),
            title: "Parent Task".to_string(),
            description: "Parent desc".to_string(),
            stage_id: "plan".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: Some("worktrees/CVH-999".to_string()),
            branch: Some("feat/CVH-999".to_string()),
            verdict: None,
            custom_flow: None,
            status: Some("waiting_subtask".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: Some("CVH-999.1".to_string()),
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
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        // 3. Child subtask CVH-999.1 has NO worktree_path and NO branch
        let child = Task {
            id: "CVH-999.1".to_string(),
            title: "Child in dev".to_string(),
            description: "Child desc".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: Some("CVH-999".to_string()),
            subtask_index: Some(1),
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
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        save_tasks_to_disk(&tmp, &[parent, child]).await.expect("save tasks to disk");

        // Call resolve_task_worktree for the subtask CVH-999.1
        let (resolved_task, workdir, branch) = crate::commands::git::resolve_task_worktree(&tmp, "CVH-999.1", None)
            .await
            .expect("resolve_task_worktree must succeed for subtask");

        // Assert workdir is inside worktrees/CVH-999 and NOT falling back to tmp (root)
        assert_eq!(workdir, wt_dir, "Subtask workdir must inherit parent worktree path and not fall back to root repo");
        assert_eq!(resolved_task.worktree_path.as_deref(), Some("worktrees/CVH-999"), "Subtask worktree_path must be auto-healed");
        assert_eq!(resolved_task.branch.as_deref(), Some("feat/CVH-999"), "Subtask branch must be auto-healed from parent");
        assert_ne!(branch, "main", "Branch must never be main when isolated worktree exists");

        // Verify task persisted in SQLite/disk now has worktree_path and branch
        let loaded = load_tasks_from_disk(&tmp).await;
        let loaded_child = loaded.iter().find(|t| t.id == "CVH-999.1").expect("child exists");
        assert_eq!(loaded_child.worktree_path.as_deref(), Some("worktrees/CVH-999"));
        assert_eq!(loaded_child.branch.as_deref(), Some("feat/CVH-999"));

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_resolve_task_worktree_filesystem_heuristic_heals_parent_and_child() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_diff_heuristic_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        // Create worktrees/CVH-888 on disk physically
        let wt_dir = tmp.join("worktrees").join("CVH-888");
        tokio::fs::create_dir_all(&wt_dir).await.expect("create fake worktree dir");

        // Both parent and child have worktree_path: None and branch: None
        let parent = Task {
            id: "CVH-888".to_string(),
            title: "Parent Wiped".to_string(),
            description: "desc".to_string(),
            stage_id: "plan".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("waiting_subtask".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: Some("CVH-888.1".to_string()),
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
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };
        let child = Task {
            id: "CVH-888.1".to_string(),
            title: "Child in dev".to_string(),
            description: "desc".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: Some("CVH-888".to_string()),
            subtask_index: Some(1),
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
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        save_tasks_to_disk(&tmp, &[parent, child]).await.expect("save tasks");

        let (resolved_task, workdir, branch) = crate::commands::git::resolve_task_worktree(&tmp, "CVH-888.1", None)
            .await
            .expect("resolve_task_worktree must heal from filesystem heuristic");

        assert_eq!(workdir, wt_dir);
        assert_eq!(resolved_task.worktree_path.as_deref(), Some("worktrees/CVH-888"));
        assert_eq!(resolved_task.branch.as_deref(), Some("feat/CVH-888"));
        assert_ne!(branch, "main", "Branch must not be main when worktree is resolved");

        // Verify parent is also healed on disk
        let loaded = load_tasks_from_disk(&tmp).await;
        let loaded_parent = loaded.iter().find(|t| t.id == "CVH-888").expect("parent exists");
        assert_eq!(loaded_parent.worktree_path.as_deref(), Some("worktrees/CVH-888"));
        assert_eq!(loaded_parent.branch.as_deref(), Some("feat/CVH-888"));

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_save_tasks_preserves_worktree_path_and_branch_from_client_wipes() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_preserve_wt_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        let task = Task {
            id: "CVH-777".to_string(),
            title: "Task with WT".to_string(),
            description: "desc".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("development".to_string()),
            worktree_path: Some("worktrees/CVH-777".to_string()),
            branch: Some("feat/CVH-777".to_string()),
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
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };
        save_tasks_to_disk(&tmp, &[task]).await.expect("initial save");

        let state = AppState::new(tmp.clone());

        // Client sends updated task with worktree_path: None and branch: None
        let client_task = Task {
            id: "CVH-777".to_string(),
            title: "Task with WT Updated Title".to_string(),
            description: "desc".to_string(),
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
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        internal_save_tasks(&tmp, vec![client_task], Some(&state.tasks)).await.expect("internal_save_tasks");

        let loaded = load_tasks_from_disk(&tmp).await;
        let loaded_task = loaded.iter().find(|t| t.id == "CVH-777").expect("task exists");
        assert_eq!(loaded_task.title, "Task with WT Updated Title");
        assert_eq!(loaded_task.worktree_path.as_deref(), Some("worktrees/CVH-777"), "worktree_path must NOT be wiped by client save");
        assert_eq!(loaded_task.branch.as_deref(), Some("feat/CVH-777"), "branch must NOT be wiped by client save");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_completed_at_lifecycle_on_stage_transitions() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_completed_at_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "dev", "title": "В работе", "auto_run": false },
                        { "id": "ready", "title": "Готово", "terminal": true }
                    ]
                }
            ]
        });
        tokio::fs::write(
            tmp.join(".cvh").join("boards.json"),
            serde_json::to_string_pretty(&boards_cfg).unwrap(),
        )
        .await
        .expect("write boards.json");

        let task = internal_create_task(
            &tmp,
            "CompletedAt Lifecycle Task".to_string(),
            "Testing completed_at tracking".to_string(),
            "dev".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-999".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        assert_eq!(task.completed_at, None, "Task in dev must have completed_at None");

        // 1. Move to terminal 'ready'
        let moved_to_ready = internal_move_task_stage(
            &tmp,
            "CVH-999",
            "ready",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move to ready");

        assert_eq!(moved_to_ready.stage_id, "ready");
        assert_eq!(moved_to_ready.status.as_deref(), Some("done"));
        assert!(moved_to_ready.completed_at.is_some(), "Task in ready must have completed_at timestamp");

        // 2. Move from 'ready' back to 'dev' (rework)
        let moved_back_to_dev = internal_move_task_stage(
            &tmp,
            "CVH-999",
            "dev",
            None,
            None,
            None,
            None,
            false,
            Some("development".to_string()),
        )
        .await
        .expect("move back to dev");

        assert_eq!(moved_back_to_dev.stage_id, "dev");
        assert_eq!(moved_back_to_dev.completed_at, None, "Task moving out of terminal must reset completed_at to None");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_advance_task_sets_subtask_completed_at() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_subtask_completed_at_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let _ = tokio::fs::create_dir_all(tmp.join(".cvh")).await;

        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "plan", "title": "План", "auto_run": false },
                        { "id": "dev", "title": "В работе", "auto_run": false },
                        { "id": "ready", "title": "Готово", "terminal": true }
                    ]
                }
            ]
        });
        tokio::fs::write(
            tmp.join(".cvh").join("boards.json"),
            serde_json::to_string_pretty(&boards_cfg).unwrap(),
        )
        .await
        .expect("write boards.json");

        let _parent = internal_create_task(
            &tmp,
            "Parent Subtask CompletedAt Test".to_string(),
            "Testing subtask completed_at".to_string(),
            "plan".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some("CVH-777".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create parent task");

        let sub1 = internal_create_subtask(
            &tmp,
            "CVH-777",
            Some("plan"),
            Some("Parent Subtask CompletedAt Test (План)"),
            Some("Planning..."),
            None,
        )
        .await
        .expect("create sub1");

        assert_eq!(sub1.completed_at, None);

        // Advance subtask to dev (non-terminal transition)
        let adv_res = internal_advance_task_to_stage(&tmp, &sub1.id, Some("dev"), "Plan completed", None)
            .await
            .expect("advance to dev");
        assert!(adv_res.contains("УСПЕШНО"));

        let loaded = load_tasks_from_disk(&tmp).await;
        let sub1_after = loaded.iter().find(|t| t.id == sub1.id).expect("sub1 loaded");
        assert_eq!(sub1_after.status.as_deref(), Some("done"));
        assert!(sub1_after.completed_at.is_some(), "Subtask 1 completed_at must be populated after advance");

        // Find the created subtask 2 in dev
        let sub2 = loaded.iter().find(|t| t.parent_id.as_deref() == Some("CVH-777") && t.stage_id == "dev").expect("sub2");
        assert_eq!(sub2.completed_at, None);

        // Advance subtask 2 to ready (terminal transition)
        let adv_terminal = internal_advance_task_to_stage(&tmp, &sub2.id, Some("ready"), "Implementation finished", None)
            .await
            .expect("advance to ready");
        assert!(adv_terminal.contains("УСПЕШНО"));

        let loaded_final = load_tasks_from_disk(&tmp).await;
        let sub2_final = loaded_final.iter().find(|t| t.id == sub2.id).expect("sub2 final");
        assert_eq!(sub2_final.status.as_deref(), Some("done"));
        assert!(sub2_final.completed_at.is_some(), "Subtask 2 completed_at must be populated after terminal advance");

        let parent_final = loaded_final.iter().find(|t| t.id == "CVH-777").expect("parent final");
        assert_eq!(parent_final.status.as_deref(), Some("done"));
        assert!(parent_final.completed_at.is_some(), "Parent task completed_at must be populated after terminal advance");

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_advance_task_auto_completes_unfinished_todos() {
        let base = PathBuf::from("target").join(format!("cvh_auto_complete_todos_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let dev_board = crate::config::BoardsConfig {
            active_board_id: "development".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "development".to_string(),
                name: "Development".to_string(),
                columns: vec![
                    test_col("backlog"),
                    test_col("dev"),
                    test_col("ready"),
                ],
            }],
        };
        crate::config::save_boards_config(&base, &dev_board).await.expect("save boards");

        let task_id = "CVH-TODO-1";
        let initial_todos = vec![
            crate::agent::TodoItem {
                id: "todo-1".to_string(),
                content: "Implement feature".to_string(),
                status: "in_progress".to_string(),
            },
            crate::agent::TodoItem {
                id: "todo-2".to_string(),
                content: "Run test suite".to_string(),
                status: "pending".to_string(),
            },
        ];

        let task = Task {
            id: task_id.to_string(),
            title: "Task with unfinished todos".to_string(),
            description: "Testing fallback auto-completion of todos".to_string(),
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
            todos: Some(initial_todos.clone()),
            priority: None,
            tags: None,
            current_tokens: None,
            skip_worktree: Some(true),
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            stage_entered_at: None,
            completed_at: None,
            position: None,
            hook_states: None,
        };

        save_tasks_to_disk(&base, &[task]).await.expect("save task");
        crate::db::db_save_todos(&base, task_id, &initial_todos).expect("save initial todos to db");

        // Verify todos before advance
        let pre_task = crate::db::db_get_task(&base, task_id).expect("load task").expect("task exists");
        let pre_todos = pre_task.todos.expect("pre todos exist");
        assert_eq!(pre_todos.len(), 2);
        assert_eq!(pre_todos[0].status, "in_progress");
        assert_eq!(pre_todos[1].status, "pending");

        // Advance task to next stage (from dev to ready)
        let adv_res = internal_advance_task_to_stage(
            &base,
            task_id,
            Some("ready"),
            "Completed all work and verified",
            None,
        )
        .await
        .expect("internal_advance_task_to_stage must succeed");

        assert!(adv_res.contains("УСПЕШНО"));

        // Verify task in memory/disk after advance
        let disk_tasks = load_tasks_from_disk(&base).await;
        let advanced_task = disk_tasks.iter().find(|t| t.id == task_id).expect("task on disk");
        assert_eq!(advanced_task.stage_id, "ready");
        let disk_todos = advanced_task.todos.as_ref().expect("disk todos present");
        assert_eq!(disk_todos.len(), 2);
        assert_eq!(disk_todos[0].status, "completed");
        assert_eq!(disk_todos[1].status, "completed");

        // Verify task and todos directly in SQLite
        let db_task = crate::db::db_get_task(&base, task_id).expect("db_get_task").expect("task in db");
        let db_todos = db_task.todos.as_ref().expect("db todos present");
        assert_eq!(db_todos.len(), 2);
        assert_eq!(db_todos[0].status, "completed");
        assert_eq!(db_todos[1].status, "completed");

        // Also test idempotency: calling auto_complete_task_todos again returns false
        let mut task_clone = db_task.clone();
        let changed_again = auto_complete_task_todos(&base, &mut task_clone, None);
        assert!(!changed_again, "auto_complete_task_todos must be idempotent and return false when already completed");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_stage_entered_at_persistence_and_stage_transitions() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_stage_entered_at_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&base).await;
        let _ = tokio::fs::create_dir_all(base.join(".cvh")).await;

        let boards_cfg = serde_json::json!({
            "active_board_id": "development",
            "boards": [
                {
                    "id": "development",
                    "name": "Разработка",
                    "columns": [
                        { "id": "backlog", "title": "Бэклог", "auto_run": false },
                        { "id": "dev", "title": "В работе", "auto_run": false },
                        { "id": "ready", "title": "Готово", "terminal": true }
                    ]
                }
            ]
        });
        tokio::fs::write(
            base.join(".cvh").join("boards.json"),
            serde_json::to_string_pretty(&boards_cfg).unwrap(),
        )
        .await
        .expect("write boards.json");

        let task_id = "CVH-998";
        let task = internal_create_task(
            &base,
            "Timer Test Task".to_string(),
            "Testing stage_entered_at persistence".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            None,
            Some(task_id.to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("create task");

        // 1. Initial creation must set stage_entered_at
        assert!(task.stage_entered_at.is_some(), "New task must have stage_entered_at set");
        let initial_stage_entered_at = task.stage_entered_at.unwrap();
        assert!(initial_stage_entered_at > 0);

        // 2. Direct SQLite load must match
        let db_task = crate::db::db_get_task(&base, task_id)
            .expect("db_get_task")
            .expect("task must exist in SQLite");
        assert_eq!(db_task.stage_entered_at, Some(initial_stage_entered_at));

        // 3. save_tasks from client (even with stage_entered_at: None) must preserve existing timestamp if stage unchanged
        let mut client_task = db_task.clone();
        client_task.stage_entered_at = None;
        client_task.title = "Updated Title".to_string();
        internal_save_tasks(&base, vec![client_task], None)
            .await
            .expect("save_tasks");

        let db_after_save = crate::db::db_get_task(&base, task_id)
            .expect("db_get_task")
            .expect("task exists");
        assert_eq!(
            db_after_save.stage_entered_at,
            Some(initial_stage_entered_at),
            "save_tasks must preserve stage_entered_at when stage has not changed"
        );
        assert_eq!(db_after_save.title, "Updated Title");

        // 4. Moving to a new stage ('dev') MUST update stage_entered_at
        tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
        let moved_task = internal_move_task_stage(
            &base,
            task_id,
            "dev",
            None,
            Some("running".to_string()),
            None,
            None,
            false,
            None,
        )
        .await
        .expect("move to dev");

        assert_eq!(moved_task.stage_id, "dev");
        assert!(moved_task.stage_entered_at.is_some());
        let dev_stage_entered_at = moved_task.stage_entered_at.unwrap();
        assert!(
            dev_stage_entered_at > initial_stage_entered_at,
            "stage_entered_at must be updated when stage changes"
        );

        // 5. Updating status within the same stage ('dev') must NOT reset stage_entered_at
        tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
        let same_stage_update = internal_move_task_stage(
            &base,
            task_id,
            "dev",
            None,
            Some("idle".to_string()),
            None,
            None,
            false,
            None,
        )
        .await
        .expect("update status in same stage");

        assert_eq!(same_stage_update.stage_id, "dev");
        assert_eq!(
            same_stage_update.stage_entered_at,
            Some(dev_stage_entered_at),
            "stage_entered_at must NOT change when staying in the same stage"
        );

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_create_task_model_and_task_id_order_regression() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cvh_test_model_order_{}", uuid::Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&tmp).await;

        save_task_sequence(&tmp, 1).await.expect("save initial sequence");

        let task = internal_create_task(
            &tmp,
            "Regression Test Task".to_string(),
            "Assert model and task_id parameters do not get swapped".to_string(),
            "backlog".to_string(),
            Some("development".to_string()),
            Some(true),
            Some("gemini-3.8-flash-high".to_string()),
            None,
            None,
            None,
            None,
        )
        .await
        .expect("create task with model and no task_id must succeed");

        assert!(
            task.id.starts_with("CVH-"),
            "Task ID must start with 'CVH-', but was '{}'",
            task.id
        );
        assert_ne!(
            task.id, "gemini-3.8-flash-high",
            "Task ID must NOT be the model name 'gemini-3.8-flash-high'"
        );
        assert_eq!(
            task.model,
            Some("gemini-3.8-flash-high".to_string()),
            "Task model must be preserved as Some('gemini-3.8-flash-high')"
        );

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }
