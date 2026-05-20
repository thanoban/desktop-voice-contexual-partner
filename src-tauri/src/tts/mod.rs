pub mod kokoro;
pub mod piper;

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Command;

// ── Shared audio helpers (used by both piper and kokoro) ─────────────────────

pub(crate) fn play_wav(path: &PathBuf) -> Result<()> {
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

pub(crate) fn temp_wav_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "vp_tts_{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ))
}
