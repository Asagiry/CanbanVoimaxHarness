//! Web Tools Implementation (DuckDuckGo Search & Clean Markdown Fetch)
//! Extracted from agent.rs per ub-mode Clean Code & Command Pattern principles.

use std::time::Duration;
use anyhow::{bail, Context, Result};
use crate::agent::ToolContext;

pub async fn execute_web_search(_ctx: &ToolContext<'_>, args: &serde_json::Value) -> Result<String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'query'")?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    search_web(query, limit).await
}

pub async fn execute_web_fetch(_ctx: &ToolContext<'_>, args: &serde_json::Value) -> Result<String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .context("Missing required parameter 'url'")?;
    fetch_web_url(url).await
}

pub async fn search_web(query: &str, limit: usize) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    // 1. DuckDuckGo HTML Search
    let url = format!("https://html.duckduckgo.com/html/?q={}", simple_url_encode(query));
    if let Ok(resp) = client.get(&url).send().await {
        if resp.status().is_success() {
            if let Ok(html) = resp.text().await {
                let results = parse_ddg_results(&html, limit);
                if !results.is_empty() {
                    return Ok(results.join("\n\n"));
                }
            }
        }
    }

    // 2. Fallback: DuckDuckGo Instant Answer API
    let api_url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        simple_url_encode(query)
    );
    if let Ok(resp) = client.get(&api_url).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            let mut out = Vec::new();
            if let Some(abs) = json.get("AbstractText").and_then(|v| v.as_str()) {
                if !abs.is_empty() {
                    let source = json.get("AbstractURL").and_then(|v| v.as_str()).unwrap_or("");
                    out.push(format!("### Abstract\n{}\nSource: {}", abs, source));
                }
            }
            if let Some(topics) = json.get("RelatedTopics").and_then(|v| v.as_array()) {
                for t in topics.iter().take(limit) {
                    if let Some(txt) = t.get("Text").and_then(|v| v.as_str()) {
                        let u = t.get("FirstURL").and_then(|v| v.as_str()).unwrap_or("");
                        out.push(format!("- **{}**: {}", txt, u));
                    }
                }
            }
            if !out.is_empty() {
                return Ok(out.join("\n\n"));
            }
        }
    }

    Ok(format!("Search completed for query '{}'. No web records matched.", query))
}

pub fn simple_url_encode(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}

pub fn parse_ddg_results(html: &str, limit: usize) -> Vec<String> {
    let mut results = Vec::new();
    let mut pos = 0;
    while let Some(start_res) = html[pos..].find("class=\"result__body\"") {
        let abs_start = pos + start_res;
        let end_res = html[abs_start..].find("</div>\n</div>").unwrap_or(1500);
        let block = &html[abs_start..abs_start + end_res];

        let title = extract_snippet_tag(block, "result__a");
        let snippet = extract_snippet_tag(block, "result__snippet");
        let url = extract_snippet_href(block, "result__a");

        if !title.is_empty() {
            results.push(format!(
                "### [{}]({})\n{}",
                clean_tags(&title),
                url,
                clean_tags(&snippet)
            ));
        }

        pos = abs_start + end_res;
        if results.len() >= limit {
            break;
        }
    }
    results
}

pub fn extract_snippet_tag(block: &str, class_name: &str) -> String {
    if let Some(idx) = block.find(class_name) {
        if let Some(open) = block[idx..].find('>') {
            let start = idx + open + 1;
            if let Some(close) = block[start..].find('<') {
                return block[start..start + close].trim().to_string();
            }
        }
    }
    String::new()
}

pub fn extract_snippet_href(block: &str, class_name: &str) -> String {
    if let Some(idx) = block.find(class_name) {
        let search_range = &block[idx.saturating_sub(60)..idx.min(block.len())];
        if let Some(href_idx) = search_range.find("href=\"") {
            let start = href_idx + 6;
            if let Some(end) = search_range[start..].find('"') {
                return search_range[start..start + end].to_string();
            }
        }
    }
    String::new()
}

pub fn clean_tags(s: &str) -> String {
    let mut clean = String::new();
    let mut inside = false;
    for c in s.chars() {
        if c == '<' {
            inside = true;
        } else if c == '>' {
            inside = false;
        } else if !inside {
            clean.push(c);
        }
    }
    clean
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}

pub async fn fetch_web_url(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(18))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch URL: {}", url))?;

    if !response.status().is_success() {
        bail!("HTTP request failed with status: {}", response.status());
    }

    let html = response.text().await.context("Failed to read HTML body")?;
    let markdown = convert_html_to_clean_markdown(&html);
    Ok(markdown)
}

pub fn convert_html_to_clean_markdown(html: &str) -> String {
    let mut clean = html.to_string();
    for tag in &["script", "style", "head", "nav", "footer", "svg"] {
        clean = strip_html_block(&clean, tag);
    }

    let mut md = String::new();
    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let stripped = clean_tags(trimmed);
        if !stripped.is_empty() {
            md.push_str(&stripped);
            md.push('\n');
        }
    }

    if md.is_empty() {
        "(Empty web page content)".to_string()
    } else {
        md
    }
}

pub fn strip_html_block(html: &str, tag: &str) -> String {
    let open_pat = format!("<{}", tag);
    let close_pat = format!("</{}>", tag);
    let mut result = String::new();
    let mut pos = 0;

    while let Some(open_idx) = html[pos..].to_lowercase().find(&open_pat) {
        let abs_open = pos + open_idx;
        result.push_str(&html[pos..abs_open]);

        if let Some(close_idx) = html[abs_open..].to_lowercase().find(&close_pat) {
            pos = abs_open + close_idx + close_pat.len();
        } else {
            pos = html.len();
            break;
        }
    }
    result.push_str(&html[pos..]);
    result
}
