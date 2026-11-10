/// Represents a line span with its text and byte boundaries in a file string.
#[derive(Debug, Clone)]
pub struct LineBoundary<'a> {
    pub text: &'a str,
    pub start: usize,
    pub end_content: usize,
    pub end_full: usize,
}

pub fn parse_line_boundaries<'a>(content: &'a str) -> Vec<LineBoundary<'a>> {
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
pub fn replace_whitespace_relaxed(
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
