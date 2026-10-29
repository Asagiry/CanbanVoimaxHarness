// Text matching, similarity, CRLF normalization, and whitespace-relaxed replacement routines.
// Extracted from src-tauri/src/agent.rs per ub-mode Zero-Risk Pure Extraction.

/// Tokenizes a line into identifier/number words, symbols, and punctuation tokens.
pub fn tokenize_line(s: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = None;
    let mut in_word = false;

    for (idx, ch) in s.char_indices() {
        if ch.is_whitespace() {
            if let Some(st) = start.take() {
                tokens.push(&s[st..idx]);
            }
            in_word = false;
        } else if ch.is_alphanumeric() || ch == '_' {
            if !in_word {
                if let Some(st) = start.take() {
                    tokens.push(&s[st..idx]);
                }
                start = Some(idx);
                in_word = true;
            }
        } else {
            if let Some(st) = start.take() {
                tokens.push(&s[st..idx]);
            }
            let char_len = ch.len_utf8();
            tokens.push(&s[idx..idx + char_len]);
            in_word = false;
        }
    }
    if let Some(st) = start {
        tokens.push(&s[st..]);
    }
    tokens
}

/// Computes token-level Dice/F1 similarity between two line strings (range 0.0 .. 1.0).
pub fn line_similarity(a: &str, b: &str) -> f32 {
    let a_trim = a.trim();
    let b_trim = b.trim();
    if a_trim == b_trim {
        return 1.0;
    }
    if a_trim.is_empty() || b_trim.is_empty() {
        return 0.0;
    }
    let a_tokens = tokenize_line(a_trim);
    let b_tokens = tokenize_line(b_trim);
    if a_tokens.is_empty() || b_tokens.is_empty() {
        return 0.0;
    }
    let n = a_tokens.len();
    let m = b_tokens.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            if a_tokens[i] == b_tokens[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }
    let lcs = dp[n][m];
    (2.0 * lcs as f32) / ((n + m) as f32)
}

/// Generates a human-readable explanation of token differences between expected and actual lines.
pub fn diff_line_tokens(expected: &str, actual: &str) -> String {
    let exp_trim = expected.trim();
    let act_trim = actual.trim();
    if exp_trim == act_trim {
        return "whitespace or indentation mismatch".to_string();
    }
    let exp_tokens = tokenize_line(exp_trim);
    let act_tokens = tokenize_line(act_trim);
    if exp_tokens.is_empty() && act_tokens.is_empty() {
        return "whitespace mismatch".to_string();
    }
    if exp_tokens.is_empty() {
        return format!("expected empty line, found '{}'", act_trim);
    }
    if act_tokens.is_empty() {
        return format!("expected '{}', found empty line in file", exp_trim);
    }

    let n = exp_tokens.len();
    let m = act_tokens.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            if exp_tokens[i] == act_tokens[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut i = n;
    let mut j = m;
    let mut raw_diffs = Vec::new();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && exp_tokens[i - 1] == act_tokens[j - 1] {
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            raw_diffs.push((None, Some(act_tokens[j - 1])));
            j -= 1;
        } else if i > 0 && (j == 0 || dp[i][j - 1] < dp[i - 1][j]) {
            raw_diffs.push((Some(exp_tokens[i - 1]), None));
            i -= 1;
        }
    }
    raw_diffs.reverse();

    let mut diffs = Vec::new();
    let mut k = 0;
    while k < raw_diffs.len() {
        if k + 1 < raw_diffs.len() {
            if let ((Some(exp), None), (None, Some(act))) = (raw_diffs[k], raw_diffs[k + 1]) {
                diffs.push(format!("expected '{}', found '{}'", exp, act));
                k += 2;
                continue;
            }
        }
        if let (Some(e), None) = raw_diffs[k] {
            diffs.push(format!("missing '{}'", e));
            k += 1;
        } else if let (None, Some(a)) = raw_diffs[k] {
            diffs.push(format!("unexpected '{}'", a));
            k += 1;
        } else {
            k += 1;
        }
    }

    if diffs.is_empty() {
        "whitespace or formatting mismatch".to_string()
    } else if diffs.len() > 6 {
        let truncated = diffs[..6].join("; ");
        format!("{}; ... ({} more token diffs)", truncated, diffs.len() - 6)
    } else {
        diffs.join("; ")
    }
}

/// Calculates informative error message when `old_string` is not found in file content.
/// Computes multi-line block similarity and outputs a clear line-by-line diff with token-level diagnostics.
pub fn build_edit_mismatch_error(content: &str, file_path: &str, old_string: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if lines.is_empty() {
        return format!(
            "Edit failed: File '{}' is empty (0 lines). Cannot locate 'old_string'.",
            file_path
        );
    }

    let search_lines: Vec<&str> = old_string.lines().collect();
    if search_lines.is_empty() {
        return format!(
            "Edit failed: 'old_string' is empty. Cannot locate target in '{}'.",
            file_path
        );
    }
    let search_block_len = search_lines.len();

    // Slide window of size search_block_len across file lines to find best matching block
    let mut best_score = 0.0f32;
    let mut best_idx = 0;

    for idx in 0..total_lines {
        let cand_len = search_block_len.min(total_lines - idx);
        let mut score_sum = 0.0f32;
        for k in 0..search_block_len {
            if k < cand_len {
                score_sum += line_similarity(search_lines[k], lines[idx + k]);
            }
        }
        let block_score = score_sum / (search_block_len as f32);
        if block_score > best_score {
            best_score = block_score;
            best_idx = idx;
        }
    }

    // Safety guard: if subsequent lines or tokens differ, never claim 100% similarity!
    let mut all_lines_identical = true;
    let cand_len = search_block_len.min(total_lines.saturating_sub(best_idx));
    if cand_len < search_block_len {
        all_lines_identical = false;
    } else {
        for k in 0..search_block_len {
            if search_lines[k] != lines[best_idx + k] {
                all_lines_identical = false;
                break;
            }
        }
    }
    if !all_lines_identical && best_score >= 0.999 {
        best_score = 0.99;
    }

    let mut msg = format!(
        "Target 'old_string' not found in '{}' (total {} lines).\n",
        file_path, total_lines
    );

    if best_score >= 0.25 {
        let start_line = best_idx + 1;
        let end_line = (best_idx + cand_len).max(start_line);
        let pct = (best_score * 100.0).clamp(0.0, if all_lines_identical { 100.0 } else { 99.0 });

        msg.push_str(&format!(
            "Closest matching candidate block identified at lines {}-{} (block similarity: {:.0}%):\n\n",
            start_line, end_line, pct
        ));

        msg.push_str("--- Line-by-Line Diff ([-] old_string vs [+] file candidate) ---\n");
        for k in 0..search_block_len {
            let exp_line = search_lines[k];
            if k < cand_len {
                let act_line = lines[best_idx + k];
                let file_line_no = best_idx + k + 1;
                if exp_line == act_line {
                    msg.push_str(&format!(
                        "  Line {} (file line {}): EXACT MATCH\n    \"{}\"\n",
                        k + 1, file_line_no, act_line
                    ));
                } else if exp_line.trim() == act_line.trim() {
                    msg.push_str(&format!(
                        "  Line {} (file line {}): WHITESPACE MISMATCH\n    - old_string: \"{}\"\n    + file:       \"{}\"\n    Diff: whitespace or indentation mismatch\n",
                        k + 1, file_line_no, exp_line, act_line
                    ));
                } else {
                    let diff_diag = diff_line_tokens(exp_line, act_line);
                    msg.push_str(&format!(
                        "  Line {} (file line {}): MISMATCH\n    - old_string: \"{}\"\n    + file:       \"{}\"\n    Diff: {}\n",
                        k + 1, file_line_no, exp_line, act_line, diff_diag
                    ));
                }
            } else {
                msg.push_str(&format!(
                    "  Line {}: MISMATCH (file ended before this line)\n    - old_string: \"{}\"\n",
                    k + 1, exp_line
                ));
            }
        }

        msg.push_str("\n--- Candidate Block in File (use exact lines below for edit_file) ---\n");
        let ctx_start = best_idx.saturating_sub(2);
        let ctx_end = (best_idx + cand_len + 2).min(total_lines);
        for (offset, line_content) in lines[ctx_start..ctx_end].iter().enumerate() {
            let line_num = ctx_start + offset + 1;
            let line_no_0 = ctx_start + offset;
            let is_cand = line_no_0 >= best_idx && line_no_0 < best_idx + cand_len;
            let marker = if is_cand { ">>" } else { "  " };
            msg.push_str(&format!("{} {:4} | {}\n", marker, line_num, line_content));
        }

        let read_offset = start_line;
        let read_limit = (cand_len + 4).max(5);
        msg.push_str(&format!(
            "\nRecommendation: Use the exact candidate lines above or inspect with read_file(file_path: \"{}\", offset: {}, limit: {}) to update old_string.",
            file_path, read_offset, read_limit
        ));
    } else {
        msg.push_str("No close match found (similarity < 25%). First 10 lines of file for context:\n");
        let end = 10.min(total_lines);
        for (i, line) in lines[..end].iter().enumerate() {
            msg.push_str(&format!("   {:4} | {}\n", i + 1, line));
        }
        msg.push_str("\nRecommendation: Use read_file to inspect exact lines and retry edit_file with precise context.");
    }

    msg
}

/// Represents a line span with its text and byte boundaries in a file string.
#[derive(Debug, Clone)]
struct LineBoundary<'a> {
    text: &'a str,
    start: usize,
    end_content: usize,
    end_full: usize,
}

fn parse_line_boundaries<'a>(content: &'a str) -> Vec<LineBoundary<'a>> {
    let mut lines = Vec::new();
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut line_start = 0;

    while line_start < len {
        let mut i = line_start;
        while i < len && bytes[i] != b'\n' && bytes[i] != b'\r' {
            i += 1;
        }
        let end_content = i;
        if i < len && bytes[i] == b'\r' {
            i += 1;
        }
        if i < len && bytes[i] == b'\n' {
            i += 1;
        }
        let end_full = i;
        lines.push(LineBoundary {
            text: &content[line_start..end_content],
            start: line_start,
            end_content,
            end_full,
        });
        line_start = end_full;
    }

    lines
}

/// Performs whitespace-relaxed / normalized line matching and replacement.
/// Trims leading and trailing whitespace on each line to locate the target block,
/// while respecting CRLF/LF line ending conventions.
fn replace_whitespace_relaxed(
    current: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
    file_has_crlf: bool,
) -> Result<String, String> {
    let file_lines = parse_line_boundaries(current);
    let search_lines: Vec<&str> = old_string.lines().collect();
    if search_lines.is_empty() || file_lines.len() < search_lines.len() {
        return Err("NOT_FOUND".to_string());
    }

    let m = search_lines.len();
    let search_trimmed: Vec<&str> = search_lines.iter().map(|l| l.trim()).collect();

    // Do not match if search lines are entirely whitespace
    if search_trimmed.iter().all(|l| l.is_empty()) {
        return Err("NOT_FOUND".to_string());
    }

    let mut matches = Vec::new();
    let mut idx = 0;
    while idx <= file_lines.len().saturating_sub(m) {
        let mut matched = true;
        for k in 0..m {
            if file_lines[idx + k].text.trim() != search_trimmed[k] {
                matched = false;
                break;
            }
        }
        if matched {
            matches.push(idx);
            if replace_all {
                idx += m;
                continue;
            }
        }
        idx += 1;
    }

    if matches.is_empty() {
        return Err("NOT_FOUND".to_string());
    }

    if !replace_all && matches.len() > 1 {
        return Err(format!(
            "Ambiguous match: 'old_string' occurs {} times in file (whitespace-relaxed). Set replace_all: true or supply more surrounding lines.",
            matches.len()
        ));
    }

    let old_ends_with_newline = old_string.ends_with('\n') || old_string.ends_with("\r\n");

    // Adapt new_string newline convention to match the file
    let base_new = if file_has_crlf {
        new_string.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        new_string.replace("\r\n", "\n")
    };

    let new_ends_with_newline = base_new.ends_with('\n') || base_new.ends_with("\r\n");
    let replace_includes_newline = old_ends_with_newline || new_ends_with_newline;

    // Collect replacement ranges and replacement strings
    // We sort matches in reverse order so applying edits from bottom-to-top preserves byte offsets.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();

    for &idx in &matches {
        let span_start = file_lines[idx].start;
        let last_line = &file_lines[idx + m - 1];
        let span_end = if replace_includes_newline {
            last_line.end_full
        } else {
            last_line.end_content
        };

        // Determine indentation adaptation:
        let file_indent: String = file_lines[idx]
            .text
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        let old_indent: String = search_lines[0]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        let adapted_new = if file_indent != old_indent && file_indent.starts_with(&old_indent) {
            let extra_indent = &file_indent[old_indent.len()..];
            let nl = if file_has_crlf { "\r\n" } else { "\n" };
            let new_lines: Vec<&str> = base_new.lines().collect();
            let mut rebuilt = Vec::new();
            for l in &new_lines {
                if l.trim().is_empty() {
                    rebuilt.push(l.to_string());
                } else if !l.starts_with(&file_indent) {
                    rebuilt.push(format!("{}{}", extra_indent, l));
                } else {
                    rebuilt.push(l.to_string());
                }
            }
            let mut joined = rebuilt.join(nl);
            if new_ends_with_newline {
                joined.push_str(nl);
            }
            joined
        } else if file_indent != old_indent && old_indent.starts_with(&file_indent) {
            let strip_len = old_indent.len() - file_indent.len();
            let nl = if file_has_crlf { "\r\n" } else { "\n" };
            let new_lines: Vec<&str> = base_new.lines().collect();
            let all_can_strip = new_lines
                .iter()
                .all(|l| l.trim().is_empty() || l.starts_with(&old_indent));
            if all_can_strip {
                let mut rebuilt = Vec::new();
                for l in &new_lines {
                    if l.trim().is_empty() {
                        rebuilt.push(l.to_string());
                    } else {
                        rebuilt.push(l[strip_len..].to_string());
                    }
                }
                let mut joined = rebuilt.join(nl);
                if new_ends_with_newline {
                    joined.push_str(nl);
                }
                joined
            } else {
                base_new.clone()
            }
        } else {
            base_new.clone()
        };

        edits.push((span_start, span_end, adapted_new));
    }

    // Sort edits by span_start descending so replacing tail doesn't invalidate earlier byte offsets
    edits.sort_by_key(|a| std::cmp::Reverse(a.0));

    let mut result = current.to_string();
    for (start, end, rep) in edits {
        if start <= result.len() && end <= result.len() && start <= end {
            result.replace_range(start..end, &rep);
        }
    }

    Ok(result)
}

/// Performs string replacement with CRLF vs LF newline normalization and resilient line matching.
/// Handles cases where a file on Windows has `\r\n` but LLM sends `\n` (or vice versa),
/// whitespace/indentation discrepancies, while preserving the file's original line endings.
pub fn replace_text_normalized(
    current: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<String, String> {
    // 1. Direct match first (fast path & exact match)
    if current.contains(old_string) {
        if !replace_all {
            let count = current.matches(old_string).count();
            if count > 1 {
                return Err(format!(
                    "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more surrounding lines.",
                    count
                ));
            }
            return Ok(current.replacen(old_string, new_string, 1));
        } else {
            return Ok(current.replace(old_string, new_string));
        }
    }

    // 2. Windows CRLF adaptation: file has \r\n, old_string has LF \n
    let file_has_crlf = current.contains("\r\n");
    if file_has_crlf && !old_string.contains("\r\n") && old_string.contains('\n') {
        let crlf_old = old_string.replace('\n', "\r\n");
        let crlf_new = new_string.replace("\r\n", "\n").replace('\n', "\r\n");
        if current.contains(&crlf_old) {
            if !replace_all {
                let count = current.matches(&crlf_old).count();
                if count > 1 {
                    return Err(format!(
                        "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more surrounding lines.",
                        count
                    ));
                }
                return Ok(current.replacen(&crlf_old, &crlf_new, 1));
            } else {
                return Ok(current.replace(&crlf_old, &crlf_new));
            }
        }
    }

    // 3. Reverse adaptation: file has LF \n, old_string has CRLF \r\n
    if !file_has_crlf && old_string.contains("\r\n") {
        let lf_old = old_string.replace("\r\n", "\n");
        let lf_new = new_string.replace("\r\n", "\n");
        if current.contains(&lf_old) {
            if !replace_all {
                let count = current.matches(&lf_old).count();
                if count > 1 {
                    return Err(format!(
                        "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more surrounding lines.",
                        count
                    ));
                }
                return Ok(current.replacen(&lf_old, &lf_new, 1));
            } else {
                return Ok(current.replace(&lf_old, &lf_new));
            }
        }
    }

    // 4. Mixed line endings: normalize both to LF \n, perform replacement, then restore original convention
    let norm_current = current.replace("\r\n", "\n");
    let norm_old = old_string.replace("\r\n", "\n");
    let norm_new = new_string.replace("\r\n", "\n");

    if norm_current.contains(&norm_old) {
        let count = norm_current.matches(&norm_old).count();
        if !replace_all && count > 1 {
            return Err(format!(
                "Ambiguous match: 'old_string' occurs {} times in file. Set replace_all: true or supply more surrounding lines.",
                count
            ));
        }

        let norm_updated = if replace_all {
            norm_current.replace(&norm_old, &norm_new)
        } else {
            norm_current.replacen(&norm_old, &norm_new, 1)
        };

        let final_updated = if file_has_crlf {
            norm_updated.replace('\n', "\r\n")
        } else {
            norm_updated
        };

        return Ok(final_updated);
    }

    // 5. Whitespace-relaxed line matching:
    // If exact byte match and newline normalization fail, attempt normalized line matching
    // (trimming leading/trailing whitespace on each line).
    replace_whitespace_relaxed(current, old_string, new_string, replace_all, file_has_crlf)
}
