//! Text-generation provider interface and the HTTP plumbing shared by adapters.
//!
//! Adapters only translate a request into a provider protocol and report text
//! deltas through a [`TokenSink`]; routing, consent and credential lookup are
//! decided before an adapter is constructed (see `router.rs`).

use std::time::Duration;

use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use super::client::ChatMessage;
use super::sse::{SseDecoder, SseEvent};
use crate::conversation::controller::CancellationToken;
use crate::domain::provider::ProviderCapabilities;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Ollama,
    OpenaiCompatible,
    Anthropic,
}

impl ProviderKind {
    pub const ALL: [ProviderKind; 3] = [Self::Ollama, Self::OpenaiCompatible, Self::Anthropic];

    pub fn id(self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::OpenaiCompatible => "openai_compatible",
            Self::Anthropic => "anthropic",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::OpenaiCompatible => "OpenAI-compatible",
            Self::Anthropic => "Anthropic",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == value)
    }

    /// Whether requests are refused when no key is stored for the destination.
    /// OpenAI-compatible servers such as LM Studio may run without auth.
    pub fn requires_key(self) -> bool {
        matches!(self, Self::Anthropic)
    }

    pub fn accepts_key(self) -> bool {
        !matches!(self, Self::Ollama)
    }
}

/// Where a connection terminates. Loopback alone does not prove downstream
/// locality, so only the qualified local Ollama route counts as `Local`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Locality {
    Local,
    LoopbackUnverified,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DataDestination {
    pub provider: ProviderKind,
    /// Host and non-default port, e.g. `api.openai.com` or `localhost:11434`.
    pub host: String,
    pub origin: String,
    pub locality: Locality,
}

#[derive(Debug)]
pub struct StreamChatResult {
    pub content: String,
    pub cancelled: bool,
}

impl StreamChatResult {
    pub fn completed(content: String) -> Self {
        Self {
            content,
            cancelled: false,
        }
    }

    pub fn cancelled(content: String) -> Self {
        Self {
            content,
            cancelled: true,
        }
    }
}

/// Receives each non-empty text delta in order.
pub type TokenSink<'a> = dyn FnMut(&str) + Send + 'a;

pub trait ChatProvider {
    /// Advertised features. `cancellation` means the local stream is aborted;
    /// none of these adapters is live-qualified yet.
    fn capabilities(&self) -> ProviderCapabilities;

    fn destination(&self) -> &DataDestination;

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        cancellation: &CancellationToken,
        sink: &mut TokenSink<'_>,
    ) -> Result<StreamChatResult>;
}

pub fn text_capabilities(kind: ProviderKind, model: &str) -> ProviderCapabilities {
    ProviderCapabilities {
        provider_id: kind.id().into(),
        model_id: model.into(),
        tested_revision: None,
        text: true,
        audio_input: false,
        audio_output: false,
        vision: false,
        tools: false,
        streaming: true,
        cancellation: true,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StreamTimeouts {
    /// Until response headers arrive.
    pub response: Duration,
    /// Maximum silence between body chunks.
    pub idle: Duration,
}

impl Default for StreamTimeouts {
    fn default() -> Self {
        Self {
            response: Duration::from_secs(30),
            idle: Duration::from_secs(120),
        }
    }
}

/// Remote client: verified TLS (reqwest default), bounded connect time, and no
/// redirects so credentials are never replayed to another origin.
pub fn remote_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| anyhow!("HTTP client build failed: {}", e))
}

pub enum SseStep {
    Continue,
    Token(String),
    Done,
}

pub trait SseHandler: Send {
    fn on_event(&mut self, event: &SseEvent) -> Result<SseStep>;
    /// Called when the body ends without a `Done` step.
    fn on_eof(&self) -> Result<()>;
}

const MAX_ERROR_BODY_BYTES: usize = 8 * 1024;
const MAX_ERROR_MESSAGE_CHARS: usize = 300;

/// Send a streaming request and feed SSE events to `handler` until it reports
/// completion, the token is cancelled, or a timeout/protocol error occurs.
pub async fn stream_sse(
    label: &str,
    request: reqwest::RequestBuilder,
    secret: Option<&str>,
    timeouts: StreamTimeouts,
    cancellation: &CancellationToken,
    sink: &mut TokenSink<'_>,
    handler: &mut dyn SseHandler,
) -> Result<StreamChatResult> {
    let send = tokio::time::timeout(timeouts.response, request.send());
    let response = tokio::select! {
        _ = cancellation.cancelled() => return Ok(StreamChatResult::cancelled(String::new())),
        response = send => match response {
            Err(_) => return Err(anyhow!("{} did not respond within {:?}", label, timeouts.response)),
            Ok(Err(error)) => {
                return Err(anyhow!("{} request failed: {}", label, redact(&error.to_string(), secret)))
            }
            Ok(Ok(response)) => response,
        },
    };

    if !response.status().is_success() {
        return Err(error_from_response(label, response, secret).await);
    }

    let mut content = String::new();
    let mut decoder = SseDecoder::default();
    let mut stream = response.bytes_stream();
    loop {
        let next = tokio::select! {
            _ = cancellation.cancelled() => return Ok(StreamChatResult::cancelled(content)),
            next = tokio::time::timeout(timeouts.idle, stream.next()) => next,
        };
        let next = next
            .map_err(|_| anyhow!("{} stream stalled for more than {:?}", label, timeouts.idle))?;
        let Some(chunk) = next else { break };
        let bytes = chunk
            .map_err(|e| anyhow!("{} stream error: {}", label, redact(&e.to_string(), secret)))?;
        for event in decoder.push(&bytes)? {
            if apply_step(handler, &event, &mut content, sink, label, secret)? {
                return Ok(StreamChatResult::completed(content));
            }
            if cancellation.is_cancelled() {
                return Ok(StreamChatResult::cancelled(content));
            }
        }
    }
    for event in decoder.finish()? {
        if apply_step(handler, &event, &mut content, sink, label, secret)? {
            return Ok(StreamChatResult::completed(content));
        }
    }
    handler.on_eof()?;
    Ok(StreamChatResult::completed(content))
}

fn apply_step(
    handler: &mut dyn SseHandler,
    event: &SseEvent,
    content: &mut String,
    sink: &mut TokenSink<'_>,
    label: &str,
    secret: Option<&str>,
) -> Result<bool> {
    match handler.on_event(event) {
        Ok(SseStep::Continue) => Ok(false),
        Ok(SseStep::Done) => Ok(true),
        Ok(SseStep::Token(token)) => {
            if !token.is_empty() {
                content.push_str(&token);
                sink(&token);
            }
            Ok(false)
        }
        Err(error) => Err(anyhow!(
            "{} stream error: {}",
            label,
            bounded(&redact(&error.to_string(), secret))
        )),
    }
}

async fn error_from_response(
    label: &str,
    mut response: reqwest::Response,
    secret: Option<&str>,
) -> anyhow::Error {
    let status = response.status();
    let mut body = Vec::new();
    while body.len() < MAX_ERROR_BODY_BYTES {
        match response.chunk().await {
            Ok(Some(chunk)) => body.extend_from_slice(&chunk),
            _ => break,
        }
    }
    body.truncate(MAX_ERROR_BODY_BYTES);
    let detail = error_message_from_body(&body);
    let hint = match status.as_u16() {
        401 | 403 => " (check the API key for this provider)",
        429 => " (rate limited or out of credit)",
        _ => "",
    };
    let detail = bounded(&redact(&detail, secret));
    if detail.is_empty() {
        anyhow!("{} returned HTTP {}{}", label, status, hint)
    } else {
        anyhow!("{} returned HTTP {}{}: {}", label, status, hint, detail)
    }
}

/// Extract `error.message` (OpenAI and Anthropic shapes) or fall back to text.
pub fn error_message_from_body(body: &[u8]) -> String {
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) {
        if let Some(message) = error_message_from_value(&value) {
            return message;
        }
    }
    String::from_utf8_lossy(body).trim().to_string()
}

pub fn error_message_from_value(value: &serde_json::Value) -> Option<String> {
    let error = value.get("error")?;
    if let Some(message) = error.get("message").and_then(|m| m.as_str()) {
        return Some(message.to_string());
    }
    error.as_str().map(str::to_string)
}

pub fn redact(message: &str, secret: Option<&str>) -> String {
    match secret {
        Some(secret) if !secret.is_empty() => message.replace(secret, "[redacted]"),
        _ => message.to_string(),
    }
}

fn bounded(message: &str) -> String {
    let message = message.trim();
    if message.chars().count() <= MAX_ERROR_MESSAGE_CHARS {
        return message.to_string();
    }
    let mut bounded: String = message.chars().take(MAX_ERROR_MESSAGE_CHARS).collect();
    bounded.push('…');
    bounded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_ids_round_trip() {
        for kind in ProviderKind::ALL {
            assert_eq!(ProviderKind::parse(kind.id()), Some(kind));
            assert_eq!(
                serde_json::to_value(kind).unwrap(),
                serde_json::Value::String(kind.id().into())
            );
        }
        assert_eq!(ProviderKind::parse("openai"), None);
    }

    #[test]
    fn error_messages_are_extracted_redacted_and_bounded() {
        let body = br#"{"type":"error","error":{"type":"authentication_error","message":"bad key sk-secret"}}"#;
        let message = error_message_from_body(body);
        assert_eq!(redact(&message, Some("sk-secret")), "bad key [redacted]");
        assert!(bounded(&"x".repeat(1000)).chars().count() <= MAX_ERROR_MESSAGE_CHARS + 1);
    }
}
