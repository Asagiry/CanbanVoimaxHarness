//! Unit tests for config modules.

use super::*;
use std::path::{Path, PathBuf};


    #[test]
    fn test_llm_settings_serde_camel_and_snake_case() {
        // Test snake_case deserialization
        let json_snake = r#"{
            "provider": "open-router",
            "api_key": "sk-secret-123",
            "base_url": "https://openrouter.ai/api/v1",
            "model": "anthropic/claude-3-7-sonnet",
            "temperature": 0.5
        }"#;
        let s1: LlmSettings = serde_json::from_str(json_snake).expect("Failed to deserialize snake_case");
        assert_eq!(s1.provider, "open-router");
        assert_eq!(s1.api_key, "sk-secret-123");
        assert_eq!(s1.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(s1.model, "anthropic/claude-3-7-sonnet");
        assert_eq!(s1.temperature, Some(0.5));

        // Test camelCase deserialization
        let json_camel = r#"{
            "provider": "antigravity",
            "apiKey": "sk-local-abc",
            "baseUrl": "http://127.0.0.1:19387/api/ai",
            "model": "gemini-3.8-flash-high"
        }"#;
        let s2: LlmSettings = serde_json::from_str(json_camel).expect("Failed to deserialize camelCase");
        assert_eq!(s2.provider, "antigravity");
        assert_eq!(s2.api_key, "sk-local-abc");
        assert_eq!(s2.base_url, "http://127.0.0.1:19387/api/ai");
        assert_eq!(s2.model, "gemini-3.8-flash-high");
        assert_eq!(s2.temperature, None);
    }

    #[tokio::test]
    async fn test_scaffold_cvh_project_pure_empty_structure() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("Failed to create test dir in target");

        scaffold_cvh_project(&base_dir).await.expect("Scaffold failed");

        let cvh = base_dir.join(".cvh");
        assert!(cvh.exists());
        assert!(cvh.join("agents").is_dir());
        assert!(cvh.join("skills").is_dir());
        assert!(cvh.join("settings").is_dir());

        // Verify cvh-dev modular rules are seeded
        let cvh_dev = cvh.join("agents").join("cvh-dev");
        assert!(cvh_dev.join("persona.md").is_file());
        assert!(cvh_dev.join("rules").join("boards.md").is_file());
        assert!(cvh_dev.join("rules").join("agents.md").is_file());
        assert!(cvh_dev.join("rules").join("hooks.md").is_file());
        assert!(cvh_dev.join("rules").join("pipeline-setup.md").is_file());

        // Ensure no legacy flat persona files exist in agents/
        let mut read_agents = tokio::fs::read_dir(cvh.join("agents")).await.expect("Read dir failed");
        while let Ok(Some(entry)) = read_agents.next_entry().await {
            assert!(entry.path().is_dir(), "Expected only directories in agents/, found: {:?}", entry.path());
        }

        // Verify .cvh/rules/ exists as an empty directory without hardcoded CVH maps
        let rules_dir = cvh.join("rules");
        assert!(rules_dir.is_dir());
        assert!(!rules_dir.join("backend_map.md").exists(), "Fresh scaffold must NOT seed CVH backend_map.md");
        assert!(!rules_dir.join("frontend_map.md").exists(), "Fresh scaffold must NOT seed CVH frontend_map.md");

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_scaffold_creates_empty_rules_directory_without_cvh_maps() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_rules_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("Failed to create test dir");

        scaffold_cvh_project(&base_dir).await.expect("Scaffold failed");

        let rules_dir = base_dir.join(".cvh").join("rules");
        assert!(rules_dir.is_dir(), ".cvh/rules/ directory must be created");

        // Count entries in .cvh/rules/
        let mut entries = tokio::fs::read_dir(&rules_dir).await.expect("read_dir failed");
        let mut count = 0;
        while let Ok(Some(_)) = entries.next_entry().await {
            count += 1;
        }
        assert_eq!(count, 0, ".cvh/rules/ must be initially empty without hardcoded maps");

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_ensure_cvh_dev_scaffold_seeding() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_dev_scaffold_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("create test dir");

        // Run ensure_cvh_dev_scaffold on empty directory
        ensure_cvh_dev_scaffold(&base_dir).await.expect("ensure_cvh_dev_scaffold failed");

        let cvh_dev = base_dir.join(".cvh").join("agents").join("cvh-dev");
        assert!(cvh_dev.join("persona.md").is_file());
        assert!(cvh_dev.join("setting").is_file());
        assert!(cvh_dev.join("rules").join("boards.md").is_file());
        assert!(cvh_dev.join("rules").join("agents.md").is_file());
        assert!(cvh_dev.join("rules").join("hooks.md").is_file());

        // Check context_window in setting
        let setting_str = tokio::fs::read_to_string(cvh_dev.join("setting")).await.expect("read setting");
        let setting_val: serde_json::Value = serde_json::from_str(&setting_str).expect("parse setting");
        assert_eq!(setting_val.get("context_window").and_then(|v| v.as_u64()), Some(200000));

        // Calling it again is idempotent
        ensure_cvh_dev_scaffold(&base_dir).await.expect("ensure_cvh_dev_scaffold re-run failed");
        assert!(cvh_dev.join("persona.md").is_file());

        // Verify .cvh/rules/ exists and contains no hardcoded maps
        let rules_dir = base_dir.join(".cvh").join("rules");
        assert!(rules_dir.is_dir());
        assert!(!rules_dir.join("backend_map.md").exists());
        assert!(!rules_dir.join("frontend_map.md").exists());

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_scaffold_minimal_backlog_ready_only() {
        // Fresh scaffold: pipeline.json holds exactly 3 columns:
        // backlog (no persona, first), dev (cvh-dev persona), ready (terminal, last).
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_min_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("test dir");

        scaffold_cvh_project(&base_dir).await.expect("Scaffold failed");

        let cvh = base_dir.join(".cvh");
        let pipe_raw = tokio::fs::read_to_string(cvh.join("pipeline.json"))
            .await
            .expect("pipeline.json missing");
        let pipe: PipelineConfig = serde_json::from_str(&pipe_raw).expect("pipeline parse");
        assert_eq!(pipe.columns.len(), 3, "minimal scaffold must have exactly 3 columns");
        assert_eq!(pipe.columns[0].id, "backlog");
        assert_eq!(pipe.columns[0].title, "Backlog");
        assert_eq!(pipe.columns[0].persona, None, "backlog must have no persona");
        assert_eq!(pipe.columns[1].id, "dev");
        assert_eq!(pipe.columns[1].title, "CVH Development");
        assert_eq!(pipe.columns[1].persona.as_deref(), Some("cvh-dev"), "dev must have cvh-dev persona");
        assert_eq!(pipe.columns[2].id, "ready");
        assert_eq!(pipe.columns[2].title, "Ready");
        assert_eq!(pipe.columns[2].terminal, Some(true), "ready must be terminal last");

        // Verify boards.json is also created and has 3 columns with cvh-dev
        let boards_raw = tokio::fs::read_to_string(cvh.join("boards.json"))
            .await
            .expect("boards.json missing");
        let boards_cfg: BoardsConfig = serde_json::from_str(&boards_raw).expect("boards parse");
        assert_eq!(boards_cfg.boards.len(), 1);
        assert_eq!(boards_cfg.boards[0].columns.len(), 3);
        assert_eq!(boards_cfg.boards[0].columns[1].persona.as_deref(), Some("cvh-dev"));

        // cvh-dev setting must have context_window 200000
        let cvh_dev = cvh.join("agents").join("cvh-dev");
        let setting_raw = tokio::fs::read_to_string(cvh_dev.join("setting"))
            .await
            .expect("setting missing");
        let parsed_setting: AgentSetting = serde_json::from_str(&setting_raw).expect("setting parse");
        assert_eq!(parsed_setting.context_window, Some(200000));

        // cvh-dev seeded with modular rules, no flat personas
        assert!(cvh_dev.join("persona.md").is_file());
        assert!(cvh_dev.join("rules").join("boards.md").is_file());
        let mut rd = tokio::fs::read_dir(cvh.join("agents")).await.expect("agents dir");
        while let Ok(Some(entry)) = rd.next_entry().await {
            assert!(entry.path().is_dir(), "Expected only directories in agents/, found: {:?}", entry.path());
        }

        // remaining scaffold files/dirs in place
        assert!(cvh.join("dialogs").is_dir());
        assert!(cvh.join("config.json").is_file());
        assert!(!cvh.join("tasks.json").exists(), "tasks.json must not be scaffolded");
        assert!(tool_policy_path(&base_dir).is_file());

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_load_agent_setting_broken_falls_back_to_default() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_set_{}", uuid::Uuid::new_v4()));
        let dir = base.join(".cvh").join("agents").join("dev");
        tokio::fs::create_dir_all(&dir).await.expect("test dir");

        // Missing → default.
        let missing = load_agent_setting(&base, "dev").await;
        assert_eq!(missing, AgentSetting::default());

        // Broken JSON → default.
        tokio::fs::write(dir.join("setting"), "{ not valid json {{{").await.expect("seed broken");
        let broken = load_agent_setting(&base, "dev").await;
        assert_eq!(broken, AgentSetting::default());

        // Valid snake_case + camelCase aliases + icon/color.
        tokio::fs::write(
            dir.join("setting"),
            r##"{"model": "m1", "temperature": 0.7, "contextWindow": 64000, "allowedTools": ["read_file", "bash"], "icon": "Sparkles", "color": "#14b8a6"}"##,
        )
        .await
        .expect("seed valid");
        let valid = load_agent_setting(&base, "dev").await;
        assert_eq!(valid.model.as_deref(), Some("m1"));
        assert_eq!(valid.temperature, Some(0.7));
        assert_eq!(valid.context_window, Some(64000));
        assert_eq!(
            valid.allowed_tools,
            Some(vec!["read_file".to_string(), "bash".to_string()])
        );
        assert_eq!(valid.icon.as_deref(), Some("Sparkles"));
        assert_eq!(valid.color.as_deref(), Some("#14b8a6"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[test]
    fn test_agent_allows_tool_intersection() {
        // None → unrestricted.
        assert!(agent_allows_tool(None, "read_file"));
        assert!(agent_allows_tool(None, "bash"));
        // Listed tools pass, others are cut (intersection semantics).
        let allowed = vec!["read_file".to_string(), "execute_command".to_string()];
        assert!(agent_allows_tool(Some(&allowed), "read_file"));
        assert!(agent_allows_tool(Some(&allowed), "execute_command"));
        assert!(agent_allows_tool(Some(&allowed), "bash"), "bash alias must match execute_command");
        assert!(!agent_allows_tool(Some(&allowed), "write_file"));
        assert!(!agent_allows_tool(Some(&allowed), "web_search"));
        // Empty list → nothing allowed.
        let empty: Vec<String> = vec![];
        assert!(!agent_allows_tool(Some(&empty), "read_file"));
    }

    #[test]
    fn test_resolve_hook_path_primary_then_agent_fallback() {
        let base = std::path::PathBuf::from("target").join(format!("cvh_hook_{}", uuid::Uuid::new_v4()));
        let hooks = base.join(".cvh").join("hooks");
        let agent_hooks = base.join(".cvh").join("agents").join("dev").join("hooks");
        std::fs::create_dir_all(&hooks).expect("hooks dir");
        std::fs::create_dir_all(&agent_hooks).expect("agent hooks dir");
        std::fs::write(hooks.join("on_enter.bat"), "rem").expect("seed primary");
        std::fs::write(agent_hooks.join("on_enter.bat"), "rem agent").expect("seed agent");

        // Primary exists → primary wins.
        let p1 = resolve_hook_path(&base, Some("dev"), ".cvh/hooks/on_enter.bat");
        assert_eq!(p1, base.join(".cvh").join("hooks").join("on_enter.bat"));

        // Primary missing → agent fallback by basename.
        std::fs::remove_file(hooks.join("on_enter.bat")).expect("remove primary");
        let p2 = resolve_hook_path(&base, Some("dev"), ".cvh/hooks/on_enter.bat");
        assert_eq!(p2, agent_hooks.join("on_enter.bat"));

        // No persona → configured path unchanged.
        let p3 = resolve_hook_path(&base, None, ".cvh/hooks/on_enter.bat");
        assert_eq!(p3, base.join(".cvh").join("hooks").join("on_enter.bat"));

        let _ = std::fs::remove_dir_all(&base);
    }

    fn policy_with_dev_exec_disabled() -> ToolPolicyConfig {
        let mut policy = ToolPolicyConfig::default();
        let dev = policy.stage_overrides.entry("dev".to_string()).or_default();
        dev.insert("execute_command".to_string(), false);
        dev.insert("read_file".to_string(), false);
        policy
    }

    #[test]
    fn test_disabled_tool_blocked_for_stage_including_alias() {
        // Regression: a matrix with execute_command=false / read_file=false
        // for a stage must deny both the canonical name and the SWE-bench
        // alias ("bash"), exactly as the execute_tool gate evaluates them.
        let policy = policy_with_dev_exec_disabled();

        assert!(!is_tool_allowed(&policy, Some("dev"), "execute_command"));
        assert!(!is_tool_allowed(&policy, Some("dev"), "bash"));
        assert!(!is_tool_allowed(&policy, Some("dev"), "read_file"));
        // Untouched stage entries keep working (review allows execute_command
        // by default).
        assert!(is_tool_allowed(&policy, Some("review"), "execute_command"));
        // str_replace alias normalizes to edit_file (disabled in review).
        assert!(!is_tool_allowed(&policy, Some("review"), "str_replace"));
        // ask_user alias normalizes to question (enabled in dev and review by default).
        assert!(is_tool_allowed(&policy, Some("dev"), "question"));
        assert!(is_tool_allowed(&policy, Some("dev"), "ask_user"));
    }

    #[test]
    fn test_strip_frontmatter_tools() {
        let content_single = "---\nname: dev\ntools: [read_file, glob]\nmodel: inherit\n---\n# Dev persona";
        let stripped = strip_frontmatter_tools(content_single);
        assert!(!stripped.contains("tools:"));
        assert!(stripped.contains("name: dev"));
        assert!(stripped.contains("model: inherit"));
        assert!(stripped.contains("# Dev persona"));

        let content_multi = "---\nname: dev\ntools:\n  - read_file\n  - glob\nmodel: inherit\n---\n# Dev persona";
        let stripped_multi = strip_frontmatter_tools(content_multi);
        assert!(!stripped_multi.contains("tools:"));
        assert!(!stripped_multi.contains("read_file"));
        assert!(stripped_multi.contains("name: dev"));
        assert!(stripped_multi.contains("model: inherit"));
        assert!(stripped_multi.contains("# Dev persona"));
    }

    #[test]
    fn test_tool_policy_stage_overrides_authoritative() {
        // Single Authoritative Layer: stage_overrides directly dictates tool availability.
        // An explicit stage/role override controls the agent directly.
        let mut policy = ToolPolicyConfig::default();

        // dev explicitly allows read_file
        policy
            .stage_overrides
            .entry("dev".to_string())
            .or_default()
            .insert("read_file".to_string(), true);
        assert!(is_tool_allowed(&policy, Some("dev"), "read_file"));

        // dev explicitly disables read_file
        policy
            .stage_overrides
            .entry("dev".to_string())
            .or_default()
            .insert("read_file".to_string(), false);
        assert!(!is_tool_allowed(&policy, Some("dev"), "read_file"));

        // Fallback for stages with no record: falls back to global_enabled
        policy.global_enabled.insert("glob".to_string(), false);
        assert!(!is_tool_allowed(&policy, Some("no-such-stage"), "glob"));
        assert!(!is_tool_allowed(&policy, None, "glob"));
    }

    #[test]
    fn test_tool_permissions_and_semantics() {
        // Same AND-semantics for the legacy allowlist model:
        // effective = global_ok && stage_ok (missing entries default to true).
        use std::collections::HashMap;

        // global=false + stage=true -> false.
        let mut stage_map = HashMap::new();
        stage_map.insert(
            "dev".to_string(),
            vec!["read_file".to_string(), "execute_command".to_string()],
        );
        let perms = ToolPermissions::new(vec!["execute_command".to_string()], stage_map);
        assert!(!perms.is_tool_allowed("dev", "read_file"));
        // global=true + stage=false (absent from stage list) -> false.
        assert!(!perms.is_tool_allowed("dev", "glob"));
        // Both true -> true (incl. bash alias normalization).
        assert!(perms.is_tool_allowed("dev", "execute_command"));
        assert!(perms.is_tool_allowed("dev", "bash"));
        // No stage record + global=false -> false; unrestricted tool -> true.
        assert!(!perms.is_tool_allowed("no-such-stage", "read_file"));
        assert!(perms.is_tool_allowed("no-such-stage", "execute_command"));

        // Empty global + no stages: fail-open default preserved.
        let open = ToolPermissions::new(vec![], HashMap::new());
        assert!(open.is_tool_allowed("dev", "read_file"));
    }

    #[test]
    fn test_tool_policy_path_canonical() {
        // Single canonical file shared by scaffold, save and load:
        // `<root>/.cvh/settings/tool-policy.json`. No legacy fallbacks.
        let root = Path::new("/repo");
        assert_eq!(
            tool_policy_path(root),
            root.join(".cvh").join("settings").join("tool-policy.json")
        );
    }

    #[tokio::test]
    async fn test_tool_policy_save_load_roundtrip_canonical() {
        // Regression: what the matrix saves is what the agent loop loads,
        // from the canonical settings file.
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_policy_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("test dir");

        save_tool_policies(&base_dir, &policy_with_dev_exec_disabled())
            .await
            .expect("save failed");

        assert!(tool_policy_path(&base_dir).exists());

        let loaded = load_tool_policies(&base_dir).await.expect("load failed");
        assert!(!is_tool_allowed(&loaded, Some("dev"), "execute_command"));
        assert!(!is_tool_allowed(&loaded, Some("dev"), "bash"));
        assert!(!is_tool_allowed(&loaded, Some("dev"), "read_file"));

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[test]
    fn test_default_boards_config() {
        let cfg = default_boards_config();
        assert_eq!(cfg.active_board_id, "development");
        assert_eq!(cfg.boards.len(), 1);
        assert_eq!(cfg.boards[0].id, "development");
        assert_eq!(cfg.boards[0].name, "cvh-board");
        assert_eq!(cfg.boards[0].columns.len(), 3);
    }

    #[test]
    fn test_get_board_template_columns_modeler() {
        let cols = get_board_template_columns("modeler");
        assert_eq!(cols.len(), 4);
        assert!(cols.iter().all(|c| c.model_override.is_none()));
        assert_eq!(cols[0].id, "concept");
        assert_eq!(cols[0].persona.as_deref(), Some("concept-artist"));
        assert_eq!(cols[1].id, "modeling");
        assert_eq!(cols[1].persona.as_deref(), Some("modeler"));
        assert_eq!(cols[2].id, "texturing");
        assert_eq!(cols[2].persona.as_deref(), Some("texture-artist"));
        assert_eq!(cols[3].id, "ready");
        assert_eq!(cols[3].terminal, Some(true));
    }

    #[test]
    fn test_get_board_template_columns_canonical_default() {
        let cols = get_board_template_columns("canonical");
        assert_eq!(cols.len(), 3);
        assert!(cols.iter().all(|c| c.model_override.is_none()));
        assert_eq!(cols[0].id, "backlog");
        assert_eq!(cols[0].title, "Backlog");
        assert_eq!(cols[0].persona, None);
        assert_eq!(cols[1].id, "dev");
        assert_eq!(cols[1].title, "CVH Development");
        assert_eq!(cols[1].persona.as_deref(), Some("cvh-dev"));
        assert_eq!(cols[2].id, "ready");
        assert_eq!(cols[2].title, "Ready");
        assert_eq!(cols[2].persona, None);
        assert_eq!(cols[2].terminal, Some(true));
    }

    #[tokio::test]
    async fn test_boards_config_persistence_and_sync() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_test_boards_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base_dir).await.expect("test dir");

        let mut cfg = default_boards_config();
        let modeler_cols = get_board_template_columns("modeler");
        cfg.boards.push(BoardDef {
            id: "modeler".to_string(),
            name: "3D Моделирование".to_string(),
            columns: modeler_cols,
        });
        cfg.active_board_id = "modeler".to_string();

        save_boards_config(&base_dir, &cfg).await.expect("save boards");

        // Verify boards.json exists
        assert!(base_dir.join(".cvh").join("boards.json").exists());
        // Verify pipeline.json synchronized with active board (normalized with mandatory backlog and ready)
        let pipe = load_pipeline_config(&base_dir).await.expect("load pipeline");
        assert_eq!(pipe.title, "3D Моделирование");
        assert_eq!(pipe.columns.len(), 4);

        // Load boards config back
        let reloaded = load_boards_config(&base_dir).await.expect("load boards");
        assert_eq!(reloaded.active_board_id, "modeler");
        assert_eq!(reloaded.boards.len(), 2);

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[tokio::test]
    async fn test_boards_config_fallback_to_pipeline() {
        // boards.json missing → single-board config derived from pipeline.json
        // columns (kept fallback, not a migration: no files are moved).
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_test_boards_fb_{}", uuid::Uuid::new_v4()));
        let cvh = base_dir.join(".cvh");
        tokio::fs::create_dir_all(&cvh).await.expect("test dir");

        let pipeline_only = serde_json::json!({
            "id": "custom-engineering",
            "title": "Мой пайплайн",
            "columns": [
                {
                    "id": "col1",
                    "title": "Стадия 1",
                    "color": "#64748b",
                    "icon": "Inbox",
                    "persona": null,
                    "contextWindow": 128000,
                    "modelOverride": null,
                    "reasoningEffort": null,
                    "temperature": null,
                    "gatekeeperSensor": null,
                    "allowedTools": null,
                    "hooks": {},
                    "routes": null,
                    "autoRun": null,
                    "terminal": null
                }
            ]
        });

        atomic_write_file(&cvh.join("pipeline.json"), &serde_json::to_string_pretty(&pipeline_only).unwrap()).await.unwrap();

        let boards_cfg = load_boards_config(&base_dir).await.expect("load boards from pipeline fallback");
        assert_eq!(boards_cfg.active_board_id, "development");
        assert_eq!(boards_cfg.boards.len(), 1);
        assert_eq!(boards_cfg.boards[0].name, "Мой пайплайн");
        assert_eq!(boards_cfg.boards[0].columns.len(), 1);

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[test]
    fn test_ensure_project_gitignore_creates_missing_file() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_gi_create_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        ensure_project_gitignore(&base_dir).expect("ensure gitignore failed");

        let gi_path = base_dir.join(".gitignore");
        assert!(gi_path.is_file(), ".gitignore must be created if missing");

        let content = std::fs::read_to_string(&gi_path).expect("read gitignore");
        assert!(content.contains(CVH_GITIGNORE_HEADER));
        for entry in CVH_RUNTIME_GITIGNORE_ENTRIES {
            assert!(content.contains(entry), "Missing entry in created gitignore: {}", entry);
        }

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn test_ensure_project_gitignore_appends_to_existing_file() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_gi_append_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        let gi_path = base_dir.join(".gitignore");
        std::fs::write(&gi_path, "/target/\nnode_modules/\n").expect("seed existing");

        ensure_project_gitignore(&base_dir).expect("ensure gitignore failed");

        let content = std::fs::read_to_string(&gi_path).expect("read gitignore");
        assert!(content.starts_with("/target/\nnode_modules/\n"));
        assert!(content.contains(CVH_GITIGNORE_HEADER));
        for entry in CVH_RUNTIME_GITIGNORE_ENTRIES {
            assert!(content.contains(entry), "Missing runtime entry: {}", entry);
        }

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn test_ensure_project_gitignore_idempotent_no_duplicates() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_gi_idempotent_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        ensure_project_gitignore(&base_dir).expect("first run");
        let content_first = std::fs::read_to_string(base_dir.join(".gitignore")).expect("read first");

        ensure_project_gitignore(&base_dir).expect("second run");
        let content_second = std::fs::read_to_string(base_dir.join(".gitignore")).expect("read second");

        assert_eq!(content_first, content_second, "Second run must not modify gitignore");

        let header_count = content_second.matches(CVH_GITIGNORE_HEADER).count();
        assert_eq!(header_count, 1, "Header must appear exactly once");

        for entry in CVH_RUNTIME_GITIGNORE_ENTRIES {
            let entry_count = content_second.lines().filter(|l| l.trim() == *entry).count();
            assert_eq!(entry_count, 1, "Entry {} must appear exactly once", entry);
        }

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn test_ensure_project_gitignore_wildcard_covered() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_gi_wildcard_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        let gi_path = base_dir.join(".gitignore");
        let initial_content = "target/\n.cvh/\nnode_modules/\n";
        std::fs::write(&gi_path, initial_content).expect("seed wildcard");

        ensure_project_gitignore(&base_dir).expect("run on wildcard");

        let content = std::fs::read_to_string(&gi_path).expect("read gitignore");
        assert_eq!(content, initial_content, "Wildcard .cvh/ must prevent modifying gitignore");

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn test_ensure_project_gitignore_partial_preserves_header_and_entries() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_gi_partial_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        let gi_path = base_dir.join(".gitignore");
        let initial = format!(
            "{}\n.cvh/worktrees/\n.cvh/llm.json\n",
            CVH_GITIGNORE_HEADER
        );
        std::fs::write(&gi_path, &initial).expect("seed partial");

        ensure_project_gitignore(&base_dir).expect("run on partial");

        let content = std::fs::read_to_string(&gi_path).expect("read gitignore");
        let header_count = content.matches(CVH_GITIGNORE_HEADER).count();
        assert_eq!(header_count, 1, "Existing header must not be duplicated");

        for entry in CVH_RUNTIME_GITIGNORE_ENTRIES {
            assert!(content.contains(entry), "Entry {} must be present", entry);
        }

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[tokio::test]
    async fn test_init_project_harness_integration() {
        let base_dir = std::path::PathBuf::from("target").join(format!("cvh_init_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).expect("create test dir");

        init_project_harness(&base_dir).await.expect("init project harness failed");

        let gi_path = base_dir.join(".gitignore");
        assert!(gi_path.is_file(), ".gitignore must exist");
        let content = std::fs::read_to_string(&gi_path).expect("read gitignore");
        assert!(content.contains(".cvh/harness.db*"));
        assert!(content.contains(".cvh/spill/"));
        assert!(content.contains(".cvh/worktrees/"));
        assert!(content.contains(".cvh/llm.json"));
        assert!(content.contains(".cvh/tasks.json"));

        assert!(content.contains(".cvh/"));
        let cvh = base_dir.join(".cvh");
        assert!(cvh.is_dir());
        assert!(cvh.join("agents").is_dir());
        assert!(cvh.join("config.json").is_file());
        assert!(cvh.join("pipeline.json").is_file());

        let _ = tokio::fs::remove_dir_all(&base_dir).await;
    }

    #[test]
    fn test_is_binary_or_target_dir_protection() {
        // Must reject Release, Release/Windows, Release/Windows/bin, Release/Windows/installer
        assert!(is_binary_or_target_dir(Path::new("Release")));
        assert!(is_binary_or_target_dir(Path::new("release")));
        assert!(is_binary_or_target_dir(Path::new("Release/Windows")));
        assert!(is_binary_or_target_dir(Path::new("Release/Windows/bin")));
        assert!(is_binary_or_target_dir(Path::new("Release/Windows/installer")));
        assert!(is_binary_or_target_dir(Path::new(r"Release\Windows\bin")));
        assert!(is_binary_or_target_dir(Path::new(r"Release\Windows\installer")));
        assert!(is_binary_or_target_dir(Path::new(r"C:\Users\VibeCode\CanbanVoimaxHarness\Release\Windows\bin")));
        assert!(is_binary_or_target_dir(Path::new(r"C:\Users\VibeCode\CanbanVoimaxHarness\Release\Windows\installer")));

        // Standard build and distribution dirs
        assert!(is_binary_or_target_dir(Path::new("target")));
        assert!(is_binary_or_target_dir(Path::new("target/debug")));
        assert!(is_binary_or_target_dir(Path::new("dist")));
        assert!(is_binary_or_target_dir(Path::new("node_modules")));

        // Valid user project paths must NOT be rejected
        assert!(!is_binary_or_target_dir(Path::new(r"C:\projects\my-cool-app")));
        assert!(!is_binary_or_target_dir(Path::new("src")));
    }

    #[test]
    fn test_is_app_own_directory_protection() {
        // Exe directory of the running test binary has no .git, so it should be blocked
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                assert!(is_app_own_directory(exe_dir));
            }
        }

        // Repositories named CanbanVoimaxHarness or normal user projects must NOT be blocked
        assert!(!is_app_own_directory(Path::new(r"C:\Users\VibeCode\CanbanVoimaxHarness")));
        assert!(!is_app_own_directory(Path::new("CanbanVoimaxHarness")));
        assert!(!is_app_own_directory(Path::new(r"C:\projects\web-dashboard")));

        // A directory containing .git is a source repository and must NEVER be blocked
        let temp_dir = std::env::temp_dir().join(format!("cvh_git_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let _ = std::fs::create_dir_all(temp_dir.join(".git"));
        assert!(!is_app_own_directory(&temp_dir));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_actual_repo_root_rejects_forbidden_dirs() {
        assert_eq!(resolve_actual_repo_root(Path::new("")), PathBuf::new());
        assert_eq!(resolve_actual_repo_root(Path::new("Release/Windows/bin")), PathBuf::new());
        assert_eq!(resolve_actual_repo_root(Path::new("Release/Windows/installer")), PathBuf::new());
        assert_eq!(resolve_actual_repo_root(Path::new(r"C:\Users\VibeCode\CanbanVoimaxHarness")), PathBuf::from(r"C:\Users\VibeCode\CanbanVoimaxHarness"));
        assert_eq!(resolve_actual_repo_root(Path::new(r"C:\Users\VibeCode\CanbanVoimaxHarness\Release\Windows\bin")), PathBuf::new());
    }

    #[test]
    fn test_hook_config_serde_and_helpers() {
        // String format
        let raw_str = r#""hooks/create_worktree.bat""#;
        let hook_str: HookConfig = serde_json::from_str(raw_str).expect("deserialize string hook");
        assert_eq!(hook_str.script(), "hooks/create_worktree.bat");
        assert_eq!(hook_str.description(), None);
        assert_eq!(serde_json::to_string(&hook_str).unwrap(), raw_str);

        // Detailed object format with description
        let raw_detailed = r#"{"script":"hooks/on_enter.bat","description":"Initializes the worktree"}"#;
        let hook_det: HookConfig = serde_json::from_str(raw_detailed).expect("deserialize detailed hook");
        assert_eq!(hook_det.script(), "hooks/on_enter.bat");
        assert_eq!(hook_det.description(), Some("Initializes the worktree"));

        // Detailed object format with alias 'path' and no description
        let raw_alias = r#"{"path":"hooks/on_enter.bat"}"#;
        let hook_alias: HookConfig = serde_json::from_str(raw_alias).expect("deserialize alias hook");
        assert_eq!(hook_alias.script(), "hooks/on_enter.bat");
        assert_eq!(hook_alias.description(), None);

        // Detailed object format with alias 'command'
        let raw_cmd = r#"{"command":"hooks/on_enter.bat"}"#;
        let hook_cmd: HookConfig = serde_json::from_str(raw_cmd).expect("deserialize command hook");
        assert_eq!(hook_cmd.script(), "hooks/on_enter.bat");
        assert_eq!(hook_cmd.description(), None);

        // Deserializing within PipelineColumn
        let col_json = r#"{
            "id": "backlog",
            "title": "Backlog",
            "hooks": {
                "on_leave": {
                    "script": ".cvh/hooks/create_worktree.bat",
                    "description": "Creates worktree"
                },
                "on_enter": ".cvh/hooks/on_enter.bat"
            }
        }"#;
        let col: PipelineColumn = serde_json::from_str(col_json).expect("deserialize column with hooks");
        assert_eq!(col.hooks.get("on_leave").unwrap().script(), ".cvh/hooks/create_worktree.bat");
        assert_eq!(col.hooks.get("on_leave").unwrap().description(), Some("Creates worktree"));
        assert_eq!(col.hooks.get("on_enter").unwrap().script(), ".cvh/hooks/on_enter.bat");
        assert_eq!(col.hooks.get("on_enter").unwrap().description(), None);
    }
