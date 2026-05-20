use crate::db;
use crate::llm::client::{self, OllamaModel};
use crate::AppState;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Serialize)]
pub struct OllamaStatus {
    pub connected: bool,
    pub models: Vec<OllamaStatusModel>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OllamaStatusModel {
    pub name: String,
    pub size: u64,
    pub modified_at: String,
}

impl From<OllamaModel> for OllamaStatusModel {
    fn from(m: OllamaModel) -> Self {
        Self {
            name: m.name,
            size: m.size.unwrap_or(0),
            modified_at: m.modified_at.unwrap_or_default(),
        }
    }
}

#[tauri::command]
pub async fn get_ollama_status(state: State<'_, AppState>) -> Result<OllamaStatus, String> {
    let endpoint = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::get_setting(&conn, "endpoint")
            .unwrap_or_else(|| "http://localhost:11434".into())
    };

    match client::list_models(&endpoint).await {
        Ok(models) => Ok(OllamaStatus {
            connected: true,
            models: models.into_iter().map(Into::into).collect(),
            error: None,
        }),
        Err(e) => Ok(OllamaStatus {
            connected: false,
            models: vec![],
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub async fn list_models(state: State<'_, AppState>) -> Result<Vec<OllamaStatusModel>, String> {
    let endpoint = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::get_setting(&conn, "endpoint")
            .unwrap_or_else(|| "http://localhost:11434".into())
    };
    client::list_models(&endpoint)
        .await
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn expand_to_main(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Show a save-file dialog and write `content` to the chosen path.
/// Returns silently if the user cancels.
#[tauri::command]
pub fn export_conversation(app: AppHandle, content: String, filename: String) -> Result<(), String> {
    let result = app
        .dialog()
        .file()
        .set_file_name(&filename)
        .blocking_save_file();

    match result {
        Some(tauri_plugin_dialog::FilePath::Path(p)) => {
            std::fs::write(&p, content).map_err(|e| e.to_string())
        }
        _ => Ok(()),
    }
}
