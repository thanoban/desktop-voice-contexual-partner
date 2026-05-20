use crate::audio;
use crate::db;
use crate::AppState;
use crate::ActiveRecording;
use tauri::{AppHandle, Emitter, State};
use std::sync::Arc;
use std::path::PathBuf;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
}

#[tauri::command]
pub fn get_audio_devices() -> Vec<AudioDevice> {
    let names = audio::capture::list_input_devices();
    names
        .into_iter()
        .enumerate()
        .map(|(i, name)| AudioDevice { name, is_default: i == 0 })
        .collect()
}

#[tauri::command]
pub async fn start_listening(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // Don't start if already recording
    {
        let rec = state.recording.lock().unwrap();
        if rec.is_some() {
            return Ok(());
        }
    }

    let device_name = {
        let db = state.db.lock().unwrap();
        db::get_setting(&db, "audio_input_device").unwrap_or_default()
    };

    let wav_path = temp_wav_path();

    let stop_flag = audio::capture::start_recording(&device_name, wav_path.clone())
        .map_err(|e| e.to_string())?;

    {
        let mut rec = state.recording.lock().unwrap();
        *rec = Some(ActiveRecording { stop_flag: Arc::clone(&stop_flag), wav_path });
    }

    let _ = app.emit("audio:listening", true);
    Ok(())
}

#[tauri::command]
pub async fn stop_listening(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let recording = {
        let mut rec = state.recording.lock().unwrap();
        rec.take()
    };

    let recording = match recording {
        Some(r) => r,
        None => return Err("Not recording".into()),
    };

    // Signal the recording thread to stop
    recording.stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);

    let _ = app.emit("audio:listening", false);

    // Give the recording thread a moment to flush the WAV
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let _ = app.emit("audio:processing", ());

    let (whisper_binary, whisper_model) = {
        let db = state.db.lock().unwrap();
        let bin = db::get_setting(&db, "whisper_binary").unwrap_or_default();
        let mdl = db::get_setting(&db, "whisper_model")
            .unwrap_or_else(|| audio::stt::default_model_path().to_string_lossy().to_string());
        (bin, mdl)
    };

    let text = audio::stt::transcribe(&recording.wav_path, &whisper_binary, &whisper_model)
        .await
        .map_err(|e| {
            let _ = app.emit("audio:error", e.to_string());
            e.to_string()
        })?;

    let _ = std::fs::remove_file(&recording.wav_path);
    let _ = app.emit("audio:processing", false);

    Ok(text)
}

fn temp_wav_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "vp_rec_{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ))
}
