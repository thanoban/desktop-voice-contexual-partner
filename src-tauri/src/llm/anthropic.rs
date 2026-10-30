//! Anthropic Messages API streaming adapter (raw HTTP + SSE).

use anyhow::{anyhow, Result};

use super::client::ChatMessage;
use super::credentials::Secret;
use super::provider::{
    stream_sse, text_capabilities, ChatProvider, DataDestination, ProviderKind, SseHandler,
    SseStep, StreamChatResult, StreamTimeouts, TokenSink,
};
use super::sse::SseEvent;
use crate::conversation::controller::CancellationToken;
use crate::domain::provider::ProviderCapabilities;

const LABEL: &str = "Anthropic";
pub const ANTHROPIC_VERSION: &str = "2023-06-01";
pub const DEFAULT_MODEL: &str = "claude-sonnet-5-5";
pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const MAX_TOKENS: u32 = 4096;

pub struct AnthropicProvider {
    pub client: reqwest::Client,
    pub base_url: String,
    pub model: String,
    pub api_key: Secret,
    pub destination: DataDestination,
    pub timeouts: StreamTimeouts,
}

impl ChatProvider for AnthropicProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        text_capabilities(ProviderKind::Anthropic, &self.model)
    }

    fn destination(&self) -> &DataDestination {
        &self.destination
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        cancellation: &CancellationToken,
        sink: &mut TokenSink<'_>,
    ) -> Result<StreamChatResult> {
        let body = request_body(&self.model, messages)?;
        let request = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", self.api_key.expose())
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&body);
        stream_sse(
            LABEL,
            request,
            Some(self.api_key.expose()),
            self.timeouts,
            cancellation,
            sink,
            &mut MessagesHandler,
        )
        .await
    }
}

/// System turns move to the top-level `system` field. The conversation must
/// open with a user turn, so leading assistant turns (e.g. a greeting) are
/// dropped; consecutive same-role turns are merged by the API. Sampling
/// parameters are omitted because current models reject them.
fn request_body(model: &str, messages: &[ChatMessage]) -> Result<serde_json::Value> {
    let system = messages
        .iter()
        .filter(|message| message.role == "system")
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let turns: Vec<_> = messages
        .iter()
        .filter(|message| matches!(message.role.as_str(), "user" | "assistant"))
        .skip_while(|message| message.role != "user")
        .map(|message| serde_json::json!({"role": message.role, "content": message.content}))
        .collect();
    if turns.is_empty() {
        return Err(anyhow!("Anthropic request needs at least one user message"));
    }
    let mut body = serde_json::json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "messages": turns,
        "stream": true,
    });
    if !system.trim().is_empty() {
        body["system"] = serde_json::Value::String(system);
    }
    Ok(body)
}

struct MessagesHandler;

impl SseHandler for MessagesHandler {
    fn on_event(&mut self, event: &SseEvent) -> Result<SseStep> {
        let value: serde_json::Value =
            serde_json::from_str(&event.data).map_err(|e| anyhow!("invalid frame: {}", e))?;
        let kind = value
            .get("type")
            .and_then(|kind| kind.as_str())
            .or(event.event.as_deref())
            .unwrap_or_default();
        match kind {
            "content_block_delta" => {
                let delta = value.get("delta");
                let is_text = delta.and_then(|d| d.get("type")).and_then(|t| t.as_str())
                    == Some("text_delta");
                // Thinking and tool-input deltas are never spoken or shown.
                let text = delta
                    .and_then(|d| d.get("text"))
                    .and_then(|t| t.as_str())
                    .filter(|_| is_text)
                    .unwrap_or_default();
                Ok(SseStep::Token(text.to_string()))
            }
            "message_delta" => {
                let stop = value
                    .get("delta")
                    .and_then(|d| d.get("stop_reason"))
                    .and_then(|s| s.as_str());
                if stop == Some("refusal") {
                    return Err(anyhow!("the model declined this request"));
                }
                Ok(SseStep::Continue)
            }
            "message_stop" => Ok(SseStep::Done),
            "error" => {
                let error = value.get("error");
                let error_type = error
                    .and_then(|e| e.get("type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("error");
                let message = error
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error");
                Err(anyhow!("{}: {}", error_type, message))
            }
            // message_start, content_block_start/stop, ping and future events.
            _ => Ok(SseStep::Continue),
        }
    }

    fn on_eof(&self) -> Result<()> {
        Err(anyhow!("Anthropic stream ended before message_stop"))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;
    use crate::llm::provider::Locality;
    use crate::llm::test_support::{serve_once, test_client, FakeResponse};

    fn provider(base: &str, idle: Duration) -> AnthropicProvider {
        AnthropicProvider {
            client: test_client(),
            base_url: base.into(),
            model: DEFAULT_MODEL.into(),
            api_key: Secret::new("sk-ant-test-456".into()),
            destination: DataDestination {
                provider: ProviderKind::Anthropic,
                host: "127.0.0.1".into(),
                origin: base.into(),
                locality: Locality::LoopbackUnverified,
            },
            timeouts: StreamTimeouts {
                response: Duration::from_secs(5),
                idle,
            },
        }
    }

    fn messages() -> Vec<ChatMessage> {
        vec![
            ChatMessage {
                role: "system".into(),
                content: "You are Amy.".into(),
            },
            ChatMessage {
                role: "assistant".into(),
                content: "Hi there!".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: "hello".into(),
            },
        ]
    }

    fn event(name: &str, data: serde_json::Value) -> String {
        format!("event: {}\ndata: {}\n\n", name, data)
    }

    fn text(text: &str) -> String {
        event(
            "content_block_delta",
            serde_json::json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":text}}),
        )
    }

    fn preamble() -> String {
        format!(
            "{}{}{}",
            event(
                "message_start",
                serde_json::json!({"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[]}})
            ),
            event(
                "content_block_start",
                serde_json::json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}})
            ),
            event("ping", serde_json::json!({"type":"ping"})),
        )
    }

    fn ending() -> String {
        format!(
            "{}{}{}",
            event(
                "content_block_stop",
                serde_json::json!({"type":"content_block_stop","index":0})
            ),
            event(
                "message_delta",
                serde_json::json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":3}})
            ),
            event("message_stop", serde_json::json!({"type":"message_stop"})),
        )
    }

    async fn run(provider: &AnthropicProvider) -> (Result<StreamChatResult>, Vec<String>) {
        let mut tokens = Vec::new();
        let mut sink = |token: &str| tokens.push(token.to_string());
        let result = provider
            .stream_chat(&messages(), &CancellationToken::default(), &mut sink)
            .await;
        (result, tokens)
    }

    #[test]
    fn body_moves_system_and_starts_with_user() {
        let body = request_body(DEFAULT_MODEL, &messages()).unwrap();
        assert_eq!(body["system"], "You are Amy.");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["stream"], true);
        assert!(body.get("temperature").is_none());
        assert!(request_body(DEFAULT_MODEL, &messages()[..2]).is_err());
    }

    #[tokio::test]
    async fn streams_split_events_with_required_headers() {
        let stream = format!("{}{}{}{}", preamble(), text("Hel"), text("lo ✨"), ending());
        let bytes = stream.as_bytes().to_vec();
        let sparkle = stream.find('✨').unwrap() + 1;
        let (base, server) = serve_once(FakeResponse::bytes(vec![
            bytes[..40].to_vec(),
            bytes[40..sparkle].to_vec(),
            bytes[sparkle..].to_vec(),
        ]))
        .await;
        let (result, tokens) = run(&provider(&base, Duration::from_secs(5))).await;
        let result = result.unwrap();
        assert_eq!(result.content, "Hello ✨");
        assert_eq!(tokens, vec!["Hel", "lo ✨"]);

        let request = server.await.unwrap();
        let lower = request.to_ascii_lowercase();
        assert!(request.starts_with("POST /v1/messages "));
        assert!(lower.contains("x-api-key: sk-ant-test-456"));
        assert!(lower.contains("anthropic-version: 2023-06-01"));
        assert!(!lower.contains("authorization:"));
        assert!(request.contains("\"model\":\"claude-sonnet-5-5\""));
    }

    #[tokio::test]
    async fn many_events_in_one_chunk_and_thinking_is_ignored() {
        let thinking = event(
            "content_block_delta",
            serde_json::json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"secret plan"}}),
        );
        let body = format!("{}{}{}{}", preamble(), thinking, text("ok"), ending());
        let (base, _server) = serve_once(FakeResponse::sse(&[&body])).await;
        let (result, tokens) = run(&provider(&base, Duration::from_secs(5))).await;
        assert_eq!(result.unwrap().content, "ok");
        assert_eq!(tokens, vec!["ok"]);
    }

    #[tokio::test]
    async fn error_event_fails_with_redacted_message() {
        let error = event(
            "error",
            serde_json::json!({"type":"error","error":{"type":"overloaded_error","message":"Overloaded sk-ant-test-456"}}),
        );
        let (base, _server) =
            serve_once(FakeResponse::sse(&[&preamble(), &text("par"), &error])).await;
        let (result, tokens) = run(&provider(&base, Duration::from_secs(5))).await;
        let error = result.unwrap_err().to_string();
        assert!(error.contains("overloaded_error"));
        assert!(!error.contains("sk-ant-test-456"));
        assert_eq!(tokens, vec!["par"]);
    }

    #[tokio::test]
    async fn refusal_and_missing_message_stop_are_errors() {
        let refusal = event(
            "message_delta",
            serde_json::json!({"type":"message_delta","delta":{"stop_reason":"refusal"}}),
        );
        let (base, _server) = serve_once(FakeResponse::sse(&[&preamble(), &refusal])).await;
        let (result, _) = run(&provider(&base, Duration::from_secs(5))).await;
        assert!(result.unwrap_err().to_string().contains("declined"));

        let (base, _server) = serve_once(FakeResponse::sse(&[&preamble(), &text("x")])).await;
        let (result, _) = run(&provider(&base, Duration::from_secs(5))).await;
        assert!(result.unwrap_err().to_string().contains("message_stop"));
    }

    #[tokio::test]
    async fn cancellation_mid_stream_stops_reading() {
        let mut response = FakeResponse::sse(&[&preamble(), &text("first")]);
        response.hold_open = Some(Duration::from_secs(10));
        let (base, _server) = serve_once(response).await;
        let provider = provider(&base, Duration::from_secs(30));
        let cancellation = Arc::new(CancellationToken::default());
        let canceller = Arc::clone(&cancellation);
        let mut sink = move |_: &str| canceller.cancel();
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            provider.stream_chat(&messages(), &cancellation, &mut sink),
        )
        .await
        .expect("cancellation must not wait for the server")
        .unwrap();
        assert!(result.cancelled);
        assert_eq!(result.content, "first");
    }

    #[tokio::test]
    async fn http_error_reports_status_without_key() {
        let mut response = FakeResponse::sse(&[
            r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key sk-ant-test-456"}}"#,
        ]);
        response.status = "401 Unauthorized";
        response.content_type = "application/json";
        let (base, _server) = serve_once(response).await;
        let (result, _) = run(&provider(&base, Duration::from_secs(5))).await;
        let error = result.unwrap_err().to_string();
        assert!(error.contains("401"));
        assert!(!error.contains("sk-ant-test-456"));
    }
}
