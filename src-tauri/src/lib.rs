mod commands;
mod db;
mod llm;
mod safety;
mod tts;

use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
}

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

            // Create app data dir if it doesn't exist
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            tracing::info!("Opening database at {:?}", db_path);

            let conn = rusqlite::Connection::open(&db_path)
                .expect("Failed to open SQLite database");

            db::migrations::run(&conn)
                .expect("Database migration failed");

            app.manage(AppState {
                db: Mutex::new(conn),
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

/// Convenience: get setting value from AppHandle (for use in setup hooks)
#[allow(dead_code)]
pub fn get_setting_from_handle(app: &AppHandle, key: &str) -> Option<String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().ok()?;
    db::get_setting(&conn, key)
}
