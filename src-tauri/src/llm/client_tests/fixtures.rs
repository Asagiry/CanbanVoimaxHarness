use anyhow::Result;
use crate::llm::client::LlmClient;
use crate::llm::models::*;

pub async fn read_test_request(socket: &mut tokio::net::TcpStream) -> (String, serde_json::Value) {
    use tokio::io::AsyncReadExt;
    let mut request = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let count = socket.read(&mut buffer).await.unwrap();
        assert_ne!(count, 0);
        request.extend_from_slice(&buffer[..count]);
        if let Some(end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
            let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: "))
                .unwrap().parse().unwrap();
            if request.len() >= end + 4 + length {
                return (headers, serde_json::from_slice(&request[end + 4..end + 4 + length]).unwrap());
            }
        }
    }
}

pub async fn stream_test_http_response(status: &str, body: String, missing_bytes: usize) -> (Result<()>, Vec<StreamChunk>) {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let response = format!("HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len() + missing_bytes);
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        read_test_request(&mut socket).await;
        socket.write_all(response.as_bytes()).await.unwrap();
    });
    let client = LlmClient {
        client: reqwest::Client::builder().no_proxy().build().unwrap(),
        ..LlmClient::new()
    };
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let request = OpenAIChatRequest {
        model: "test".into(), messages: vec![ChatMessage::user("test")], temperature: None,
        stream: true, tools: None, reasoning_effort: None, stream_options: None,
    };
    let client_task = tokio::spawn(async move { client.stream_openai(&endpoint, None, request, tx).await });
    let events = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            let retry = matches!(event, StreamChunk::Retry { .. });
            events.push(event);
            if retry { break; }
        }
        events
    }).await.unwrap();
    drop(rx);
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), client_task).await.unwrap().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), server).await.unwrap().unwrap();
    (result, events)
}

pub async fn stream_test_response(response: String) -> (Result<()>, Vec<StreamChunk>) {
    stream_test_http_response("200 OK", response, 0).await
}

pub fn tool_stream_frame(arguments: &str) -> String {
    format!("data: {}\n\n", serde_json::json!({"choices": [{"delta": {"tool_calls": [{
        "index": 0, "id": "call-1", "function": {"name": "read_file", "arguments": arguments}
    }]}}]}))
}
