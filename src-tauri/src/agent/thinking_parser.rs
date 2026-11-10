/// Checks if an opening tag candidate (<thinking> or <think>) at `pos` is a genuine model
/// control tag rather than markdown inline code (`<thinking>`), code fence, or word substring.
pub fn is_valid_opening_tag(buffer: &str, pos: usize, tag_len: usize) -> bool {
    if !buffer.is_char_boundary(pos) || !buffer.is_char_boundary(pos + tag_len) || pos + tag_len > buffer.len() {
        return false;
    }

    // 1. Check character immediately preceding the tag
    if pos > 0 {
        if let Some(prev_char) = buffer.get(..pos).and_then(|s| s.chars().next_back()) {
            // If preceded by a backtick (e.g. `<thinking>` or ```<thinking>), it's markdown inline code or fence
            if prev_char == '`' {
                return false;
            }
            // If preceded by an alphanumeric char or underscore, it's inside an identifier/word (e.g. var<thinking>)
            if prev_char.is_alphanumeric() || prev_char == '_' {
                return false;
            }
        }
    }

    // 2. Check character immediately following the tag
    if pos + tag_len < buffer.len() {
        if let Some(next_char) = buffer.get(pos + tag_len..).and_then(|s| s.chars().next()) {
            // If immediately followed by a backtick (e.g. <thinking>`), it's markdown
            if next_char == '`' {
                return false;
            }
        }
    }

    // 3. Check if inside unclosed inline backtick code on the current line
    if let Some(before) = buffer.get(..pos) {
        let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        if let Some(line_before) = before.get(line_start..) {
            let backticks_before = line_before.chars().filter(|&c| c == '`').count();
            if backticks_before % 2 == 1 {
                return false;
            }
        }
    }

    true
}

/// Checks if a closing tag candidate (</thinking> or </think>) at `pos` is a genuine model
/// control tag rather than markdown inline code (`</thinking>`).
pub fn is_valid_closing_tag(buffer: &str, pos: usize, tag_len: usize) -> bool {
    if !buffer.is_char_boundary(pos) || !buffer.is_char_boundary(pos + tag_len) || pos + tag_len > buffer.len() {
        return false;
    }

    if pos > 0 {
        if let Some(prev_char) = buffer.get(..pos).and_then(|s| s.chars().next_back()) {
            if prev_char == '`' {
                return false;
            }
        }
    }

    if pos + tag_len < buffer.len() {
        if let Some(next_char) = buffer.get(pos + tag_len..).and_then(|s| s.chars().next()) {
            if next_char == '`' {
                return false;
            }
        }
    }

    if let Some(before) = buffer.get(..pos) {
        let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        if let Some(line_before) = before.get(line_start..) {
            let backticks_before = line_before.chars().filter(|&c| c == '`').count();
            if backticks_before % 2 == 1 {
                return false;
            }
        }
    }

    true
}

/// Helper to check if a potential partial closing tag prefix at `pos` could be a valid closing tag
pub fn is_valid_partial_closing_tag(buffer: &str, pos: usize) -> bool {
    if !buffer.is_char_boundary(pos) || pos > buffer.len() {
        return false;
    }
    if pos > 0 {
        if let Some(prev_char) = buffer.get(..pos).and_then(|s| s.chars().next_back()) {
            if prev_char == '`' {
                return false;
            }
        }
    }
    if let Some(before) = buffer.get(..pos) {
        let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        if let Some(line_before) = before.get(line_start..) {
            let backticks_before = line_before.chars().filter(|&c| c == '`').count();
            if backticks_before % 2 == 1 {
                return false;
            }
        }
    }
    true
}

/// Helper to check if a potential partial tag prefix at `pos` could be a valid opening tag
pub fn is_valid_partial_opening_tag(buffer: &str, pos: usize) -> bool {
    if !buffer.is_char_boundary(pos) || pos > buffer.len() {
        return false;
    }
    if pos > 0 {
        if let Some(prev_char) = buffer.get(..pos).and_then(|s| s.chars().next_back()) {
            if prev_char == '`' || prev_char.is_alphanumeric() || prev_char == '_' {
                return false;
            }
        }
    }
    if let Some(before) = buffer.get(..pos) {
        let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        if let Some(line_before) = before.get(line_start..) {
            let backticks_before = line_before.chars().filter(|&c| c == '`').count();
            if backticks_before % 2 == 1 {
                return false;
            }
        }
    }
    true
}

#[derive(Debug, Default)]
pub struct ThinkingStreamParser {
    in_thinking: bool,
    in_backtick: bool,
    buffer: String,
}

impl ThinkingStreamParser {
    pub fn new() -> Self {
        Self {
            in_thinking: false,
            in_backtick: false,
            buffer: String::new(),
        }
    }

    /// Process an incoming token chunk. Returns a list of (is_thought, text) chunks.
    pub fn push(&mut self, chunk: &str) -> Vec<(bool, String)> {
        self.buffer.push_str(chunk);
        let mut outputs = Vec::new();

        loop {
            if !self.in_thinking {
                // Look for opening tag <thinking> or <think>
                let mut earliest_open: Option<(usize, usize)> = None;
                let mut search_pos = 0;
                while search_pos < self.buffer.len() {
                    let sub = match self.buffer.get(search_pos..) {
                        Some(s) => s,
                        None => break,
                    };
                    let open_thinking = sub.find("<thinking>").map(|p| search_pos + p);
                    let open_think = sub.find("<think>").map(|p| search_pos + p);
                    let open_thought = sub.find("<thought>").map(|p| search_pos + p);
                    let open_reasoning = sub.find("<reasoning>").map(|p| search_pos + p);

                    let mut cands: Vec<(usize, usize)> = Vec::new();
                    if let Some(p) = open_thinking {
                        cands.push((p, "<thinking>".len()));
                    }
                    if let Some(p) = open_think {
                        cands.push((p, "<think>".len()));
                    }
                    if let Some(p) = open_thought {
                        cands.push((p, "<thought>".len()));
                    }
                    if let Some(p) = open_reasoning {
                        cands.push((p, "<reasoning>".len()));
                    }
                    let next_cand = cands.into_iter().min_by_key(|c| c.0);

                    match next_cand {
                        Some((pos, tag_len)) => {
                            let backticks_before = self.buffer.get(..pos).map(|s| s.chars().filter(|&c| c == '`').count()).unwrap_or(0);
                            let effective_in_backtick = (self.in_backtick as usize + backticks_before) % 2 == 1;

                            if !effective_in_backtick && is_valid_opening_tag(&self.buffer, pos, tag_len) {
                                earliest_open = Some((pos, tag_len));
                                break;
                            } else {
                                let advance = self.buffer.get(pos..).and_then(|s| s.chars().next()).map(|c| c.len_utf8()).unwrap_or(1);
                                search_pos = pos + advance;
                            }
                        }
                        None => break,
                    }
                }

                if let Some((pos, tag_len)) = earliest_open {
                    if pos > 0 {
                        let text = self.buffer.get(..pos).unwrap_or("").to_string();
                        for c in text.chars() {
                            if c == '`' {
                                self.in_backtick = !self.in_backtick;
                            }
                        }
                        outputs.push((false, text));
                    }
                    self.in_thinking = true;
                    self.in_backtick = false;
                    self.buffer = self.buffer.get(pos + tag_len..).unwrap_or("").to_string();
                    continue;
                }

                // If not found, check if end of buffer could be a partial opening tag
                let mut partial_len = 0;
                if !self.in_backtick {
                    for tag in &["<thinking>", "<think>", "<thought>", "<reasoning>"] {
                        for len in 1..tag.len() {
                            if self.buffer.ends_with(&tag[..len]) && len > partial_len {
                                let pos = self.buffer.len() - len;
                                let backticks_before = self.buffer.get(..pos).map(|s| s.chars().filter(|&c| c == '`').count()).unwrap_or(0);
                                let effective_in_backtick = (self.in_backtick as usize + backticks_before) % 2 == 1;
                                if !effective_in_backtick && is_valid_partial_opening_tag(&self.buffer, pos) {
                                    partial_len = len;
                                }
                            }
                        }
                    }
                }

                if partial_len > 0 {
                    let safe_len = self.buffer.len() - partial_len;
                    if safe_len > 0 && self.buffer.is_char_boundary(safe_len) {
                        let text = self.buffer.get(..safe_len).unwrap_or("").to_string();
                        for c in text.chars() {
                            if c == '`' {
                                self.in_backtick = !self.in_backtick;
                            }
                        }
                        outputs.push((false, text));
                        self.buffer = self.buffer.get(safe_len..).unwrap_or("").to_string();
                    }
                    break;
                } else {
                    if !self.buffer.is_empty() {
                        let text = std::mem::take(&mut self.buffer);
                        for c in text.chars() {
                            if c == '`' {
                                self.in_backtick = !self.in_backtick;
                            }
                        }
                        outputs.push((false, text));
                    }
                    break;
                }
            } else {
                // In thinking: look for closing tag </thinking> or </think>
                let mut earliest_close: Option<(usize, usize)> = None;
                let mut search_pos = 0;
                while search_pos < self.buffer.len() {
                    let sub = match self.buffer.get(search_pos..) {
                        Some(s) => s,
                        None => break,
                    };
                    let close_thinking = sub.find("</thinking>").map(|p| search_pos + p);
                    let close_think = sub.find("</think>").map(|p| search_pos + p);
                    let close_thought = sub.find("</thought>").map(|p| search_pos + p);
                    let close_reasoning = sub.find("</reasoning>").map(|p| search_pos + p);

                    let mut cands: Vec<(usize, usize)> = Vec::new();
                    if let Some(p) = close_thinking {
                        cands.push((p, "</thinking>".len()));
                    }
                    if let Some(p) = close_think {
                        cands.push((p, "</think>".len()));
                    }
                    if let Some(p) = close_thought {
                        cands.push((p, "</thought>".len()));
                    }
                    if let Some(p) = close_reasoning {
                        cands.push((p, "</reasoning>".len()));
                    }
                    let next_cand = cands.into_iter().min_by_key(|c| c.0);

                    match next_cand {
                        Some((pos, tag_len)) => {
                            if is_valid_closing_tag(&self.buffer, pos, tag_len) {
                                earliest_close = Some((pos, tag_len));
                                break;
                            } else {
                                let advance = self.buffer.get(pos..).and_then(|s| s.chars().next()).map(|c| c.len_utf8()).unwrap_or(1);
                                search_pos = pos + advance;
                            }
                        }
                        None => break,
                    }
                }

                if let Some((pos, tag_len)) = earliest_close {
                    if pos > 0 {
                        let text = self.buffer.get(..pos).unwrap_or("").to_string();
                        outputs.push((true, text));
                    }
                    self.in_thinking = false;
                    self.in_backtick = false;
                    self.buffer = self.buffer.get(pos + tag_len..).unwrap_or("").to_string();
                    continue;
                }

                // If not found, check if end of buffer could be a partial closing tag
                let mut partial_len = 0;
                for tag in &["</thinking>", "</think>", "</thought>", "</reasoning>"] {
                    for len in 2..tag.len() {
                        if self.buffer.ends_with(&tag[..len]) && len > partial_len {
                            let pos = self.buffer.len() - len;
                            if is_valid_partial_closing_tag(&self.buffer, pos) {
                                partial_len = len;
                            }
                        }
                    }
                }

                if partial_len > 0 {
                    let safe_len = self.buffer.len() - partial_len;
                    if safe_len > 0 && self.buffer.is_char_boundary(safe_len) {
                        let text = self.buffer.get(..safe_len).unwrap_or("").to_string();
                        outputs.push((true, text));
                        self.buffer = self.buffer.get(safe_len..).unwrap_or("").to_string();
                    }
                    break;
                } else {
                    if !self.buffer.is_empty() {
                        outputs.push((true, std::mem::take(&mut self.buffer)));
                    }
                    break;
                }
            }
        }

        outputs
    }

    /// Flush any remaining buffered characters at the end of the stream
    pub fn flush(&mut self) -> Vec<(bool, String)> {
        if self.buffer.is_empty() {
            Vec::new()
        } else {
            let item = (self.in_thinking, std::mem::take(&mut self.buffer));
            vec![item]
        }
    }
}
