use super::*;
use std::path::PathBuf;


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


