use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

/// Calls Ollama /api/embeddings and returns the embedding vector.
/// Returns Err if the model is not available — callers should handle silently.
pub async fn embed_text(endpoint: &str, model: &str, text: &str) -> Result<Vec<f32>> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/embeddings", endpoint.trim_end_matches('/'));

    let resp = client
        .post(&url)
        .json(&serde_json::json!({ "model": model, "prompt": text }))
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| anyhow!("Embedding request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Ollama embeddings {} — {}", status, body));
    }

    let data: EmbedResponse = resp
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse embedding response: {}", e))?;

    if data.embedding.is_empty() {
        return Err(anyhow!("Empty embedding returned by model '{}'", model));
    }

    Ok(data.embedding)
}
