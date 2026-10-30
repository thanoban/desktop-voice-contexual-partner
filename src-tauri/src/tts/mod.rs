pub mod kokoro;
pub mod piper;
pub mod playback;

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Emitter;

pub(crate) struct SpeechActivity {
    _audio: crate::conversation::controller::AudioLease,
    app: tauri::AppHandle,
    timer: Option<crate::telemetry::timing::StageTimer>,
}

impl SpeechActivity {
    pub fn begin(app: &tauri::AppHandle) -> Result<Self> {
        let audio = crate::commands::chat::begin_audio(
            app,
            crate::conversation::controller::AudioPhase::Speaking,
        )
        .map_err(anyhow::Error::msg)?;
        let _ = app.emit("tts:start", ());
        Ok(Self {
            _audio: audio,
            app: app.clone(),
            timer: Some(crate::telemetry::timing::StageTimer::start(
                "speech_activity",
            )),
        })
    }
}

impl Drop for SpeechActivity {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.take() {
            let timing = timer.finish();
            tracing::info!(
                stage = timing.stage.as_str(),
                duration_ms = timing.duration_ms,
                "voice stage ended"
            );
        }
        let _ = self.app.emit("tts:end", ());
    }
}

pub(crate) struct SpeechFile(pub PathBuf);
impl Drop for SpeechFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// ── Shared audio helpers (used by both piper and kokoro) ─────────────────────

/// Plays a WAV through a tracked subprocess. Returns `Ok(false)` when playback was cancelled.
pub(crate) fn play_wav(path: &Path, epoch: u64) -> Result<bool> {
    let command = playback_command(path);
    match playback::global().run_tracked(command, None, epoch)? {
        playback::RunOutcome::Completed(status) if status.success() => Ok(true),
        playback::RunOutcome::Completed(status) => Err(anyhow::anyhow!(
            "Speech playback failed with status {status}"
        )),
        playback::RunOutcome::Cancelled => Ok(false),
    }
}

/// Runs a synthesis subprocess under cancellation tracking. Returns `Ok(None)` if cancelled.
pub(crate) fn run_synthesis(
    command: Command,
    stdin: Option<&[u8]>,
    epoch: u64,
) -> Result<Option<std::process::ExitStatus>> {
    match playback::global().run_tracked(command, stdin, epoch)? {
        playback::RunOutcome::Completed(status) if status.success() => Ok(Some(status)),
        playback::RunOutcome::Completed(status) => Err(anyhow::anyhow!(
            "Speech synthesis failed with status {status}"
        )),
        playback::RunOutcome::Cancelled => Ok(None),
    }
}

fn playback_command(path: &Path) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "$ErrorActionPreference = 'Stop'; (New-Object Media.SoundPlayer '{}').PlaySync()",
                path.display()
            ),
        ]);
        command
    }
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("afplay");
        command.arg(path);
        command
    }
    #[cfg(target_os = "linux")]
    {
        let mut command = Command::new("aplay");
        command.arg(path);
        command
    }
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

#[cfg(test)]
mod tests {
    #[test]
    fn failed_synthesis_does_not_advance_to_playback() {
        #[cfg(target_os = "windows")]
        let mut command = std::process::Command::new("cmd");
        #[cfg(target_os = "windows")]
        command.args(["/C", "exit 23"]);
        #[cfg(not(target_os = "windows"))]
        let mut command = std::process::Command::new("sh");
        #[cfg(not(target_os = "windows"))]
        command.args(["-c", "exit 23"]);
        let epoch = super::playback::global().epoch();
        let error = super::run_synthesis(command, None, epoch).unwrap_err();
        assert!(error.to_string().contains("Speech synthesis failed"));
    }
}
