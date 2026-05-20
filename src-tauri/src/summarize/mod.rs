use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}

/// Asks Ollama (non-streaming) to produce a 2-3 sentence memory summary
/// of recent conversation turns.
pub async fn summarize_session(
    endpoint: &str,
    model: &str,
    turns: &[(String, String)],
) -> Result<String> {
    if turns.is_empty() {
        return Err(anyhow!("No turns to summarize"));
    }

    let transcript = turns
        .iter()
        .map(|(role, content)| format!("{}: {}", role, content))
        .collect::<Vec<_>>()
        .join("\n");

    let user_prompt = format!(
        "Summarize this conversation in 2-3 sentences. Capture:\n\
         - What the user was working on or thinking about\n\
         - Any important personal facts, preferences, or decisions shared\n\
         - Their emotional state if it was notable\n\
         Write in third person about \"the user\". Be specific.\n\n\
         Conversation:\n{}",
        transcript
    );

    let client = reqwest::Client::new();
    let url = format!("{}/api/chat", endpoint.trim_end_matches('/'));

    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "model": model,
            "stream": false,
            "messages": [
                {
                    "role": "system",
                    "content": "You extract precise, factual memory summaries from conversations. \
                                Never invent details not present in the transcript."
                },
                { "role": "user", "content": user_prompt }
            ]
        }))
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| anyhow!("Summarize request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(anyhow!("Summarize API error: {}", resp.status()));
    }

    let data: OllamaResponse = resp
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse summary: {}", e))?;

    let summary = data.message.content.trim().to_string();
    if summary.is_empty() {
        return Err(anyhow!("Empty summary returned"));
    }

    Ok(summary)
}
