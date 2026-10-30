use crate::conversation::controller::CancellationToken;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

const OUTPUT_LIMIT: usize = 1024 * 1024;

struct TranscriptFile(PathBuf);
impl Drop for TranscriptFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Cancellation kills and reaps the Whisper child before returning an empty transcript.
pub async fn transcribe(
    wav_path: &Path,
    binary: &str,
    model: &str,
    cancellation: &CancellationToken,
) -> Result<String> {
    let txt_path = wav_path.with_extension("txt");
    let _cleanup = TranscriptFile(txt_path.clone());
    if cancellation.is_cancelled() {
        return Ok(String::new());
    }
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
    let command = whisper_command(wav_path, binary, model);
    transcribe_command(command, &txt_path, cancellation).await
}

fn whisper_command(wav_path: &Path, binary: &str, model: &str) -> Command {
    let mut command = Command::new(binary);
    command.args([
        "-m",
        model,
        "-f",
        wav_path.to_str().unwrap_or(""),
        "-nt",
        "-l",
        "en",
        "--no-prints",
        "-otxt",
    ]);
    // Whisper appends .txt to its output base; without -of it appends to
    // the full input filename (.wav.txt), outside our tracked cleanup path.
    command.arg("-of").arg(wav_path.with_extension(""));
    command
}

async fn transcribe_command(
    command: Command,
    txt_path: &Path,
    cancellation: &CancellationToken,
) -> Result<String> {
    let _cleanup = TranscriptFile(txt_path.to_owned());
    let Some(output) = run_process(command, cancellation).await? else {
        return Ok(String::new());
    };
    if txt_path.exists() {
        let file = tokio::fs::File::open(txt_path).await?;
        let mut bytes = Vec::new();
        file.take((OUTPUT_LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .await?;
        if bytes.len() > OUTPUT_LIMIT {
            return Err(anyhow!("Whisper transcript exceeded size limit"));
        }
        let text = String::from_utf8_lossy(&bytes).trim().to_string();
        if !text.is_empty() {
            return Ok(text);
        }
    }
    let text = String::from_utf8_lossy(&output).trim().to_string();
    if text.is_empty() {
        Err(anyhow!("Whisper returned empty transcription"))
    } else {
        Ok(text)
    }
}

async fn run_process(
    mut command: Command,
    cancellation: &CancellationToken,
) -> Result<Option<Vec<u8>>> {
    if cancellation.is_cancelled() {
        return Ok(None);
    }
    let mut child = command
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Missing Whisper stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("Missing Whisper stderr"))?;
    let collect = async {
        let read = |pipe| async move {
            let mut bytes = Vec::new();
            tokio::io::BufReader::new(pipe)
                .take((OUTPUT_LIMIT + 1) as u64)
                .read_to_end(&mut bytes)
                .await?;
            if bytes.len() > OUTPUT_LIMIT {
                return Err(anyhow!("Whisper output exceeded size limit"));
            }
            Ok::<_, anyhow::Error>(bytes)
        };
        // Different pipe types require erasing their concrete reader types.
        let stdout: Box<dyn tokio::io::AsyncRead + Unpin + Send> = Box::new(stdout);
        let stderr: Box<dyn tokio::io::AsyncRead + Unpin + Send> = Box::new(stderr);
        let (out, err) = tokio::try_join!(read(stdout), read(stderr))?;
        Ok::<_, anyhow::Error>((out, err))
    };
    let output = tokio::select! {
        biased;
        _ = cancellation.cancelled() => { None }
        result = collect => { Some(result) }
    };
    let Some(output) = output else {
        child.kill().await?;
        child.wait().await?;
        return Ok(None);
    };
    let (stdout, stderr) = match output {
        Ok(output) => output,
        Err(error) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(error);
        }
    };
    let status = tokio::select! {
        biased;
        _ = cancellation.cancelled() => { child.kill().await?; child.wait().await?; return Ok(None); }
        status = child.wait() => status?,
    };
    if !status.success() {
        return Err(anyhow!(
            "Whisper failed: {}",
            String::from_utf8_lossy(&stderr)
        ));
    }
    Ok(Some(stdout))
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
#[cfg(test)]
mod process_tests {
    use super::*;

    // Reuse this test executable as a shell-free fake Whisper child. Normal
    // suite execution has no fixture environment and performs no writes.
    #[test]
    fn fake_whisper_executable() {
        let Some(base) = std::env::var_os("VP_STT_FIXTURE_OUTPUT_BASE") else {
            return;
        };
        let mut sidecar = base;
        sidecar.push(".txt");
        std::fs::write(PathBuf::from(sidecar), "  fixture transcript  ").unwrap();
    }

    #[tokio::test]
    async fn explicit_output_base_roundtrip_reads_and_removes_actual_whisper_sidecar() {
        let wav = std::env::temp_dir().join(format!("vp-stt output-{}.wav", uuid::Uuid::new_v4()));
        let expected_sidecar = wav.with_extension("txt");
        let production = whisper_command(&wav, "unused-whisper", "unused-model");
        let args: Vec<_> = production.as_std().get_args().collect();
        let output_index = args.iter().position(|arg| *arg == "-of").unwrap();
        let output_base = args[output_index + 1];
        assert_eq!(Path::new(output_base), wav.with_extension(""));
        let mut actual_sidecar = output_base.to_os_string();
        actual_sidecar.push(".txt");
        assert_eq!(Path::new(&actual_sidecar), expected_sidecar);

        let mut child = Command::new(std::env::current_exe().unwrap());
        child.args([
            "--exact",
            "audio::stt::process_tests::fake_whisper_executable",
            "--nocapture",
        ]);
        child.env("VP_STT_FIXTURE_OUTPUT_BASE", output_base);
        let token = CancellationToken::default();
        let text = transcribe_command(child, &expected_sidecar, &token)
            .await
            .unwrap();
        assert_eq!(text, "fixture transcript");
        assert!(!expected_sidecar.exists());
        assert!(!wav.with_extension("wav.txt").exists());
    }
    #[tokio::test]
    async fn precancelled_request_never_spawns_even_an_invalid_executable() {
        let token = CancellationToken::default();
        token.cancel();
        let command = Command::new("voicepartner-nonexistent-whisper-test-executable");
        assert!(run_process(command, &token).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn oversized_subprocess_output_fails_instead_of_growing_unbounded() {
        let token = CancellationToken::default();
        #[cfg(windows)]
        let command = fake("[Console]::Out.Write(('x' * 1048578))");
        #[cfg(not(windows))]
        let command = fake("head -c 1048578 /dev/zero");
        let error = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            run_process(command, &token),
        )
        .await
        .unwrap()
        .unwrap_err();
        assert!(error.to_string().contains("exceeded size limit"));
    }

    #[tokio::test]
    async fn transcript_sidecar_is_removed_after_validation_failure_and_precancellation() {
        let wav = std::env::temp_dir().join(format!("vp-stt-cleanup-{}.wav", uuid::Uuid::new_v4()));
        let sidecar = wav.with_extension("txt");
        std::fs::write(&sidecar, "private failed transcript").unwrap();
        let token = CancellationToken::default();
        assert!(transcribe(&wav, "", "", &token).await.is_err());
        assert!(!sidecar.exists());
        std::fs::write(&sidecar, "private cancelled transcript").unwrap();
        token.cancel();
        assert_eq!(
            transcribe(&wav, "invalid-executable", "model", &token)
                .await
                .unwrap(),
            ""
        );
        assert!(!sidecar.exists());
    }
    fn fake(script: &str) -> Command {
        #[cfg(windows)]
        let mut command = Command::new("powershell");
        #[cfg(windows)]
        command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        #[cfg(not(windows))]
        let mut command = Command::new("sh");
        #[cfg(not(windows))]
        command.args(["-c", script]);
        command
    }
    #[tokio::test]
    async fn subprocess_success_and_failure() {
        let token = CancellationToken::default();
        assert!(!run_process(fake("echo hello"), &token)
            .await
            .unwrap()
            .unwrap()
            .is_empty());
        assert!(run_process(fake("exit 7"), &token).await.is_err());
    }
    #[tokio::test]
    async fn subprocess_cancel_kills_and_reaps_promptly() {
        let token = CancellationToken::default();
        #[cfg(windows)]
        let command = fake("Start-Sleep -Seconds 30");
        #[cfg(not(windows))]
        let command = fake("exec sleep 30");
        let cancel = async {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            token.cancel();
        };
        let run = run_process(command, &token);
        let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            tokio::join!(run, cancel)
        })
        .await
        .unwrap();
        assert!(result.unwrap().is_none());
    }
}
