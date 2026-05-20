use crate::db;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub endpoint: String,
    pub model: String,
    pub companion_name: String,
    pub personality: String,
    pub piper_binary: String,
    pub piper_voice: String,
    pub onboarding_done: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".into(),
            model: "llama3.2:8b".into(),
            companion_name: "Amy".into(),
            personality: "gentle".into(),
            piper_binary: String::new(),
            piper_voice: "en_US-amy-medium".into(),
            onboarding_done: "false".into(),
        }
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let pairs: HashMap<String, String> = db::get_all_settings(&conn).into_iter().collect();
    let defaults = Settings::default();

    Ok(Settings {
        endpoint:        pairs.get("endpoint").cloned().unwrap_or(defaults.endpoint),
        model:           pairs.get("model").cloned().unwrap_or(defaults.model),
        companion_name:  pairs.get("companion_name").cloned().unwrap_or(defaults.companion_name),
        personality:     pairs.get("personality").cloned().unwrap_or(defaults.personality),
        piper_binary:    pairs.get("piper_binary").cloned().unwrap_or(defaults.piper_binary),
        piper_voice:     pairs.get("piper_voice").cloned().unwrap_or(defaults.piper_voice),
        onboarding_done: pairs.get("onboarding_done").cloned().unwrap_or(defaults.onboarding_done),
    })
}

#[tauri::command]
pub fn update_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}
