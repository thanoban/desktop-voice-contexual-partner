use crate::AppState;
use crate::SharedContext;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize, Clone)]
pub struct ContextStatus {
    pub sharing: bool,
    pub window_title: Option<String>,
    pub custom_note: Option<String>,
}

/// Returns the current foreground window title without enabling sharing.
#[tauri::command]
pub fn get_window_title() -> Option<String> {
    crate::context::get_active_window_title()
}

/// Enables window context sharing: snaps the current window title
/// and injects it into every subsequent LLM prompt.
#[tauri::command]
pub fn start_sharing_context(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<ContextStatus, String> {
    let title = crate::context::get_active_window_title();

    let status = {
        let mut ctx = state.context.lock().unwrap();
        ctx.sharing = true;
        ctx.window_title = title.clone();
        ContextStatus {
            sharing: true,
            window_title: ctx.window_title.clone(),
            custom_note: ctx.custom_note.clone(),
        }
    };

    let _ = app.emit("context:update", &status);
    Ok(status)
}

/// Disables all context sharing and clears stored context.
#[tauri::command]
pub fn stop_sharing_context(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut ctx = state.context.lock().unwrap();
        *ctx = SharedContext::default();
    }
    let _ = app.emit("context:update", ContextStatus {
        sharing: false,
        window_title: None,
        custom_note: None,
    });
    Ok(())
}

/// Adds a one-line note about what the user is doing right now.
/// This persists for the current sharing session.
#[tauri::command]
pub fn set_context_note(
    state: State<'_, AppState>,
    app: AppHandle,
    note: String,
) -> Result<ContextStatus, String> {
    let status = {
        let mut ctx = state.context.lock().unwrap();
        ctx.sharing = true;
        ctx.custom_note = if note.is_empty() { None } else { Some(note) };
        // Refresh window title when note is set
        ctx.window_title = crate::context::get_active_window_title();
        ContextStatus {
            sharing: ctx.sharing,
            window_title: ctx.window_title.clone(),
            custom_note: ctx.custom_note.clone(),
        }
    };
    let _ = app.emit("context:update", &status);
    Ok(status)
}

/// Called internally by send_message to build the context injection string.
pub fn build_context_injection(state: &AppState) -> Option<String> {
    let ctx = state.context.lock().ok()?;
    if !ctx.sharing {
        return None;
    }

    let mut parts = Vec::new();
    if let Some(ref title) = ctx.window_title {
        parts.push(format!("Active window: {}", title));
    }
    if let Some(ref note) = ctx.custom_note {
        parts.push(format!("User note: {}", note));
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!(
            "[CONTEXT — what the user is working on]\n{}\n[/CONTEXT]",
            parts.join("\n")
        ))
    }
}
