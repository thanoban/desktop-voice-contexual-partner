use crate::db;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    // LLM
    pub endpoint: String,
    pub model: String,
    // Companion
    pub companion_name: String,
    pub personality: String,
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
    // Voice tone (M1.1)
    pub voice_speed: String,
    pub voice_expressiveness: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            endpoint:            "http://localhost:11434".into(),
            model:               "llama3.2:8b".into(),
            companion_name:      "Amy".into(),
            personality:         "gentle".into(),
            onboarding_done:     "false".into(),
            piper_binary:        String::new(),
            piper_voice:         "en_US-amy-medium".into(),
            audio_input_device:  "default".into(),
            whisper_binary:      String::new(),
            whisper_model:       String::new(),
            voice_threshold_db:  "-30".into(),
            window_context_auto: "false".into(),
            voice_speed:          "1.0".into(),
            voice_expressiveness: "0.667".into(),
        }
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let pairs: HashMap<String, String> = db::get_all_settings(&conn).into_iter().collect();
    let d = Settings::default();

    Ok(Settings {
        endpoint:            pairs.get("endpoint").cloned().unwrap_or(d.endpoint),
        model:               pairs.get("model").cloned().unwrap_or(d.model),
        companion_name:      pairs.get("companion_name").cloned().unwrap_or(d.companion_name),
        personality:         pairs.get("personality").cloned().unwrap_or(d.personality),
        onboarding_done:     pairs.get("onboarding_done").cloned().unwrap_or(d.onboarding_done),
        piper_binary:        pairs.get("piper_binary").cloned().unwrap_or(d.piper_binary),
        piper_voice:         pairs.get("piper_voice").cloned().unwrap_or(d.piper_voice),
        audio_input_device:  pairs.get("audio_input_device").cloned().unwrap_or(d.audio_input_device),
        whisper_binary:      pairs.get("whisper_binary").cloned().unwrap_or(d.whisper_binary),
        whisper_model:       pairs.get("whisper_model").cloned().unwrap_or(d.whisper_model),
        voice_threshold_db:  pairs.get("voice_threshold_db").cloned().unwrap_or(d.voice_threshold_db),
        window_context_auto:  pairs.get("window_context_auto").cloned().unwrap_or(d.window_context_auto),
        voice_speed:          pairs.get("voice_speed").cloned().unwrap_or(d.voice_speed),
        voice_expressiveness: pairs.get("voice_expressiveness").cloned().unwrap_or(d.voice_expressiveness),
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
