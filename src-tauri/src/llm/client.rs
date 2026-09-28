use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri::Emitter;

use crate::conversation::controller::CancellationToken;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
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

#[derive(Debug, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: Option<u64>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

pub async fn list_models(endpoint: &str) -> Result<Vec<OllamaModel>> {
    let client = build_client()?;
    let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| anyhow!("Cannot reach Ollama at {}: {}", endpoint, e))?;

    let tags: OllamaTagsResponse = resp.json().await?;
    Ok(tags.models)
}

pub async fn stream_chat(
    app: &AppHandle,
    endpoint: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    cancellation: &CancellationToken,
) -> Result<StreamChatResult> {
    let client = build_client()?;
    let url = format!("{}/api/chat", endpoint.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "options": {
            "temperature": 0.85,
            "num_predict": 512
        }
    });

    let request = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send();
    let response = tokio::select! {
        _ = cancellation.cancelled() => {
            return Ok(StreamChatResult { content: String::new(), cancelled: true });
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
                return Ok(StreamChatResult { content: full_content, cancelled: true });
            }
            next = stream.next() => next,
        };
        let Some(chunk) = next else { break };
        let bytes = chunk.map_err(|e| anyhow!("Stream error: {}", e))?;
        for chunk in decoder.push(&bytes)? {
            terminal = consume_chunk(app, chunk, &mut full_content)?;
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
            terminal = consume_chunk(app, chunk, &mut full_content)?;
        }
    }

    if !terminal {
        return Err(anyhow!("Ollama stream ended without a terminal frame"));
    }

    Ok(StreamChatResult {
        content: full_content,
        cancelled: false,
    })
}

pub struct StreamChatResult {
    pub content: String,
    pub cancelled: bool,
}

fn consume_chunk(
    app: &AppHandle,
    chunk: OllamaChatChunk,
    full_content: &mut String,
) -> Result<bool> {
    if let Some(error) = chunk.error {
        return Err(anyhow!("Ollama stream error: {}", error));
    }
    if let Some(message) = chunk.message {
        if !message.content.is_empty() {
            full_content.push_str(&message.content);
            let _ = app.emit("chat:token", &message.content);
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

fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .build()
        .map_err(|e| anyhow!("HTTP client build failed: {}", e))
}

#[cfg(test)]
mod tests {
    use super::NdjsonDecoder;

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
}
