use crate::memory::{self, Memory};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn get_memories(state: State<'_, AppState>) -> Vec<Memory> {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    memory::list_memories(&conn)
}

#[tauri::command]
pub fn get_memory_count(state: State<'_, AppState>) -> i64 {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(_) => return 0,
    };
    memory::count_memories(&conn)
}

#[tauri::command]
pub fn delete_memory(
    state: State<'_, AppState>,
    id: String,
    project_id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    crate::db::require_selected_project(&conn, &project_id)?;
    memory::delete_memory(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn forget_all(state: State<'_, AppState>, project_id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    crate::db::require_selected_project(&conn, &project_id)?;
    memory::forget_all(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_retention_settings(
    state: State<'_, AppState>,
) -> Result<memory::retention::RetentionSettings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(memory::retention::settings(&conn))
}

/// Saves retention limits (days, 0 = keep forever). Purging is a separate,
/// explicit `purge_expired_data` call and also runs at startup.
#[tauri::command]
pub fn set_retention_settings(
    state: State<'_, AppState>,
    memory_days: u32,
    history_days: u32,
) -> Result<memory::retention::RetentionSettings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let settings = memory::retention::RetentionSettings {
        memory_days,
        history_days,
    };
    memory::retention::save_settings(&conn, &settings)?;
    Ok(memory::retention::settings(&conn))
}

#[tauri::command]
pub fn purge_expired_data(
    state: State<'_, AppState>,
) -> Result<memory::retention::PurgeReport, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    memory::retention::purge_expired(&conn, crate::domain::now_ms()).map_err(|e| e.to_string())
}
