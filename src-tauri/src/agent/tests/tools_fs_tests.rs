use super::*;
use crate::config::ToolPolicyConfig;
use std::sync::Arc;

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
