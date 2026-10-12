use crate::llm::ChatMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateSnapshot {
    #[serde(default)]
    pub modified_files: Vec<String>,
    #[serde(default)]
    pub sensor_status: String,
    #[serde(default)]
    pub next_step: String,
    #[serde(default)]
    pub verdict: Option<String>,
}

impl StateSnapshot {
    pub fn to_markdown(&self) -> String {
        let mut out = String::from("## 📌 Preserved Execution State\n");
        let files = if self.modified_files.is_empty() {
            "None".to_string()
        } else {
            self.modified_files.join(", ")
        };
        out.push_str(&format!("- Modified Files: {}\n", files));
        out.push_str(&format!(
            "- Sensor Status: {}\n",
            if self.sensor_status.is_empty() {
                "UNKNOWN"
            } else {
                &self.sensor_status
            }
        ));
        out.push_str(&format!(
            "- Next Step: {}\n",
            if self.next_step.is_empty() {
                "Continue execution"
            } else {
                &self.next_step
            }
        ));
        if let Some(ref v) = self.verdict {
            out.push_str(&format!("- Verdict: {}\n", v));
        }
        out
    }
}

/// Evaporates large tool outputs older than 3 turns while preserving immutable anchors
/// and injecting a structured execution state snapshot.
pub fn compact_conversation_history(
    messages: &mut [ChatMessage],
    current_turn: usize,
    snapshot: Option<&StateSnapshot>,
) -> usize {
    if messages.len() <= 3 {
        return 0;
    }

    let mut evaporated_count = 0;
    let total_len = messages.len();

    // Immutable Anchors:
    // Index 0: System prompt
    // Index 1: Initial task prompt
    // Last 2 turns: Protected from any compression
    // In our ReAct loop, 1 turn typically generates 1 assistant message + N tool messages.
    // We protect the last 4-6 messages (representing roughly the last 2 turns).
    let protected_tail_start = if total_len > 6 {
        total_len - 6
    } else {
        total_len
    };

    // Messages between index 2 and protected_tail_start are candidates if older than 3 turns
    if current_turn > 3 {
        for idx in 2..protected_tail_start {
            if let Some(msg) = messages.get_mut(idx) {
                // If it is a tool response and length > 500 characters
                if msg.role == crate::llm::Role::Tool {
                    if let Some(ref content) = msg.content {
                        if content.len() > 500 && !content.starts_with("[Evaporated:") {
                            let original_chars = content.len();
                            msg.content = Some(format!(
                                "[Evaporated: tool output previously verified, {} chars compressed]",
                                original_chars
                            ));
                            evaporated_count += 1;
                        }
                    }
                }
            }
        }
    }

    // If snapshot is provided, ensure index 1 (initial task prompt) contains preserved execution state
    if let Some(snap) = snapshot {
        if messages.len() > 1 {
            let snap_md = snap.to_markdown();
            if let Some(ref orig) = messages[1].content {
                if !orig.contains("## 📌 Preserved Execution State") {
                    messages[1].content = Some(format!("{}\n\n{}", orig, snap_md));
                }
            }
        }
    }

    evaporated_count
}

/// Estimate tokens from character count (~4 chars per token)
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// Calculate total estimated tokens across all messages
pub fn estimate_conversation_tokens(messages: &[ChatMessage]) -> usize {
    let mut total_chars = 0;
    for m in messages {
        if let Some(ref c) = m.content {
            total_chars += c.len();
        }
        if let Some(ref tc) = m.tool_calls {
            for t in tc {
                total_chars += t.function.name.len() + t.function.arguments.len();
            }
        }
    }
    estimate_tokens(&"x".repeat(total_chars))
}
