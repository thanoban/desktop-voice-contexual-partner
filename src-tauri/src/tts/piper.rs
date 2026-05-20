use anyhow::{anyhow, Result};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::AppHandle;
use tauri::Emitter;

/// Speaks `text` using Piper TTS.
/// Falls back silently if piper is not configured or not found.
pub async fn speak(app: &AppHandle, piper_binary: &str, voice: &str, text: &str) -> Result<()> {
    let binary = resolve_binary(piper_binary)?;
    let voice_path = resolve_voice(voice)?;
    let out_file = temp_wav_path();

    let _ = app.emit("tts:start", ());

    // Piper: read text from stdin, write WAV to file
    let mut child = Command::new(&binary)
        .args([
            "--model",
            voice_path.to_str().unwrap_or(voice),
            "--output_file",
            out_file.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn piper: {}", e))?;

    if let Some(stdin) = child.stdin.take() {
        let mut s = stdin;
        let _ = s.write_all(text.as_bytes());
    }

    child
        .wait()
        .map_err(|e| anyhow!("Piper wait failed: {}", e))?;

    // Play the WAV file using the platform's built-in audio player
    play_wav(&out_file)?;

    let _ = std::fs::remove_file(&out_file);
    let _ = app.emit("tts:end", ());
    Ok(())
}

fn play_wav(path: &PathBuf) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "(New-Object Media.SoundPlayer '{}').PlaySync()",
                    path.display()
                ),
            ])
            .status()
            .map_err(|e| anyhow!("WAV playback failed: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("afplay")
            .arg(path)
            .status()
            .map_err(|e| anyhow!("afplay failed: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        // Try aplay first, fall back to paplay
        if Command::new("aplay").arg(path).status().is_err() {
            Command::new("paplay")
                .arg(path)
                .status()
                .map_err(|e| anyhow!("Audio playback failed: {}", e))?;
        }
    }
    Ok(())
}

fn resolve_binary(configured: &str) -> Result<String> {
    if !configured.is_empty() {
        return Ok(configured.to_string());
    }
    // Check if piper is on PATH
    which("piper").ok_or_else(|| {
        anyhow!(
            "Piper TTS binary not found. Configure the path in Settings > Voice, \
             or add piper to your PATH."
        )
    })
}

fn resolve_voice(voice_id: &str) -> Result<PathBuf> {
    // Voice model files should be in app data dir / voices /
    // For M0, accept either a full path or a bare voice ID
    let p = PathBuf::from(voice_id);
    if p.exists() {
        return Ok(p);
    }
    // Try alongside the piper binary or in a bundled voices dir
    let voices_dir = voices_directory();
    let candidate = voices_dir.join(format!("{}.onnx", voice_id));
    if candidate.exists() {
        return Ok(candidate);
    }
    Err(anyhow!(
        "Piper voice model not found for '{}'. Download voices from \
         https://huggingface.co/rhasspy/piper-voices and place them in {:?}",
        voice_id,
        voices_dir
    ))
}

fn voices_directory() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VoicePartner")
        .join("voices")
}

fn temp_wav_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "vp_tts_{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ))
}

fn which(name: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    let cmd = format!("where {}", name);
    #[cfg(not(target_os = "windows"))]
    let cmd = format!("which {}", name);

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", &cmd]).output()
    } else {
        Command::new("sh").args(["-c", &cmd]).output()
    };

    output.ok().and_then(|o| {
        String::from_utf8(o.stdout)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}
