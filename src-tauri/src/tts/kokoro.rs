use anyhow::{anyhow, Result};
use std::process::Command;
use tauri::{AppHandle, Emitter};

// Inline Python script executed via `python -c`. Uses only stdlib + kokoro_onnx + numpy.
const KOKORO_SCRIPT: &str = r#"
import sys, wave
try:
    from kokoro_onnx import Kokoro
except ImportError:
    print("ERROR: kokoro_onnx not installed. Run: pip install kokoro-onnx", file=sys.stderr)
    sys.exit(1)
import numpy as np
model_path, voices_path, voice, speed, out_path = sys.argv[1], sys.argv[2], sys.argv[3], float(sys.argv[4]), sys.argv[5]
text = " ".join(sys.argv[6:])
kok = Kokoro(model_path, voices_path)
samples, sr = kok.create(text, voice=voice, speed=speed, lang="en-us")
samples_i16 = (np.clip(samples, -1.0, 1.0) * 32767).astype(np.int16)
with wave.open(out_path, "wb") as wf:
    wf.setnchannels(1); wf.setsampwidth(2); wf.setframerate(int(sr)); wf.writeframes(samples_i16.tobytes())
"#;

/// Speak `text` using Kokoro TTS via a Python subprocess.
/// Requires `python` on PATH and `pip install kokoro-onnx`.
pub async fn speak_kokoro(
    app: &AppHandle,
    model_path: &str,
    voices_path: &str,
    voice_name: &str,
    text: &str,
    speed: f32,
) -> Result<()> {
    if model_path.is_empty() || voices_path.is_empty() {
        return Err(anyhow!(
            "Kokoro model not configured. Download kokoro-v1.0.onnx and voices.bin \
             in Settings › Voice › Kokoro."
        ));
    }

    let _ = app.emit("tts:start", ());
    let out_file = super::temp_wav_path();
    let speed_str = format!("{:.2}", speed.clamp(0.5, 2.0));

    // Split text into words as separate argv entries to avoid shell quoting issues
    let words: Vec<&str> = text.split_whitespace().collect();

    let mut args = vec![
        "-c",
        KOKORO_SCRIPT,
        model_path,
        voices_path,
        voice_name,
        &speed_str,
        out_file.to_str().unwrap_or(""),
    ];
    args.extend_from_slice(&words);

    let status = tokio::task::spawn_blocking({
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        move || {
            Command::new("python")
                .args(&args)
                .status()
        }
    })
    .await
    .map_err(|e| anyhow!("Kokoro task panicked: {}", e))?
    .map_err(|e| {
        anyhow!(
            "Failed to launch Python for Kokoro TTS: {}. \
             Make sure Python is installed and on PATH.",
            e
        )
    })?;

    if !status.success() {
        return Err(anyhow!(
            "Kokoro TTS failed. Ensure kokoro-onnx is installed: pip install kokoro-onnx"
        ));
    }

    super::play_wav(&out_file)?;
    let _ = std::fs::remove_file(&out_file);
    let _ = app.emit("tts:end", ());
    Ok(())
}

/// Check whether Python and kokoro_onnx are available on this system.
/// Returns `(python_ok, kokoro_ok)`.
pub fn check_kokoro_available() -> (bool, bool) {
    let python_ok = Command::new("python")
        .args(["-c", "import sys; sys.exit(0)"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !python_ok {
        return (false, false);
    }

    let kokoro_ok = Command::new("python")
        .args(["-c", "import kokoro_onnx"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    (python_ok, kokoro_ok)
}

/// The 10 Kokoro voice IDs shipped with kokoro-onnx v1.0.
#[allow(dead_code)]
pub const KOKORO_VOICE_IDS: &[&str] = &[
    "af_bella", "af_sarah", "af_sky", "af_nicole",
    "am_adam",  "am_michael",
    "bf_emma",  "bf_isabella",
    "bm_george","bm_lewis",
];
