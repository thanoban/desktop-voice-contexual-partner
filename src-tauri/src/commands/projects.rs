use crate::{db, AppState, SharedContext};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<db::projects::Project>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    db::projects::list(&conn).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    name: String,
    kind: String,
) -> Result<db::projects::Project, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    db::projects::create(&conn, &name, &kind)
}

#[tauri::command]
pub fn select_project(
    state: State<'_, AppState>,
    app: AppHandle,
    project_id: String,
) -> Result<db::projects::ProjectSelection, String> {
    state.conversation.while_idle(|| {
        let recording = state.recording.lock().map_err(|error| error.to_string())?;
        if recording.is_some() {
            return Err("Finish the recording before switching projects".into());
        }
        let mut context = state.context.lock().map_err(|error| error.to_string())?;
        let conn = state.db.lock().map_err(|error| error.to_string())?;
        let previous = db::selected_project_id(&conn);
        let selection = db::projects::select(&conn, &project_id)?;
        if previous != project_id {
            *context = SharedContext::default();
            let _ = app.emit("project:selected", &selection);
        }
        Ok(selection)
    })
}
