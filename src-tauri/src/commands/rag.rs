use crate::memory::{self, DocumentInfo};
use crate::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize, Clone)]
pub struct IngestProgress {
    pub project_id: String,
    pub source: String,
    pub current: usize,
    pub total: usize,
}

#[derive(Serialize, Clone)]
pub struct IngestResult {
    pub project_id: String,
    pub source: String,
    pub chunks: usize,
}

#[derive(Serialize, Clone)]
pub struct IngestError {
    pub project_id: String,
    pub message: String,
}

/// Opens a file picker and returns the selected path (or null if cancelled).
#[tauri::command]
pub fn pick_document(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .add_filter("Documents", &["txt", "md", "markdown", "pdf", "docx"])
        .blocking_pick_file()
        .and_then(|fp| match fp {
            tauri_plugin_dialog::FilePath::Path(p) => Some(p.to_string_lossy().to_string()),
            _ => None,
        })
}

/// Ingests a document: extracts text → chunks → embeds → stores.
/// Returns immediately; progress is emitted via `rag:progress` events.
/// Emits `rag:done` on success, `rag:error` on failure.
#[tauri::command]
pub async fn ingest_document(
    state: State<'_, AppState>,
    app: AppHandle,
    path: String,
    project_id: String,
) -> Result<String, String> {
    let (project_id, endpoint, embedding_model) = {
        let conn = state.db.lock().map_err(|error| error.to_string())?;
        crate::db::require_selected_project(&conn, &project_id)?;
        (
            crate::db::selected_project_id(&conn),
            crate::db::get_setting(&conn, "endpoint")
                .unwrap_or_else(|| "http://localhost:11434".into()),
            crate::db::get_setting(&conn, "embedding_model")
                .unwrap_or_else(|| "nomic-embed-text".into()),
        )
    };
    let path_buf = PathBuf::from(&path);
    let source_file = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document")
        .to_string();

    let src = source_file.clone();
    let app_bg = app.clone();

    tokio::spawn(async move {
        let state = app_bg.state::<AppState>();
        let fail = |message: String| {
            let _ = app_bg.emit(
                "rag:error",
                IngestError {
                    project_id: project_id.clone(),
                    message,
                },
            );
        };

        // Extract text (CPU-bound — spawn_blocking so we don't stall Tokio)
        let text =
            match tokio::task::spawn_blocking(move || crate::rag::extract_text(&path_buf)).await {
                Ok(Ok(t)) => t,
                Ok(Err(e)) => {
                    fail(e.to_string());
                    return;
                }
                Err(_) => {
                    fail("Text extraction task panicked".into());
                    return;
                }
            };

        let chunks = crate::rag::chunk_text(&text, 450, 80);
        let total = chunks.len();

        if total == 0 {
            fail(format!("{}: no text found", src));
            return;
        }

        let _ = app_bg.emit(
            "rag:progress",
            IngestProgress {
                project_id: project_id.clone(),
                source: src.clone(),
                current: 0,
                total,
            },
        );

        let mut stored = 0usize;
        for (i, chunk) in chunks.iter().enumerate() {
            let emb = match crate::embed::embed_text(&endpoint, &embedding_model, chunk).await {
                Ok(e) => e,
                Err(e) => {
                    fail(format!(
                        "Embedding failed — is '{}' available? Run: ollama pull {}. Error: {}",
                        embedding_model, embedding_model, e
                    ));
                    return;
                }
            };

            {
                let conn = match state.db.lock() {
                    Ok(c) => c,
                    Err(_) => {
                        fail("Document storage lock failed".into());
                        return;
                    }
                };
                if let Err(error) =
                    memory::store_document_chunk(&conn, chunk, &emb, &src, &project_id)
                {
                    fail(error.to_string());
                    return;
                }
                stored += 1;
            }

            let _ = app_bg.emit(
                "rag:progress",
                IngestProgress {
                    project_id: project_id.clone(),
                    source: src.clone(),
                    current: i + 1,
                    total,
                },
            );
        }

        let _ = app_bg.emit(
            "rag:done",
            IngestResult {
                project_id,
                source: src,
                chunks: stored,
            },
        );
    });

    Ok(source_file)
}

#[tauri::command]
pub fn list_documents(state: State<'_, AppState>) -> Vec<DocumentInfo> {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    memory::list_documents(&conn)
}

#[tauri::command]
pub fn delete_document(
    state: State<'_, AppState>,
    source_file: String,
    project_id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    crate::db::require_selected_project(&conn, &project_id)?;
    memory::delete_document(&conn, &source_file).map_err(|e| e.to_string())
}
