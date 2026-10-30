//! Ollama `/api/chat` NDJSON streaming adapter. Behaviour (request body,
//! timeout, error text, terminal-frame requirement) is unchanged from the
//! original single-provider client.

use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use serde::Deserialize;

use super::client::ChatMessage;
use super::provider::{
    text_capabilities, ChatProvider, DataDestination, ProviderKind, StreamChatResult, TokenSink,
};
use crate::conversation::controller::CancellationToken;
use crate::domain::provider::ProviderCapabilities;

pub struct OllamaProvider {
    pub client: reqwest::Client,
    pub endpoint: String,
    pub model: String,
    pub destination: DataDestination,
}

#[derive(Debug, Deserialize)]
struct OllamaChatChunk {
    message: Option<OllamaChunkMessage>,
    #[serde(default)]
    done: bool,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaChunkMessage {
    content: String,
}

impl ChatProvider for OllamaProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        text_capabilities(ProviderKind::Ollama, &self.model)
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
        let url = format!("{}/api/chat", self.endpoint.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            "options": {
                "temperature": 0.85,
                "num_predict": 512
            }
        });

        let request = self
            .client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send();
        let response = tokio::select! {
            _ = cancellation.cancelled() => {
                return Ok(StreamChatResult::cancelled(String::new()));
            }
            response = request => response.map_err(|e| anyhow!("Ollama request failed: {}", e))?,
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama error {}: {}", status, body));
        }

        let mut full_content = String::new();
        let mut stream = response.bytes_stream();
        let mut decoder = NdjsonDecoder::default();
        let mut terminal = false;

        loop {
            let next = tokio::select! {
                _ = cancellation.cancelled() => {
                    return Ok(StreamChatResult::cancelled(full_content));
                }
                next = stream.next() => next,
            };
            let Some(chunk) = next else { break };
            let bytes = chunk.map_err(|e| anyhow!("Stream error: {}", e))?;
            for chunk in decoder.push(&bytes)? {
                terminal = consume_chunk(chunk, &mut full_content, sink)?;
                if terminal {
                    break;
                }
            }
            if terminal {
                break;
            }
        }

        if !terminal {
            for chunk in decoder.finish()? {
                terminal = consume_chunk(chunk, &mut full_content, sink)?;
            }
        }

        if !terminal {
            return Err(anyhow!("Ollama stream ended without a terminal frame"));
        }

        Ok(StreamChatResult::completed(full_content))
    }
}

fn consume_chunk(
    chunk: OllamaChatChunk,
    full_content: &mut String,
    sink: &mut TokenSink<'_>,
) -> Result<bool> {
    if let Some(error) = chunk.error {
        return Err(anyhow!("Ollama stream error: {}", error));
    }
    if let Some(message) = chunk.message {
        if !message.content.is_empty() {
            full_content.push_str(&message.content);
            sink(&message.content);
        }
    }
    Ok(chunk.done)
}

const MAX_NDJSON_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Default)]
struct NdjsonDecoder {
    buffer: Vec<u8>,
}

impl NdjsonDecoder {
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<OllamaChatChunk>> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > MAX_NDJSON_FRAME_BYTES && !self.buffer.contains(&b'\n') {
            return Err(anyhow!("Ollama stream frame exceeded the size limit"));
        }
        self.take_complete_lines()
    }

    fn finish(&mut self) -> Result<Vec<OllamaChatChunk>> {
        let mut chunks = self.take_complete_lines()?;
        if !self.buffer.iter().all(u8::is_ascii_whitespace) {
            chunks.push(parse_line(&self.buffer)?);
        }
        self.buffer.clear();
        Ok(chunks)
    }

    fn take_complete_lines(&mut self) -> Result<Vec<OllamaChatChunk>> {
        let mut chunks = Vec::new();
        while let Some(newline) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=newline).collect();
            let line = &line[..line.len() - 1];
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            chunks.push(parse_line(line)?);
        }
        Ok(chunks)
    }
}

fn parse_line(line: &[u8]) -> Result<OllamaChatChunk> {
    serde_json::from_slice(line).map_err(|error| anyhow!("Invalid Ollama stream frame: {}", error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::Locality;
    use crate::llm::test_support::{serve_once, test_client, FakeResponse};

    #[test]
    fn decoder_handles_split_json_and_utf8() {
        let payload = "{\"message\":{\"content\":\"hello 🌍\"},\"done\":false}\n";
        let bytes = payload.as_bytes();
        let split = payload.find('🌍').expect("emoji") + 1;
        let mut decoder = NdjsonDecoder::default();

        assert!(decoder
            .push(&bytes[..split])
            .expect("first fragment")
            .is_empty());
        let chunks = decoder.push(&bytes[split..]).expect("second fragment");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].message.as_ref().unwrap().content, "hello 🌍");
    }

    #[test]
    fn decoder_reads_multiple_frames_per_chunk() {
        let mut decoder = NdjsonDecoder::default();
        let chunks = decoder
            .push(
                b"{\"message\":{\"content\":\"a\"},\"done\":false}\n{\"message\":null,\"done\":true}\n",
            )
            .expect("frames");
        assert_eq!(chunks.len(), 2);
        assert!(chunks[1].done);
    }

    #[test]
    fn decoder_rejects_malformed_complete_frame() {
        let mut decoder = NdjsonDecoder::default();
        assert!(decoder.push(b"not-json\n").is_err());
    }

    #[test]
    fn decoder_accepts_terminal_frame_without_newline() {
        let mut decoder = NdjsonDecoder::default();
        decoder
            .push(b"{\"message\":null,\"done\":true}")
            .expect("partial terminal frame");
        let chunks = decoder.finish().expect("finish");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].done);
    }

    #[tokio::test]
    async fn provider_streams_ndjson_through_the_interface() {
        let mut response = FakeResponse::bytes(vec![
            b"{\"message\":{\"content\":\"Hi\"},\"done\":false}\n{\"message\":{\"con".to_vec(),
            b"tent\":\"!\"},\"done\":false}\n{\"done\":true}\n".to_vec(),
        ]);
        response.content_type = "application/x-ndjson";
        let (base, server) = serve_once(response).await;
        let provider = OllamaProvider {
            client: test_client(),
            endpoint: base.clone(),
            model: "llama3.2:8b".into(),
            destination: DataDestination {
                provider: ProviderKind::Ollama,
                host: "127.0.0.1".into(),
                origin: base,
                locality: Locality::Local,
            },
        };
        let mut tokens = Vec::new();
        let mut sink = |token: &str| tokens.push(token.to_string());
        let result = provider
            .stream_chat(&[], &CancellationToken::default(), &mut sink)
            .await
            .unwrap();
        assert_eq!(result.content, "Hi!");
        assert_eq!(tokens, vec!["Hi", "!"]);
        assert!(server.await.unwrap().starts_with("POST /api/chat "));
        assert!(provider.capabilities().streaming);
    }
}
