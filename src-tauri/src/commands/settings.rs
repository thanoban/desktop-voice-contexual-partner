use crate::db;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    // LLM
    pub endpoint: String,
    pub model: String,
    // Companion
    pub companion_name: String,
    pub personality: String,
    pub partner_mode: String,
    pub selected_project_id: String,
    pub onboarding_done: String,
    // TTS
    pub piper_binary: String,
    pub piper_voice: String,
    // Voice input (M1)
    pub audio_input_device: String,
    pub whisper_binary: String,
    pub whisper_model: String,
    pub voice_threshold_db: String,
    // Context sharing (M1)
    pub window_context_auto: String,
    pub window_context_allowed: String,
    // Voice tone (M1.1)
    pub voice_speed: String,
    pub voice_expressiveness: String,
    // Memory (M2)
    pub embedding_model: String,
    // Custom prompt (M3)
    pub custom_system_prompt: String,
    // Custom voices (M6): JSON array of {label, path} objects
    pub custom_voices: String,
    // Kokoro TTS (M6)
    pub kokoro_model: String,
    pub kokoro_voices: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".into(),
            model: "llama3.2:8b".into(),
            companion_name: "Amy".into(),
            personality: "gentle".into(),
            partner_mode: "company".into(),
            selected_project_id: db::DEFAULT_PROJECT_ID.into(),
            onboarding_done: "false".into(),
            piper_binary: String::new(),
            piper_voice: "en_US-amy-medium".into(),
            audio_input_device: "default".into(),
            whisper_binary: String::new(),
            whisper_model: String::new(),
            voice_threshold_db: "-30".into(),
            window_context_auto: "false".into(),
            window_context_allowed: "unset".into(),
            voice_speed: "1.0".into(),
            voice_expressiveness: "0.667".into(),
            embedding_model: "nomic-embed-text".into(),
            custom_system_prompt: String::new(),
            custom_voices: "[]".into(),
            kokoro_model: String::new(),
            kokoro_voices: String::new(),
        }
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let pairs: HashMap<String, String> = db::get_all_settings(&conn).into_iter().collect();
    let d = Settings::default();

    Ok(Settings {
        endpoint: pairs.get("endpoint").cloned().unwrap_or(d.endpoint),
        model: pairs.get("model").cloned().unwrap_or(d.model),
        companion_name: pairs
            .get("companion_name")
            .cloned()
            .unwrap_or(d.companion_name),
        personality: pairs.get("personality").cloned().unwrap_or(d.personality),
        partner_mode: pairs.get("partner_mode").cloned().unwrap_or(d.partner_mode),
        selected_project_id: db::selected_project_id(&conn),
        onboarding_done: pairs
            .get("onboarding_done")
            .cloned()
            .unwrap_or(d.onboarding_done),
        piper_binary: pairs.get("piper_binary").cloned().unwrap_or(d.piper_binary),
        piper_voice: pairs.get("piper_voice").cloned().unwrap_or(d.piper_voice),
        audio_input_device: pairs
            .get("audio_input_device")
            .cloned()
            .unwrap_or(d.audio_input_device),
        whisper_binary: pairs
            .get("whisper_binary")
            .cloned()
            .unwrap_or(d.whisper_binary),
        whisper_model: pairs
            .get("whisper_model")
            .cloned()
            .unwrap_or(d.whisper_model),
        voice_threshold_db: pairs
            .get("voice_threshold_db")
            .cloned()
            .unwrap_or(d.voice_threshold_db),
        window_context_auto: pairs
            .get("window_context_auto")
            .cloned()
            .unwrap_or(d.window_context_auto),
        voice_speed: pairs.get("voice_speed").cloned().unwrap_or(d.voice_speed),
        voice_expressiveness: pairs
            .get("voice_expressiveness")
            .cloned()
            .unwrap_or(d.voice_expressiveness),
        embedding_model: pairs
            .get("embedding_model")
            .cloned()
            .unwrap_or(d.embedding_model),
        custom_system_prompt: pairs
            .get("custom_system_prompt")
            .cloned()
            .unwrap_or(d.custom_system_prompt),
        window_context_allowed: pairs
            .get("window_context_allowed")
            .cloned()
            .unwrap_or(d.window_context_allowed),
        custom_voices: pairs
            .get("custom_voices")
            .cloned()
            .unwrap_or(d.custom_voices),
        kokoro_model: pairs.get("kokoro_model").cloned().unwrap_or(d.kokoro_model),
        kokoro_voices: pairs
            .get("kokoro_voices")
            .cloned()
            .unwrap_or(d.kokoro_voices),
    })
}

#[tauri::command]
pub fn update_setting(
    state: State<'_, AppState>,
    app: AppHandle,
    key: String,
    value: String,
) -> Result<(), String> {
    validate_public_setting_key(&key)?;
    if key == "selected_project_id" {
        return Err("Use select_project to change project scope".into());
    }
    // Keep context mutations in the same lock order as prompt assembly and
    // project switching. Revocation must clear manual notes as well as auto mode.
    if matches!(
        key.as_str(),
        "window_context_auto" | "window_context_allowed"
    ) {
        let mut context = state.context.lock().map_err(|e| e.to_string())?;
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        update_context_setting(&conn, &key, &value)?;
        if key == "window_context_allowed" && value != "true" {
            *context = crate::SharedContext::default();
            let _ = app.emit(
                "context:update",
                super::context::ContextStatus {
                    sharing: false,
                    window_title: None,
                    custom_note: None,
                },
            );
        }
        return Ok(());
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}

fn validate_public_setting_key(key: &str) -> Result<(), String> {
    if key.starts_with("_internal.") {
        return Err("Internal state cannot be changed through settings".into());
    }
    Ok(())
}

fn update_context_setting(
    conn: &rusqlite::Connection,
    key: &str,
    value: &str,
) -> Result<(), String> {
    match key {
        "window_context_auto" if !matches!(value, "true" | "false") => {
            return Err("Automatic context must be true or false".into());
        }
        "window_context_allowed" if !matches!(value, "unset" | "true" | "false") => {
            return Err("Context permission must be unset, true, or false".into());
        }
        _ => {}
    }
    let transaction = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    if key == "window_context_auto"
        && value == "true"
        && db::get_setting(&transaction, "window_context_allowed").as_deref() != Some("true")
    {
        return Err("Allow window context sharing before enabling automatic context".into());
    }
    db::set_setting(&transaction, key, value).map_err(|e| e.to_string())?;
    if key == "window_context_allowed" && value != "true" {
        db::set_setting(&transaction, "window_context_auto", "false").map_err(|e| e.to_string())?;
    }
    transaction.commit().map_err(|e| e.to_string())
}

pub(super) fn revoke_context_permission(conn: &rusqlite::Connection) -> Result<(), String> {
    update_context_setting(conn, "window_context_allowed", "false")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_revision_keys_cannot_be_changed_as_preferences() {
        assert!(validate_public_setting_key("_internal.memory_revision.project").is_err());
        assert!(validate_public_setting_key("_internal.any_future_state").is_err());
        assert!(validate_public_setting_key("companion_name").is_ok());
    }

    #[test]
    fn automatic_context_requires_permission_and_revocation_disables_it() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::migrations::run(&conn).unwrap();
        assert!(update_context_setting(&conn, "window_context_auto", "true").is_err());
        update_context_setting(&conn, "window_context_allowed", "true").unwrap();
        update_context_setting(&conn, "window_context_auto", "true").unwrap();
        revoke_context_permission(&conn).unwrap();
        assert_eq!(
            db::get_setting(&conn, "window_context_auto").as_deref(),
            Some("false")
        );
        assert_eq!(
            db::get_setting(&conn, "window_context_allowed").as_deref(),
            Some("false")
        );
        assert!(update_context_setting(&conn, "window_context_auto", "true").is_err());
    }

    #[test]
    fn invalid_context_values_cannot_replace_existing_permission() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::migrations::run(&conn).unwrap();
        update_context_setting(&conn, "window_context_allowed", "true").unwrap();
        update_context_setting(&conn, "window_context_auto", "true").unwrap();
        assert!(update_context_setting(&conn, "window_context_allowed", "yes").is_err());
        assert!(update_context_setting(&conn, "window_context_auto", "1").is_err());
        assert_eq!(
            db::get_setting(&conn, "window_context_auto").as_deref(),
            Some("true")
        );
        assert_eq!(
            db::get_setting(&conn, "window_context_allowed").as_deref(),
            Some("true")
        );
    }

    #[test]
    fn permission_revocation_rolls_back_if_auto_setting_cannot_be_disabled() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::migrations::run(&conn).unwrap();
        update_context_setting(&conn, "window_context_allowed", "true").unwrap();
        update_context_setting(&conn, "window_context_auto", "true").unwrap();
        conn.execute_batch(
            "CREATE TRIGGER block_auto_disable BEFORE INSERT ON settings
             WHEN NEW.key = 'window_context_auto' AND NEW.value = 'false'
             BEGIN SELECT RAISE(ABORT, 'test write failure'); END;",
        )
        .unwrap();
        assert!(revoke_context_permission(&conn).is_err());
        assert_eq!(
            db::get_setting(&conn, "window_context_allowed").as_deref(),
            Some("true")
        );
    }
}
