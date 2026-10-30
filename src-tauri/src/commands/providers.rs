//! Provider configuration commands. API keys are accepted but never returned;
//! views report only whether a key exists and which host it is bound to.

use crate::llm::credentials::SystemCredentialStore;
use crate::llm::router::{self, ProviderSettingsView};
use crate::AppState;
use tauri::State;

fn view(conn: &rusqlite::Connection) -> ProviderSettingsView {
    router::settings_view(conn, &SystemCredentialStore)
}

#[tauri::command]
pub fn get_provider_settings(state: State<'_, AppState>) -> Result<ProviderSettingsView, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(view(&conn))
}

#[tauri::command]
pub fn set_routing_profile(
    state: State<'_, AppState>,
    profile: String,
) -> Result<ProviderSettingsView, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    router::set_routing_profile(&conn, &profile)?;
    Ok(view(&conn))
}

#[tauri::command]
pub fn set_active_provider(
    state: State<'_, AppState>,
    provider: String,
) -> Result<ProviderSettingsView, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    router::set_active_provider(&conn, &provider)?;
    Ok(view(&conn))
}

#[tauri::command]
pub fn configure_provider(
    state: State<'_, AppState>,
    provider: String,
    base_url: String,
    model: String,
) -> Result<ProviderSettingsView, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    router::configure_provider(&conn, &provider, &base_url, &model)?;
    Ok(view(&conn))
}

#[tauri::command]
pub fn set_provider_key(
    state: State<'_, AppState>,
    provider: String,
    key: String,
) -> Result<ProviderSettingsView, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    router::set_provider_key(&conn, &SystemCredentialStore, &provider, &key)?;
    Ok(view(&conn))
}

#[tauri::command]
pub fn clear_provider_key(
    state: State<'_, AppState>,
    provider: String,
) -> Result<ProviderSettingsView, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    router::clear_provider_key(&conn, &SystemCredentialStore, &provider)?;
    Ok(view(&conn))
}

#[tauri::command]
pub fn has_provider_key(state: State<'_, AppState>, provider: String) -> Result<bool, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    router::has_provider_key(&conn, &SystemCredentialStore, &provider)
}
