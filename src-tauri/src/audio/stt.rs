use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Command;

/// Transcribe a WAV file using a whisper.cpp subprocess.
/// `binary`: path to whisper.cpp main executable (or name on PATH)
/// `model`:  path to .bin model file (ggml-base.en.bin etc.)
pub async fn transcribe(wav_path: &PathBuf, binary: &str, model: &str) -> Result<String> {
    if binary.is_empty() {
        return Err(anyhow!(
            "Whisper binary not configured. Set it in Settings > Voice > Whisper binary."
        ));
    }
    if model.is_empty() {
        return Err(anyhow!(
            "Whisper model not configured. Set it in Settings > Voice > Whisper model."
        ));
    }

    // Run synchronously in a blocking task so we don't block the async executor
    let wav = wav_path.to_owned();
    let bin = binary.to_owned();
    let mdl = model.to_owned();

    tokio::task::spawn_blocking(move || run_whisper(&wav, &bin, &mdl))
        .await
        .map_err(|e| anyhow!("Transcription task panicked: {}", e))?
}

fn run_whisper(wav_path: &PathBuf, binary: &str, model: &str) -> Result<String> {
    let output = Command::new(binary)
        .args([
            "-m", model,
            "-f", wav_path.to_str().unwrap_or(""),
            "-nt",           // no timestamps
            "-l", "en",
            "--no-prints",   // suppress status output
            "-otxt",         // write .txt file alongside WAV
        ])
        .output()
        .map_err(|e| anyhow!("Failed to run whisper binary '{}': {}", binary, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Whisper failed: {}", stderr));
    }

    // Whisper with -otxt writes a .txt file next to the .wav
    let txt_path = wav_path.with_extension("txt");
    if txt_path.exists() {
        let text = std::fs::read_to_string(&txt_path)
            .unwrap_or_default()
            .trim()
            .to_string();
        let _ = std::fs::remove_file(&txt_path);
        if !text.is_empty() {
            return Ok(text);
        }
    }

    // Fall back to stdout
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        Err(anyhow!("Whisper returned empty transcription"))
    } else {
        Ok(stdout)
    }
}

/// Returns the default location where we look for the whisper model.
pub fn default_model_path() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VoicePartner")
        .join("models")
        .join("ggml-base.en.bin")
}

#[allow(dead_code)]
pub fn model_exists(model_path: &str) -> bool {
    if model_path.is_empty() {
        default_model_path().exists()
    } else {
        PathBuf::from(model_path).exists()
    }
}
