use crate::{db, AppState};
use futures_util::StreamExt;
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

// ── Download URLs ─────────────────────────────────────────────────────────────

const PIPER_WIN_URL: &str =
    "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip";
const PIPER_VOICE_ONNX_URL: &str =
    "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/amy/medium/en_US-amy-medium.onnx";
const PIPER_VOICE_JSON_URL: &str =
    "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/amy/medium/en_US-amy-medium.onnx.json";
const WHISPER_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";

// ── Event payloads ────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct DownloadProgress {
    pub tool: String,
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Serialize, Clone)]
pub struct DownloadDone {
    pub tool: String,
    pub path: String,
}

// ── Setup status ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SetupStatus {
    pub piper_ok: bool,
    pub piper_voice_ok: bool,
    pub whisper_model_ok: bool,
    pub piper_path: String,
    pub whisper_model_path: String,
}

fn tools_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("tools")
}

/// Returns status of installed tools.
#[tauri::command]
pub fn check_setup(app: AppHandle, state: State<'_, AppState>) -> SetupStatus {
    let t = tools_dir(&app);
    let auto_piper = t.join("piper").join("piper.exe");
    let auto_voice = t.join("piper").join("voices").join("en_US-amy-medium.onnx");
    let auto_model = t.join("whisper").join("ggml-base.en.bin");

    let (cfg_piper, cfg_voice, cfg_model) = {
        let conn = state.db.lock().unwrap();
        (
            db::get_setting(&conn, "piper_binary").unwrap_or_default(),
            db::get_setting(&conn, "piper_voice").unwrap_or_default(),
            db::get_setting(&conn, "whisper_model").unwrap_or_default(),
        )
    };

    let piper_ok = auto_piper.exists()
        || (!cfg_piper.is_empty() && PathBuf::from(&cfg_piper).exists());
    let voice_ok = auto_voice.exists()
        || (!cfg_voice.is_empty() && PathBuf::from(&cfg_voice).exists());
    let model_ok = auto_model.exists()
        || (!cfg_model.is_empty() && PathBuf::from(&cfg_model).exists());

    SetupStatus {
        piper_ok,
        piper_voice_ok: voice_ok,
        whisper_model_ok: model_ok,
        piper_path: if auto_piper.exists() {
            auto_piper.to_string_lossy().to_string()
        } else {
            cfg_piper
        },
        whisper_model_path: if auto_model.exists() {
            auto_model.to_string_lossy().to_string()
        } else {
            cfg_model
        },
    }
}

// ── File picker ───────────────────────────────────────────────────────────────

/// Open a native file picker. `filters` is a list of extensions e.g. ["exe", "bin"].
#[tauri::command]
pub fn pick_file(app: AppHandle, filters: Vec<String>) -> Option<String> {
    let refs: Vec<&str> = filters.iter().map(|s| s.as_str()).collect();
    let mut picker = app.dialog().file();
    if !refs.is_empty() {
        picker = picker.add_filter("Files", &refs);
    }
    picker
        .blocking_pick_file()
        .and_then(|fp| match fp {
            tauri_plugin_dialog::FilePath::Path(p) => Some(p.to_string_lossy().to_string()),
            _ => None,
        })
}

// ── Download ──────────────────────────────────────────────────────────────────

/// Download a named tool with progress events.
/// tool: "piper_windows" | "piper_voice_amy" | "whisper_model_base_en"
/// Emits: download:progress { tool, downloaded, total }
/// Emits: download:done    { tool, path }
/// Emits: download:error   { tool, error }
#[tauri::command]
pub async fn download_tool(
    app: AppHandle,
    state: State<'_, AppState>,
    tool: String,
) -> Result<String, String> {
    let result = match tool.as_str() {
        "piper_windows"         => dl_piper_windows(&app, &state).await,
        "piper_voice_amy"       => dl_piper_voice_amy(&app, &state).await,
        "whisper_model_base_en" => dl_whisper_model(&app, &state).await,
        other => Err(format!("Unknown tool: {other}")),
    };
    if let Err(ref e) = result {
        let _ = app.emit(
            "download:error",
            serde_json::json!({ "tool": tool, "error": e }),
        );
    }
    result
}

// ── Shared download helper ────────────────────────────────────────────────────

async fn fetch_with_progress(
    app: &AppHandle,
    tool: &str,
    url: &str,
    dest: &PathBuf,
) -> Result<(), String> {
    let resp = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} for {url}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        downloaded += bytes.len() as u64;
        let _ = app.emit(
            "download:progress",
            DownloadProgress { tool: tool.to_string(), downloaded, total },
        );
    }

    Ok(())
}

// ── Per-tool downloaders ──────────────────────────────────────────────────────

async fn dl_piper_windows(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<String, String> {
    let zip_path = tools_dir(app).join("piper_win.zip");
    fetch_with_progress(app, "piper_windows", PIPER_WIN_URL, &zip_path).await?;

    let piper_dir = tools_dir(app).join("piper");
    std::fs::create_dir_all(&piper_dir).map_err(|e| e.to_string())?;

    // Extract .exe and .dll files from the zip
    let data = std::fs::read(&zip_path).map_err(|e| e.to_string())?;
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let ext = name.rsplit('.').next().unwrap_or("");
        if matches!(ext, "exe" | "dll") {
            let fname = PathBuf::from(&name)
                .file_name()
                .map(|n| n.to_owned())
                .unwrap_or_default();
            let out = piper_dir.join(&fname);
            let mut f = std::fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut f).map_err(|e| e.to_string())?;
        }
    }

    let _ = std::fs::remove_file(&zip_path);

    let exe_path = piper_dir.join("piper.exe").to_string_lossy().to_string();
    {
        let conn = state.db.lock().unwrap();
        let _ = db::set_setting(&conn, "piper_binary", &exe_path);
    }

    let _ = app.emit("download:done", DownloadDone {
        tool: "piper_windows".into(),
        path: exe_path.clone(),
    });
    Ok(exe_path)
}

async fn dl_piper_voice_amy(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<String, String> {
    let voices_dir = tools_dir(app).join("piper").join("voices");
    let onnx = voices_dir.join("en_US-amy-medium.onnx");
    let json = voices_dir.join("en_US-amy-medium.onnx.json");

    // Download .onnx model (large, with progress)
    fetch_with_progress(app, "piper_voice_amy", PIPER_VOICE_ONNX_URL, &onnx).await?;

    // Download .onnx.json config (small, no separate progress)
    let resp = reqwest::get(PIPER_VOICE_JSON_URL)
        .await
        .map_err(|e| e.to_string())?;
    std::fs::write(&json, resp.bytes().await.map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    let path_str = onnx.to_string_lossy().to_string();
    {
        // Store full path so resolve_voice() finds it immediately
        let conn = state.db.lock().unwrap();
        let _ = db::set_setting(&conn, "piper_voice", &path_str);
    }

    let _ = app.emit("download:done", DownloadDone {
        tool: "piper_voice_amy".into(),
        path: path_str.clone(),
    });
    Ok(path_str)
}

async fn dl_whisper_model(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<String, String> {
    let dest = tools_dir(app).join("whisper").join("ggml-base.en.bin");
    fetch_with_progress(app, "whisper_model_base_en", WHISPER_MODEL_URL, &dest).await?;

    let path_str = dest.to_string_lossy().to_string();
    {
        let conn = state.db.lock().unwrap();
        let _ = db::set_setting(&conn, "whisper_model", &path_str);
    }

    let _ = app.emit("download:done", DownloadDone {
        tool: "whisper_model_base_en".into(),
        path: path_str.clone(),
    });
    Ok(path_str)
}
