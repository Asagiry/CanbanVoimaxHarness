use super::*;
use crate::llm::ToolCall;

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


    #[test]
    fn test_reconstruct_messages_from_frontend_dialog_items() {
        let items = vec![ActivityItem {
            id: "call_123".to_string(),
            item_type: "tool".to_string(),
            timestamp: "12:00:00".to_string(),
            content: None,
            tool: Some(serde_json::json!({
                "id": "call_123",
                "toolName": "subagent_bg",
                "params": {
                    "role": "developer",
                    "title": "Test"
                },
                "stdout": "Success report"
            })),
            duration_ms: Some(150.0),
            images: None,
        }];

        let reconstructed = reconstruct_messages_from_dialog(
            "SYSTEM PROMPT",
            "INITIAL OBJECTIVE",
            &items,
        );

        // Sequence: [0] System, [1] User, [2] Assistant (ToolCall), [3] Tool response
        assert_eq!(reconstructed.len(), 4);
        assert_eq!(reconstructed[0].role, Role::System);
        assert_eq!(reconstructed[1].role, Role::User);

        // Assistant with tool call
        assert_eq!(reconstructed[2].role, Role::Assistant);
        let tool_calls = reconstructed[2].tool_calls.as_ref().expect("tool_calls present");
        assert_eq!(tool_calls.len(), 1);
        let tc = &tool_calls[0];
        assert_eq!(tc.id, "call_123");
        assert_eq!(tc.function.name, "subagent_bg");
        assert_ne!(tc.function.name, "unknown_tool");

        let parsed_args: serde_json::Value =
            serde_json::from_str(&tc.function.arguments).expect("valid args json");
        assert_eq!(parsed_args["role"], "developer");
        assert_eq!(parsed_args["title"], "Test");

        // Tool response
        assert_eq!(reconstructed[3].role, Role::Tool);
        assert_eq!(reconstructed[3].tool_call_id.as_deref(), Some("call_123"));
        assert_eq!(reconstructed[3].content.as_deref(), Some("Success report"));
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


