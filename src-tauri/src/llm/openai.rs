//! OpenAI-compatible Chat Completions streaming adapter. The same wire format
//! covers OpenAI, OpenRouter, Groq, LM Studio and similar servers by base URL.

use anyhow::{anyhow, Result};

use super::client::ChatMessage;
use super::credentials::Secret;
use super::provider::{
    error_message_from_value, stream_sse, text_capabilities, ChatProvider, DataDestination,
    ProviderKind, SseHandler, SseStep, StreamChatResult, StreamTimeouts, TokenSink,
};
use super::sse::SseEvent;
use crate::conversation::controller::CancellationToken;
use crate::domain::provider::ProviderCapabilities;

const LABEL: &str = "OpenAI-compatible provider";

pub struct OpenAiCompatibleProvider {
    pub client: reqwest::Client,
    /// Normalised base such as `https://api.openai.com/v1`.
    pub base_url: String,
    pub model: String,
    pub api_key: Option<Secret>,
    pub destination: DataDestination,
    pub timeouts: StreamTimeouts,
}

impl ChatProvider for OpenAiCompatibleProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        text_capabilities(ProviderKind::OpenaiCompatible, &self.model)
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
        // Sampling and length parameters are omitted: compatible servers and
        // newer OpenAI models disagree on which ones they accept.
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });
        let mut request = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&body);
        if let Some(key) = &self.api_key {
            request = request.bearer_auth(key.expose());
        }
        stream_sse(
            LABEL,
            request,
            self.api_key.as_ref().map(Secret::expose),
            self.timeouts,
            cancellation,
            sink,
            &mut ChatCompletionsHandler::default(),
        )
        .await
    }
}

#[derive(Default)]
struct ChatCompletionsHandler {
    finished: bool,
}

impl SseHandler for ChatCompletionsHandler {
    fn on_event(&mut self, event: &SseEvent) -> Result<SseStep> {
        let data = event.data.trim();
        if data == "[DONE]" {
            return Ok(SseStep::Done);
        }
        let value: serde_json::Value =
            serde_json::from_str(data).map_err(|e| anyhow!("invalid frame: {}", e))?;
        if let Some(message) = error_message_from_value(&value) {
            return Err(anyhow!("{}", message));
        }
        if event.event.as_deref() == Some("error") {
            return Err(anyhow!("provider reported an error event"));
        }
        let Some(choice) = value
            .get("choices")
            .and_then(|choices| choices.as_array())
            .and_then(|choices| choices.first())
        else {
            // Usage-only or keep-alive frames carry no choices.
            return Ok(SseStep::Continue);
        };
        match choice
            .get("finish_reason")
            .and_then(|reason| reason.as_str())
        {
            Some("error") => return Err(anyhow!("generation finished with an error")),
            Some(_) => self.finished = true,
            None => {}
        }
        let token = choice
            .get("delta")
            .and_then(|delta| delta.get("content"))
            .and_then(|content| content.as_str())
            .unwrap_or_default();
        Ok(SseStep::Token(token.to_string()))
    }

    fn on_eof(&self) -> Result<()> {
        // Some compatible servers close after finish_reason without `[DONE]`.
        if self.finished {
            Ok(())
        } else {
            Err(anyhow!("{} stream ended without a terminal event", LABEL))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;
    use crate::llm::provider::Locality;
    use crate::llm::test_support::{serve_once, test_client, FakeResponse};

    fn provider(base: &str, key: Option<&str>, idle: Duration) -> OpenAiCompatibleProvider {
        OpenAiCompatibleProvider {
            client: test_client(),
            base_url: format!("{}/v1", base),
            model: "test-model".into(),
            api_key: key.map(|key| Secret::new(key.into())),
            destination: DataDestination {
                provider: ProviderKind::OpenaiCompatible,
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
                content: "be brief".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: "hello".into(),
            },
        ]
    }

    async fn run(
        provider: &OpenAiCompatibleProvider,
        cancellation: &CancellationToken,
    ) -> (Result<StreamChatResult>, Vec<String>) {
        let mut tokens = Vec::new();
        let mut sink = |token: &str| tokens.push(token.to_string());
        let result = provider
            .stream_chat(&messages(), cancellation, &mut sink)
            .await;
        (result, tokens)
    }

    fn delta(text: &str) -> String {
        format!(
            "data: {}\n\n",
            serde_json::json!({"choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]})
        )
    }

    #[tokio::test]
    async fn streams_split_frames_and_sends_bearer_auth() {
        let first = delta("Hel");
        let second = delta("lo 🌍");
        let emoji = second.find('🌍').unwrap() + 1;
        let stream = format!("{}{}data: [DONE]\n\n", first, second);
        let split_a = first.len() / 2;
        let split_b = first.len() + emoji;
        let bytes = stream.as_bytes();
        let (base, server) = serve_once(FakeResponse::bytes(vec![
            bytes[..split_a].to_vec(),
            bytes[split_a..split_b].to_vec(),
            bytes[split_b..].to_vec(),
        ]))
        .await;
        let provider = provider(&base, Some("sk-test-123"), Duration::from_secs(5));
        let (result, tokens) = run(&provider, &CancellationToken::default()).await;
        let result = result.unwrap();
        assert!(!result.cancelled);
        assert_eq!(result.content, "Hello 🌍");
        assert_eq!(tokens, vec!["Hel", "lo 🌍"]);

        let request = server.await.unwrap();
        assert!(request.starts_with("POST /v1/chat/completions "));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer sk-test-123"));
        assert!(request.contains("\"stream\":true"));
        assert!(request.contains("\"model\":\"test-model\""));
    }

    #[tokio::test]
    async fn handles_many_events_in_one_chunk_and_omits_auth_without_key() {
        let body = format!(
            ": comment\n\n{}{}{}data: [DONE]\n\n",
            delta("a"),
            delta("b"),
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n"
        );
        let (base, server) = serve_once(FakeResponse::sse(&[&body])).await;
        let provider = provider(&base, None, Duration::from_secs(5));
        let (result, tokens) = run(&provider, &CancellationToken::default()).await;
        assert_eq!(result.unwrap().content, "ab");
        assert_eq!(tokens.len(), 2);
        let request = server.await.unwrap().to_ascii_lowercase();
        assert!(!request.contains("authorization:"));
    }

    #[tokio::test]
    async fn mid_stream_error_event_fails_and_redacts_key() {
        let error = "data: {\"error\":{\"message\":\"quota exceeded for sk-test-123\"}}\n\n";
        let (base, _server) = serve_once(FakeResponse::sse(&[&delta("partial"), error])).await;
        let provider = provider(&base, Some("sk-test-123"), Duration::from_secs(5));
        let (result, tokens) = run(&provider, &CancellationToken::default()).await;
        let error = result.unwrap_err().to_string();
        assert!(error.contains("quota exceeded"));
        assert!(!error.contains("sk-test-123"));
        assert_eq!(tokens, vec!["partial"]);
    }

    #[tokio::test]
    async fn http_error_body_is_bounded_and_redacted() {
        let mut response = FakeResponse::sse(&[
            "{\"error\":{\"message\":\"Incorrect API key provided: sk-test-123\"}}",
        ]);
        response.status = "401 Unauthorized";
        response.content_type = "application/json";
        let (base, _server) = serve_once(response).await;
        let provider = provider(&base, Some("sk-test-123"), Duration::from_secs(5));
        let (result, _) = run(&provider, &CancellationToken::default()).await;
        let error = result.unwrap_err().to_string();
        assert!(error.contains("401"));
        assert!(error.contains("check the API key"));
        assert!(!error.contains("sk-test-123"));
    }

    #[tokio::test]
    async fn stream_without_terminal_event_is_an_error() {
        let (base, _server) = serve_once(FakeResponse::sse(&[&delta("cut")])).await;
        let provider = provider(&base, None, Duration::from_secs(5));
        let (result, _) = run(&provider, &CancellationToken::default()).await;
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("without a terminal event"));
    }

    #[tokio::test]
    async fn cancellation_mid_stream_returns_partial_content() {
        let mut response = FakeResponse::sse(&[&delta("first")]);
        response.hold_open = Some(Duration::from_secs(10));
        let (base, _server) = serve_once(response).await;
        let provider = provider(&base, None, Duration::from_secs(30));
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
    async fn stalled_stream_times_out() {
        let mut response = FakeResponse::sse(&[&delta("x")]);
        response.hold_open = Some(Duration::from_secs(10));
        let (base, _server) = serve_once(response).await;
        let provider = provider(&base, None, Duration::from_millis(200));
        let (result, _) = run(&provider, &CancellationToken::default()).await;
        assert!(result.unwrap_err().to_string().contains("stalled"));
    }
}
