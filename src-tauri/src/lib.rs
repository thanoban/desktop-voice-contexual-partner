mod audio;
mod commands;
mod context;
mod db;
mod llm;
mod safety;
mod tts;

use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

// ── App state ────────────────────────────────────────────────────────────────

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub recording: Mutex<Option<ActiveRecording>>,
    pub context: Mutex<SharedContext>,
}

/// Holds the "stop" signal and the WAV file path for an in-progress recording.
pub struct ActiveRecording {
    pub stop_flag: Arc<AtomicBool>,
    pub wav_path: PathBuf,
}

/// What the companion currently knows about the user's environment.
#[derive(Default)]
pub struct SharedContext {
    pub window_title: Option<String>,
    pub custom_note: Option<String>,
    pub sharing: bool,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "voicepartner=info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let db_path = app
                .path()
                .app_data_dir()
                .expect("Cannot resolve app data dir")
                .join("voicepartner.db");

            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            tracing::info!("Opening database at {:?}", db_path);

            let conn = rusqlite::Connection::open(&db_path)
                .expect("Failed to open SQLite database");

            db::migrations::run(&conn).expect("Database migration failed");

            app.manage(AppState {
                db: Mutex::new(conn),
                recording: Mutex::new(None),
                context: Mutex::new(SharedContext::default()),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Chat
            commands::chat::send_message,
            commands::chat::get_greeting,
            commands::chat::start_new_session,
            commands::chat::stop_speaking,
            commands::chat::speak_text,
            // Voice (M1)
            commands::voice::start_listening,
            commands::voice::stop_listening,
            commands::voice::get_audio_devices,
            // Context sharing (M1)
            commands::context::get_window_title,
            commands::context::start_sharing_context,
            commands::context::stop_sharing_context,
            commands::context::set_context_note,
            // Settings
            commands::settings::get_settings,
            commands::settings::update_setting,
            // System
            commands::system::get_ollama_status,
            commands::system::list_models,
        ])
        .run(tauri::generate_context!())
        .expect("Error running VoicePartner");
}

#[allow(dead_code)]
pub fn get_setting_from_handle(app: &AppHandle, key: &str) -> Option<String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().ok()?;
    db::get_setting(&conn, key)
}
