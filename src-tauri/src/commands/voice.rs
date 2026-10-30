use crate::audio;
use crate::conversation::controller::AudioPhase;
use crate::db;
use crate::ActiveRecording;
use crate::AppState;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

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
        .map(|(i, name)| AudioDevice {
            name,
            is_default: i == 0,
        })
        .collect()
}

#[tauri::command]
pub async fn start_listening(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    // Don't start if already recording
    {
        let rec = state.recording.lock().unwrap();
        if rec.is_some() {
            return Ok(());
        }
    }

    let scope_lease = state.conversation.begin()?.with_events(app.clone());
    let audio = super::chat::begin_audio(&app, AudioPhase::Listening)?;
    let scope_lease = scope_lease.with_audio(audio);
    let device_name = {
        let db = state.db.lock().unwrap();
        db::get_setting(&db, "audio_input_device").unwrap_or_default()
    };

    let wav_path = temp_wav_path();

    let recording = audio::capture::start_recording(&device_name, wav_path.clone())
        .map_err(|e| e.to_string())?;
    let owner_id = scope_lease.id().to_string();
    let (completed_sender, completed_receiver) = tokio::sync::oneshot::channel();

    {
        let mut rec = state.recording.lock().unwrap();
        *rec = Some(ActiveRecording {
            scope_lease,
            stop_flag: Arc::clone(&recording.stop_flag),
            wav_path: wav_path.clone(),
            completed: completed_receiver,
        });
    }

    // Observe worker exit even when no window sends stop (device failure or max duration).
    let monitor_app = app.clone();
    let monitor_path = wav_path.clone();
    tokio::spawn(async move {
        let result = recording
            .completed
            .await
            .unwrap_or_else(|_| Err("Recording worker stopped before finalizing audio".into()));
        let abandoned = {
            let state = monitor_app.state::<AppState>();
            let mut current = state.recording.lock().unwrap();
            if current
                .as_ref()
                .is_some_and(|recording| recording.scope_lease.id() == owner_id)
            {
                current.take()
            } else {
                None
            }
        };
        let ended_without_stop = abandoned.is_some();
        if ended_without_stop
            && !abandoned
                .as_ref()
                .is_some_and(|recording| recording.scope_lease.cancellation().is_cancelled())
        {
            let message = result.as_ref().err().cloned().unwrap_or_else(|| {
                "Recording finished before it was submitted. Please start a new recording.".into()
            });
            monitor_app.state::<AppState>().conversation.fail(message);
        }
        drop(abandoned);
        if ended_without_stop || completed_sender.send(result).is_err() {
            let _ = std::fs::remove_file(monitor_path);
        }
    });

    let _ = app.emit("audio:listening", true);
    Ok(())
}

#[tauri::command]
pub async fn stop_listening(state: State<'_, AppState>, app: AppHandle) -> Result<String, String> {
    let recording = {
        let mut rec = state.recording.lock().unwrap();
        rec.take()
    };

    let recording = match recording {
        Some(r) => r,
        None => return Err("Not recording".into()),
    };
    let _recording_file = RecordingFile(recording.wav_path.clone());
    recording
        .scope_lease
        .set_audio_phase(AudioPhase::Transcribing);

    // Signal the recording thread to stop
    recording
        .stop_flag
        .store(true, std::sync::atomic::Ordering::Relaxed);

    let _ = app.emit("audio:listening", false);

    let completion = tokio::time::timeout(tokio::time::Duration::from_secs(5), recording.completed)
        .await
        .map_err(|_| {
            let message = "Timed out while finalizing the recording".to_string();
            state.conversation.fail(message.clone());
            message
        })?
        .map_err(|_| {
            let message = "Recording worker stopped before finalizing audio".to_string();
            state.conversation.fail(message.clone());
            message
        })?;

    if let Err(error) = completion {
        state.conversation.fail(error.clone());
        let _ = std::fs::remove_file(&recording.wav_path);
        let _ = app.emit("audio:error", &error);
        return Err(error);
    }

    let _ = app.emit("audio:processing", true);
    if recording.scope_lease.cancellation().is_cancelled() {
        return Ok(String::new());
    }

    let (whisper_binary, whisper_model) = {
        let db = state.db.lock().unwrap();
        let bin = db::get_setting(&db, "whisper_binary").unwrap_or_default();
        let mdl = db::get_setting(&db, "whisper_model").unwrap_or_else(|| {
            audio::stt::default_model_path()
                .to_string_lossy()
                .to_string()
        });
        (bin, mdl)
    };

    let transcription_timer = crate::telemetry::timing::StageTimer::start("transcription");
    let transcription = audio::stt::transcribe(
        &recording.wav_path,
        &whisper_binary,
        &whisper_model,
        &recording.scope_lease.cancellation(),
    )
    .await;

    let timing = transcription_timer.finish();
    tracing::info!(
        stage = timing.stage.as_str(),
        duration_ms = timing.duration_ms,
        outcome = if recording.scope_lease.cancellation().is_cancelled() {
            "cancelled"
        } else if transcription.is_err() {
            "failed"
        } else {
            "completed"
        },
        "voice stage elapsed"
    );
    let _ = std::fs::remove_file(&recording.wav_path);
    let _ = app.emit("audio:processing", false);

    if recording.scope_lease.cancellation().is_cancelled() {
        return Ok(String::new());
    }
    transcription.map_err(|error| {
        let message = error.to_string();
        state.conversation.fail(message.clone());
        let _ = app.emit("audio:error", &message);
        message
    })
}

struct RecordingFile(PathBuf);
impl Drop for RecordingFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
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
