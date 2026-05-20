use crate::memory::{self, DocumentInfo};
use crate::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize, Clone)]
pub struct IngestProgress {
    pub source: String,
    pub current: usize,
    pub total: usize,
}

#[derive(Serialize, Clone)]
pub struct IngestResult {
    pub source: String,
    pub chunks: usize,
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
    app: AppHandle,
    path: String,
) -> Result<String, String> {
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

        // Extract text (CPU-bound — spawn_blocking so we don't stall Tokio)
        let text = match tokio::task::spawn_blocking(move || crate::rag::extract_text(&path_buf))
            .await
        {
            Ok(Ok(t)) => t,
            Ok(Err(e)) => {
                let _ = app_bg.emit("rag:error", e.to_string());
                return;
            }
            Err(_) => {
                let _ = app_bg.emit("rag:error", "Text extraction task panicked");
                return;
            }
        };

        let chunks = crate::rag::chunk_text(&text, 450, 80);
        let total = chunks.len();

        if total == 0 {
            let _ = app_bg.emit("rag:error", format!("{}: no text found", src));
            return;
        }

        let _ = app_bg.emit("rag:progress", IngestProgress { source: src.clone(), current: 0, total });

        // Read embedding settings (drop lock before any await)
        let (endpoint, embedding_model) = {
            let conn = match state.db.lock() {
                Ok(c) => c,
                Err(_) => return,
            };
            let ep = crate::db::get_setting(&conn, "endpoint")
                .unwrap_or_else(|| "http://localhost:11434".into());
            let em = crate::db::get_setting(&conn, "embedding_model")
                .unwrap_or_else(|| "nomic-embed-text".into());
            (ep, em)
        };

        let mut stored = 0usize;
        for (i, chunk) in chunks.iter().enumerate() {
            let emb = match crate::embed::embed_text(&endpoint, &embedding_model, chunk).await {
                Ok(e) => e,
                Err(e) => {
                    let _ = app_bg.emit(
                        "rag:error",
                        format!(
                            "Embedding failed — is '{}' available? Run: ollama pull {}. Error: {}",
                            embedding_model, embedding_model, e
                        ),
                    );
                    return;
                }
            };

            {
                let conn = match state.db.lock() {
                    Ok(c) => c,
                    Err(_) => return,
                };
                if memory::store_document_chunk(&conn, chunk, &emb, &src).is_ok() {
                    stored += 1;
                }
            }

            let _ = app_bg.emit(
                "rag:progress",
                IngestProgress { source: src.clone(), current: i + 1, total },
            );
        }

        let _ = app_bg.emit("rag:done", IngestResult { source: src, chunks: stored });
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
pub fn delete_document(state: State<'_, AppState>, source_file: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    memory::delete_document(&conn, &source_file).map_err(|e| e.to_string())
}
