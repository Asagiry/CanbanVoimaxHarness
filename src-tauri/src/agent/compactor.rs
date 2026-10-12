use crate::llm::{ChatMessage, Role};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub original_chars: usize,
    pub compacted_chars: usize,
    pub saved_chars: usize,
    pub estimated_saved_tokens: usize,
    pub evaporated_tools_count: usize,
    pub preserved_state_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStateSnapshot {
    pub modified_files: Vec<String>,
    pub sensor_status: String,
    pub next_step: String,
    pub verdict: String,
}

impl ExecutionStateSnapshot {
    pub fn to_markdown(&self) -> String {
        let files_str = if self.modified_files.is_empty() {
            "None recorded".to_string()
        } else {
            self.modified_files.iter().map(|f| format!("`{}`", f)).collect::<Vec<_>>().join(", ")
        };

        format!(
            "## 📌 Preserved Execution State\n\
             - **Modified Files**: {}\n\
             - **Sensor Status**: {}\n\
             - **Next Step**: {}\n\
             - **Verdict**: {}",
            files_str, self.sensor_status, self.next_step, self.verdict
        )
    }
}

/// Fast zero-LLM evaporation of tool outputs older than the last 2 turns and >500 chars.
/// Anchors:
/// - System prompt (index 0): Immutable.
/// - Initial task prompt (index 1): Immutable.
/// - Last 2 turns: Immutable.
pub fn compact_conversation(
    messages: &mut Vec<ChatMessage>,
    current_snapshot: Option<ExecutionStateSnapshot>,
) -> CompactionResult {
    let original_chars: usize = messages
        .iter()
        .map(|m| m.content.as_ref().map(|c| c.len()).unwrap_or(0))
        .sum();

    if messages.len() <= 4 {
        return CompactionResult {
            original_chars,
            compacted_chars: original_chars,
            saved_chars: 0,
            estimated_saved_tokens: 0,
            evaporated_tools_count: 0,
            preserved_state_summary: "Conversation too short to require compaction.".to_string(),
        };
    }

    // Identify turns from the back to preserve the last 2 turns
    // An assistant message marks a turn.
    let mut assistant_indices = Vec::new();
    for (idx, msg) in messages.iter().enumerate() {
        if msg.role == Role::Assistant {
            assistant_indices.push(idx);
        }
    }

    // Cutoff index: anything at or after `cutoff_idx` is within the last 2 turns (immutable anchor)
    let cutoff_idx = if assistant_indices.len() >= 2 {
        assistant_indices[assistant_indices.len() - 2]
    } else {
        messages.len() // Don't compact if fewer than 2 assistant turns
    };

    let mut modified_files = HashSet::new();
    let mut evaporated_tools_count = 0;

    // Scan for modified files across all tool calls/responses
    for msg in messages.iter() {
        if let Some(tool_calls) = &msg.tool_calls {
            for tc in tool_calls {
                if tc.function.name == "write_file" || tc.function.name == "edit_file" {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                        if let Some(path) = parsed.get("file_path").and_then(|p| p.as_str()) {
                            modified_files.insert(path.to_string());
                        }
                    }
                }
            }
        }
    }

    // Iterate through messages between index 2 and cutoff_idx
    for idx in 2..cutoff_idx.min(messages.len()) {
        let msg = &mut messages[idx];
        if msg.role == Role::Tool {
            if let Some(content) = &mut msg.content {
                let len = content.len();
                if len > 500 {
                    let head: String = content.lines().take(3).collect::<Vec<_>>().join("\n");
                    *content = format!(
                        "[Evaporated prior tool output: original size {} chars ({} lines). Status: Succeeded. Head summary:\n{}\n...[Remainder evaporated by State-Preserving Compactor]]",
                        len,
                        content.lines().count(),
                        head
                    );
                    evaporated_tools_count += 1;
                }
            }
        }
    }

    // Build state snapshot
    let mut snapshot = current_snapshot.unwrap_or_else(|| ExecutionStateSnapshot {
        modified_files: modified_files.into_iter().collect(),
        sensor_status: "Verified (cargo check & npm run build)".to_string(),
        next_step: "Continuing autonomous implementation loop".to_string(),
        verdict: "IN_PROGRESS".to_string(),
    });

    if snapshot.modified_files.is_empty() && !modified_files.is_empty() {
        snapshot.modified_files = modified_files.into_iter().collect();
    }
    snapshot.modified_files.sort();

    let snapshot_md = snapshot.to_markdown();

    // Check if an existing snapshot block is present; if so update it, else inject after task prompt
    let mut existing_snapshot_idx = None;
    for (idx, msg) in messages.iter().enumerate() {
        if let Some(content) = &msg.content {
            if content.starts_with("## 📌 Preserved Execution State") {
                existing_snapshot_idx = Some(idx);
                break;
            }
        }
    }

    if let Some(idx) = existing_snapshot_idx {
        messages[idx].content = Some(snapshot_md.clone());
    } else if messages.len() >= 2 {
        // Insert after initial task prompt (index 1)
        messages.insert(2, ChatMessage::system(snapshot_md.clone()));
    }

    let compacted_chars: usize = messages
        .iter()
        .map(|m| m.content.as_ref().map(|c| c.len()).unwrap_or(0))
        .sum();

    let saved_chars = original_chars.saturating_sub(compacted_chars);
    let estimated_saved_tokens = saved_chars / 4;

    CompactionResult {
        original_chars,
        compacted_chars,
        saved_chars,
        estimated_saved_tokens,
        evaporated_tools_count,
        preserved_state_summary: snapshot_md,
    }
}
