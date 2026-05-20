use anyhow::{anyhow, Result};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::AppHandle;
use tauri::Emitter;

/// Speaks `text` using Piper TTS or Windows SAPI.
/// If `voice` starts with "sapi:" the remainder is treated as a Windows SAPI
/// voice name and speech is rendered through System.Speech on Windows.
/// Otherwise Piper is used.
pub async fn speak(
    app: &AppHandle,
    piper_binary: &str,
    voice: &str,
    text: &str,
    speed: f32,
    expressiveness: f32,
) -> Result<()> {
    if let Some(sapi_name) = voice.strip_prefix("sapi:") {
        return speak_sapi(app, sapi_name, text).await;
    }

    let binary = resolve_binary(piper_binary)?;
    let voice_path = resolve_voice(voice)?;
    let out_file = temp_wav_path();

    let _ = app.emit("tts:start", ());

    let speed_str = format!("{:.3}", speed.clamp(0.3, 3.0));
    let expr_str  = format!("{:.3}", expressiveness.clamp(0.0, 1.0));

    let mut child = Command::new(&binary)
        .args([
            "--model",
            voice_path.to_str().unwrap_or(voice),
            "--output_file",
            out_file.to_str().unwrap(),
            "--length-scale",
            &speed_str,
            "--noise-scale",
            &expr_str,
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

    play_wav(&out_file)?;
    let _ = std::fs::remove_file(&out_file);
    let _ = app.emit("tts:end", ());
    Ok(())
}

/// Returns the names of all installed Windows SAPI voices.
/// Returns an empty list on non-Windows platforms.
#[tauri::command]
pub fn get_sapi_voices() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Add-Type -AssemblyName System.Speech; \
                 $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
                 $s.GetInstalledVoices() | ForEach-Object { $_.VoiceInfo.Name }",
            ])
            .output();
        match output {
            Ok(o) => String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect(),
            Err(_) => vec![],
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec![]
    }
}

// ── SAPI speech ───────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
async fn speak_sapi(app: &AppHandle, voice_name: &str, text: &str) -> Result<()> {
    let _ = app.emit("tts:start", ());
    let out_file = temp_wav_path();

    // Single-quote escape for PowerShell ('' = literal ')
    let safe_name = voice_name.replace('\'', "''");
    let safe_text = text.replace('\'', "''");
    let wav_path  = out_file.to_str().unwrap_or("").to_string();

    let script = format!(
        "Add-Type -AssemblyName System.Speech; \
         $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
         $s.SelectVoice('{safe_name}'); \
         $s.SetOutputToWaveFile('{wav_path}'); \
         $s.Speak('{safe_text}'); \
         $s.SetOutputToDefaultAudioDevice()"
    );

    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map_err(|e| anyhow!("SAPI speak failed: {}", e))?;

    play_wav(&out_file)?;
    let _ = std::fs::remove_file(&out_file);
    let _ = app.emit("tts:end", ());
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn speak_sapi(_app: &AppHandle, _voice_name: &str, _text: &str) -> Result<()> {
    Err(anyhow!("Windows SAPI TTS is only available on Windows"))
}

// ── Piper helpers ─────────────────────────────────────────────────────────────

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
    which("piper").ok_or_else(|| {
        anyhow!(
            "Piper TTS binary not found. Configure the path in Settings > Voice, \
             or add piper to your PATH."
        )
    })
}

fn resolve_voice(voice_id: &str) -> Result<PathBuf> {
    let p = PathBuf::from(voice_id);
    if p.exists() {
        return Ok(p);
    }
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
