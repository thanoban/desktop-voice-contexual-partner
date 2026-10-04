pub mod kokoro;
pub mod piper;
pub mod playback;

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

// ── Shared audio helpers (used by both piper and kokoro) ─────────────────────

/// Plays a WAV through a tracked subprocess. Returns `Ok(false)` when playback was cancelled.
pub(crate) fn play_wav(path: &Path, epoch: u64) -> Result<bool> {
    let command = playback_command(path);
    match playback::global().run_tracked(command, None, epoch)? {
        playback::RunOutcome::Completed(_) => Ok(true),
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
        playback::RunOutcome::Completed(status) => Ok(Some(status)),
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
                "(New-Object Media.SoundPlayer '{}').PlaySync()",
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
