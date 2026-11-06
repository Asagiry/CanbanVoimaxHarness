use super::OpenAIChatRequest;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::hash::{Hash, Hasher};

pub fn opencode_headers(base_url: &str, session: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if !reqwest::Url::parse(base_url).is_ok_and(|url| url.host_str() == Some("opencode.ai")) {
        return headers;
    }
    if let Some(value) = opencode_session_header(base_url, session) {
        if let Ok(header_value) = HeaderValue::from_str(&value) {
            headers.insert("x-opencode-session", header_value);
        }
    }
    headers.insert(USER_AGENT, HeaderValue::from_static(concat!("CanbanVoimaxHarness/", env!("CARGO_PKG_VERSION"))));
    headers
}

pub fn opencode_session_header(base_url: &str, session: &str) -> Option<String> {
    if !reqwest::Url::parse(base_url).is_ok_and(|url| url.host_str() == Some("opencode.ai")) {
        return None;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    session.hash(&mut hasher);
    Some(format!("cvh-{:016x}", hasher.finish()))
}

#[derive(Debug, Default)]
pub struct PrefixTracker {
    model: String,
    tools: String,
    messages: Vec<String>,
}

impl PrefixTracker {
    pub fn observe(&mut self, request: &OpenAIChatRequest) -> Result<(), serde_json::Error> {
        let tools = serde_json::to_string(&request.tools)?;
        let messages: Vec<String> = request.messages.iter()
            .map(serde_json::to_string).collect::<Result<_, _>>()?;
        let anchors_match = self.model == request.model && self.tools == tools;
        let common_messages = if anchors_match {
            self.messages.iter().zip(&messages).take_while(|(old, new)| old == new).count()
        } else {
            0
        };
        let prefix_preserved = anchors_match && !self.messages.is_empty() && common_messages == self.messages.len();
        let common_bytes: usize = messages.iter().take(common_messages).map(String::len).sum();
        eprintln!(
            "CVH [cache] model={} previous_messages={} messages={} anchors_match={} prefix_preserved={} common_messages={} common_bytes={} first_changed_message={:?}",
            request.model, self.messages.len(), messages.len(), anchors_match, prefix_preserved,
            common_messages, common_bytes,
            if common_messages < self.messages.len() { Some(common_messages) } else { None },
        );
        self.model.clone_from(&request.model);
        self.tools = tools;
        self.messages = messages;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_routing_is_stable_and_scoped_to_opencode() {
        let first = opencode_headers("https://opencode.ai/inference/go/openai/v1", "project:task");
        assert_eq!(first, opencode_headers("https://opencode.ai/inference/go/openai/v1", "project:task"));
        assert_ne!(first["x-opencode-session"], opencode_headers("https://opencode.ai", "project:child")["x-opencode-session"]);
        assert!(first[USER_AGENT].to_str().unwrap().starts_with("CanbanVoimaxHarness/"));
        assert!(opencode_headers("https://opencode.ai.example.com/v1", "project:task").is_empty());
    }

}
