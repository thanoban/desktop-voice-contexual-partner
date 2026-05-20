mod audio;
mod commands;
mod context;
mod db;
mod embed;
mod llm;
mod memory;
mod rag;
mod safety;
mod summarize;
mod tts;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

// ── App state ─────────────────────────────────────────────────────────────────

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub recording: Mutex<Option<ActiveRecording>>,
    pub context: Mutex<SharedContext>,
}

pub struct ActiveRecording {
    pub stop_flag: Arc<AtomicBool>,
    pub wav_path: PathBuf,
}

#[derive(Default)]
pub struct SharedContext {
    pub window_title: Option<String>,
    pub custom_note: Option<String>,
    pub sharing: bool,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn toggle_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

fn show_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if !win.is_visible().unwrap_or(true) {
            let _ = win.show();
        }
        let _ = win.set_focus();
    }
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
            // ── Database ──────────────────────────────────────────────────────
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

            // ── System tray ───────────────────────────────────────────────────
            let toggle_item = MenuItem::with_id(app, "toggle", "Show / Hide", true, None::<&str>)?;
            let sep         = PredefinedMenuItem::separator(app)?;
            let quit_item   = MenuItem::with_id(app, "quit", "Quit VoicePartner", true, None::<&str>)?;
            let menu        = Menu::with_items(app, &[&toggle_item, &sep, &quit_item])?;

            let tray_icon = app
                .default_window_icon()
                .cloned()
                .unwrap_or_else(|| tauri::image::Image::new_owned(vec![0, 0, 0, 0], 1, 1));

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&menu)
                .tooltip("VoicePartner")
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => toggle_window(app),
                    "quit"   => app.exit(0),
                    _        => {}
                })
                .build(app)?;

            // ── Close → hide to tray ──────────────────────────────────────────
            if let Some(window) = app.get_webview_window("main") {
                let win = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });
            }

            // ── Global PTT shortcut: Alt+Space ────────────────────────────────
            let ptt = Shortcut::new(Some(Modifiers::ALT), Code::Space);
            app.global_shortcut().on_shortcut(ptt, |app_handle, _shortcut, event| {
                match event.state {
                    ShortcutState::Pressed => {
                        show_window(app_handle);
                        let _ = app_handle.emit("shortcut:ptt:start", ());
                    }
                    ShortcutState::Released => {
                        let _ = app_handle.emit("shortcut:ptt:end", ());
                    }
                }
            })?;

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
            // Memory (M2)
            commands::memory::get_memories,
            commands::memory::get_memory_count,
            commands::memory::delete_memory,
            commands::memory::forget_all,
            // RAG — document ingestion (M3)
            commands::rag::pick_document,
            commands::rag::ingest_document,
            commands::rag::list_documents,
            commands::rag::delete_document,
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
