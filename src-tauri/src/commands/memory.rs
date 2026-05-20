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
pub fn delete_memory(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    memory::delete_memory(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn forget_all(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    memory::forget_all(&conn).map_err(|e| e.to_string())
}
