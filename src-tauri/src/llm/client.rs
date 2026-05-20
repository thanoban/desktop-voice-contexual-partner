use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri::Emitter;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatChunk {
    message: Option<OllamaChunkMessage>,
    done: bool,
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
) -> Result<String> {
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

    let response = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| anyhow!("Ollama request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Ollama error {}: {}", status, body));
    }

    let mut full_content = String::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| anyhow!("Stream error: {}", e))?;
        let text = String::from_utf8_lossy(&bytes);

        for line in text.lines() {
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<OllamaChatChunk>(line) {
                Ok(chunk) => {
                    if let Some(msg) = &chunk.message {
                        if !msg.content.is_empty() {
                            full_content.push_str(&msg.content);
                            let _ = app.emit("chat:token", &msg.content);
                        }
                    }
                    if chunk.done {
                        break;
                    }
                }
                Err(_) => continue,
            }
        }
    }

    Ok(full_content)
}

fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .build()
        .map_err(|e| anyhow!("HTTP client build failed: {}", e))
}
