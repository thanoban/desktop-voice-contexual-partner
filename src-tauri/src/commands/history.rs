use crate::db;
use crate::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub turn_count: i64,
    pub preview: String,
}

#[derive(Debug, Serialize)]
pub struct TurnInfo {
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[tauri::command]
pub fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionInfo>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let sessions = db::list_sessions(&conn);
    Ok(sessions
        .into_iter()
        .map(|s| SessionInfo {
            id: s.id,
            started_at: s.started_at,
            ended_at: s.ended_at,
            turn_count: s.turn_count,
            preview: s
                .first_user_message
                .unwrap_or_default()
                .chars()
                .take(80)
                .collect(),
        })
        .collect())
}

#[tauri::command]
pub fn get_session_turns(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<TurnInfo>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let turns = db::get_all_turns(&conn, &session_id);
    Ok(turns
        .into_iter()
        .map(|(role, content, created_at)| TurnInfo {
            role,
            content,
            created_at,
        })
        .collect())
}
