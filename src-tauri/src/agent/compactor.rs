use crate::llm::{ChatMessage, Role};

/// State-Preserving Context Compactor for CanbanVoimaxHarness.
/// Truncates redundant tool execution logs older than 3 turns into lightweight tags
/// while preserving Immutable Anchors (System Prompt, initial prompt, last 2 turns).
pub struct ContextCompactor;

impl ContextCompactor {
    /// Evaporates tool outputs older than 3 turns (> 500 chars) into lightweight tags:
    /// `[Output evaporated: {chars} bytes. Exit: 0. Tool: {name}]`
    ///
    /// Preserves Immutable Anchors:
    /// - System Prompt (Index 0)
    /// - Initial User Prompt (Index 1)
    /// - Last 2 turns (last 4 messages: assistant + tool pairs)
    pub fn evaporate_tool_logs(messages: &mut [ChatMessage]) -> usize {
        let total = messages.len();
        // If conversation is short (<= 6 messages), keep everything intact
        if total <= 6 {
            return 0;
        }

        // Immutable Anchors:
        // Anchors start: indices 0 (System) and 1 (Initial Prompt)
        // Anchors end: last 4 messages (representing the last 2 complete turns)
        let end_cutoff = total.saturating_sub(4);
        let mut evaporated_bytes = 0;

        for (idx, msg) in messages.iter_mut().enumerate() {
            // Protect immutable anchors: system prompt (0), initial prompt (1), and last 2 turns
            if idx < 2 || idx >= end_cutoff {
                continue;
            }

            // Evaporate tool response messages if content > 500 chars
            if msg.role == Role::Tool {
                if let Some(ref content) = msg.content {
                    if content.len() > 500 {
                        let original_len = content.len();
                        let tool_name = msg.name.as_deref().unwrap_or("tool");
                        let tag = format!(
                            "[Output evaporated: {} bytes. Exit: 0. Tool: {}]",
                            original_len, tool_name
                        );
                        evaporated_bytes += original_len.saturating_sub(tag.len());
                        msg.content = Some(tag);
                    }
                }
            }
        }

        evaporated_bytes
    }

    /// Structured State Snapshot block generator (`## 📌 Preserved Execution State`)
    pub fn generate_state_snapshot(
        task_id: &str,
        branch: &str,
        modified_files: &[String],
        completed_tools: &[String],
    ) -> String {
        let files_str = if modified_files.is_empty() {
            "- None recorded".to_string()
        } else {
            modified_files
                .iter()
                .map(|f| format!("- `{}`", f))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let tools_str = if completed_tools.is_empty() {
            "- None executed".to_string()
        } else {
            completed_tools
                .iter()
                .map(|t| format!("- {}", t))
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "## 📌 Preserved Execution State\n\
            - **Task ID**: {}\n\
            - **Active Branch**: `{}`\n\
            - **Modified Workspace Files**:\n{}\n\
            - **Recent Completed Tools**:\n{}\n",
            task_id, branch, files_str, tools_str
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaporate_tool_logs_preserves_anchors() {
        let mut messages = vec![
            ChatMessage::system("System prompt anchor"),
            ChatMessage::user("Initial user prompt anchor"),
            ChatMessage::tool_response("call_1", "a".repeat(1000)),
            ChatMessage::assistant(Some("Intermediate step".into()), None),
            ChatMessage::tool_response("call_2", "b".repeat(1000)),
            ChatMessage::assistant(Some("Turn N-1".into()), None),
            ChatMessage::tool_response("call_3", "c".repeat(1000)),
        ];

        let saved = ContextCompactor::evaporate_tool_logs(&mut messages);
        assert!(saved > 0);
        // System and User prompt anchors intact
        assert_eq!(messages[0].content.as_deref(), Some("System prompt anchor"));
        assert_eq!(messages[1].content.as_deref(), Some("Initial user prompt anchor"));
        // Old tool call (index 2) evaporated
        assert!(messages[2].content.as_ref().unwrap().starts_with("[Output evaporated: 1000 bytes."));
        // Last 4 messages protected
        assert_eq!(messages[6].content.as_ref().unwrap().len(), 1000);
    }
}
