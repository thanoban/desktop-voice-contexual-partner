use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::conversation::controller::CancellationToken;

pub use super::provider::StreamChatResult;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
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

/// Generate a chat reply through the configured provider (see `router.rs`).
/// `endpoint` and `model` are the local Ollama settings; the routing profile
/// decides whether a remote provider may be used instead. There is no
/// fallback between providers.
pub async fn stream_chat(
    app: &AppHandle,
    endpoint: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    cancellation: &CancellationToken,
    project_id: &str,
    turn_id: &str,
) -> Result<StreamChatResult> {
    super::router::stream_chat(
        app,
        endpoint,
        model,
        messages,
        cancellation,
        project_id,
        turn_id,
    )
    .await
}

pub(crate) fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .build()
        .map_err(|e| anyhow!("HTTP client build failed: {}", e))
}
