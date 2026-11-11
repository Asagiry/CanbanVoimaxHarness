//! Web Tools Implementation (DuckDuckGo Search & Clean Markdown Fetch)
//! Extracted from agent.rs per ub-mode Clean Code & Command Pattern principles.

use std::net::IpAddr;
use std::sync::LazyLock;
use std::time::Duration;
use anyhow::{bail, Context, Result};
use crate::agent::ToolContext;

const MAX_WEB_BODY_BYTES: usize = 2 * 1024 * 1024;

static WEB_SEARCH_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .unwrap_or_default()
});

static WEB_FETCH_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(18))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .unwrap_or_default()
});

fn floor_char_boundary(s: &str, idx: usize) -> usize {
    let mut i = idx.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil_char_boundary(s: &str, idx: usize) -> usize {
    let mut i = idx.min(s.len());
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

pub fn is_disallowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let oct = v4.octets();
            let is_cgnat = oct[0] == 100 && (oct[1] & 0b1100_0000) == 64;
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                || is_cgnat
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_disallowed_ip(IpAddr::V4(mapped));
            }
            let seg0 = v6.segments()[0];
            let is_unique_local = (seg0 & 0xfe00) == 0xfc00;
            let is_link_local = (seg0 & 0xffc0) == 0xfe80;
            v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() || is_unique_local || is_link_local
        }
    }
}

pub async fn validate_public_url(url_str: &str) -> Result<reqwest::Url> {
    let parsed = reqwest::Url::parse(url_str.trim())
        .with_context(|| format!("Invalid URL: {:?}", url_str))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("Blocked URL scheme '{}': only http and https are allowed", parsed.scheme());
    }
    let host = parsed.host_str().context("URL has no host")?.trim_end_matches('.');
    let lower_host = host.to_ascii_lowercase();
    if lower_host == "localhost"
        || lower_host.ends_with(".localhost")
        || lower_host.ends_with(".local")
        || lower_host.ends_with(".internal")
    {
        bail!("Blocked SSRF target host: {}", host);
    }
    if let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
        if is_disallowed_ip(ip) {
            bail!("Blocked private/loopback IP address: {}", ip);
        }
        return Ok(parsed);
    }
    let port = parsed.port_or_known_default().unwrap_or(80);
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .with_context(|| format!("Failed to resolve host '{}'", host))?;
    let mut resolved_any = false;
    for addr in addrs {
        resolved_any = true;
        if is_disallowed_ip(addr.ip()) {
            bail!("Blocked SSRF target IP {} resolved for host {}", addr.ip(), host);
        }
    }
    if !resolved_any {
        bail!("DNS resolution returned no addresses for host '{}'", host);
    }
    Ok(parsed)
}

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
    let client = &*WEB_SEARCH_CLIENT;

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
    while let Some(start_res) = html.get(pos..).and_then(|s| s.find("class=\"result__body\"")) {
        let abs_start = pos + start_res;
        let slice_end = match html[abs_start..].find("</div>\n</div>") {
            Some(off) => abs_start + off,
            None => floor_char_boundary(html, abs_start.saturating_add(1500)),
        };
        let block = &html[abs_start..slice_end];

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

        pos = slice_end.max(ceil_char_boundary(html, abs_start + 1));
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
        let search_start = ceil_char_boundary(block, idx.saturating_sub(60));
        let search_range = &block[search_start..idx];
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
    let mut current_url = validate_public_url(url).await?;
    let mut response = None;

    for _ in 0..=5 {
        let resp = WEB_FETCH_CLIENT
            .get(current_url.clone())
            .send()
            .await
            .with_context(|| format!("Failed to fetch URL: {}", current_url))?;
        if resp.status().is_redirection() {
            let loc = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .context("Redirect response missing valid Location header")?;
            let next_url = current_url
                .join(loc)
                .with_context(|| format!("Invalid redirect URL: {}", loc))?;
            current_url = validate_public_url(next_url.as_str()).await?;
            continue;
        }
        response = Some(resp);
        break;
    }

    let mut response = response.context("Too many redirects while fetching URL")?;
    if !response.status().is_success() {
        bail!("HTTP request failed with status: {}", response.status());
    }
    if response.content_length().is_some_and(|len| len > MAX_WEB_BODY_BYTES as u64) {
        bail!("Response body exceeds maximum allowed size ({} bytes)", MAX_WEB_BODY_BYTES);
    }

    let mut buf = Vec::new();
    while let Some(chunk) = response.chunk().await.context("Failed to read response chunk")? {
        if buf.len() + chunk.len() > MAX_WEB_BODY_BYTES {
            let remaining = MAX_WEB_BODY_BYTES - buf.len();
            buf.extend_from_slice(&chunk[..remaining]);
            break;
        }
        buf.extend_from_slice(&chunk);
    }
    let html = String::from_utf8_lossy(&buf);
    Ok(convert_html_to_clean_markdown(&html))
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

fn find_ascii_ignore_case(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle))
}

pub fn strip_html_block(html: &str, tag: &str) -> String {
    let open_pat = format!("<{}", tag);
    let close_pat = format!("</{}>", tag);
    let open_bytes = open_pat.as_bytes();
    let close_bytes = close_pat.as_bytes();
    let bytes = html.as_bytes();
    let mut result = String::new();
    let mut pos = 0;

    while let Some(open_idx) = find_ascii_ignore_case(&bytes[pos..], open_bytes) {
        let abs_open = pos + open_idx;
        if let Some(chunk) = html.get(pos..abs_open) {
            result.push_str(chunk);
        } else {
            break;
        }
        if let Some(close_idx) = find_ascii_ignore_case(&bytes[abs_open..], close_bytes) {
            pos = abs_open + close_idx + close_bytes.len();
        } else {
            pos = html.len();
            break;
        }
    }
    if let Some(tail) = html.get(pos..) {
        result.push_str(tail);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_public_url_blocks_ssrf() {
        for bad in [
            "file:///etc/passwd",
            "http://127.0.0.1:8080/admin",
            "http://localhost:3000",
            "http://169.254.169.254/latest/meta-data/",
            "http://10.0.0.1/internal",
            "http://[::1]:8080/",
        ] {
            assert!(
                validate_public_url(bad).await.is_err(),
                "Expected SSRF validation to block {}",
                bad
            );
        }
    }

    #[test]
    fn test_web_utf8_slicing_safety() {
        let html = format!(
            "<div class=\"result__body\">{}<a class=\"result__a\" href=\"https://example.com\">Заголовок</a><span class=\"result__snippet\">Текст</span>",
            "ж".repeat(1600)
        );
        let parsed = parse_ddg_results(&html, 5);
        assert_eq!(parsed.len(), 0);
        let stripped = strip_html_block("<script>İ</script><p>Привет</p>", "script");
        assert_eq!(stripped, "<p>Привет</p>");
    }
}
