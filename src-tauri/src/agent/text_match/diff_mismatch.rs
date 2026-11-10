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
