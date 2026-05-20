use crate::db;
use crate::llm::client::ChatMessage;
use crate::memory;
use crate::safety;
use crate::AppState;
use tauri::{AppHandle, Emitter, Manager, State};

fn format_age(created_at_ms: i64) -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let diff = (now_ms - created_at_ms) / 1000;
    if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86_400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86_400)
    }
}

fn personality_prompt(preset: &str, name: &str) -> String {
    let tone = match preset {
        "gentle" => "You are warm, patient, and gently encouraging. You notice how the user is \
                     feeling and respond with care. You're present and attentive.",
        "playful" => "You are witty, light-hearted, and fun. You bring levity to conversations \
                      without being flippant. You enjoy wordplay and gentle humour.",
        "calm" => "You are steady, measured, and quietly supportive. You speak with unhurried \
                   clarity. You are a grounding presence.",
        "energetic" => "You are enthusiastic and motivating. You celebrate small wins with \
                        genuine excitement and help the user feel capable.",
        _ => "You are a warm and thoughtful companion.",
    };

    format!(
        "You are {name}, a local AI companion running entirely on this user's machine. \
         {tone} \
         Keep responses conversational and concise — like a friend, not a lecture. \
         Do not use bullet points unless specifically asked. \
         Do not start responses with 'Certainly!' or 'Of course!'. \
         Speak naturally and warmly. \
         You have no access to the internet and no knowledge beyond your training data. \
         You are NOT a therapist. If the user seems distressed, acknowledge their feelings \
         and gently encourage real human support."
    )
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    app: AppHandle,
    content: String,
) -> Result<(), String> {
    // Check distress before anything else
    let distress = safety::check(&content);

    // DB work — all sync, no .await held
    let (session_id, model, endpoint, personality, name, piper_binary, piper_voice,
         voice_speed, voice_expressiveness, window_context_auto, embedding_model) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let sid = db::ensure_session(&conn).map_err(|e| e.to_string())?;
        db::save_turn(&conn, &sid, "user", &content).map_err(|e| e.to_string())?;
        let model    = db::get_setting(&conn, "model").unwrap_or_else(|| "llama3.2:8b".into());
        let endpoint = db::get_setting(&conn, "endpoint").unwrap_or_else(|| "http://localhost:11434".into());
        let persona  = db::get_setting(&conn, "personality").unwrap_or_else(|| "gentle".into());
        let name     = db::get_setting(&conn, "companion_name").unwrap_or_else(|| "Amy".into());
        let piper    = db::get_setting(&conn, "piper_binary").unwrap_or_default();
        let voice    = db::get_setting(&conn, "piper_voice").unwrap_or_else(|| "en_US-amy-medium".into());
        let speed    = db::get_setting(&conn, "voice_speed").unwrap_or_else(|| "1.0".into());
        let expr     = db::get_setting(&conn, "voice_expressiveness").unwrap_or_else(|| "0.667".into());
        let ctx_auto = db::get_setting(&conn, "window_context_auto").unwrap_or_else(|| "false".into());
        let emb_mdl  = db::get_setting(&conn, "embedding_model").unwrap_or_else(|| "nomic-embed-text".into());
        (sid, model, endpoint, persona, name, piper, voice, speed, expr, ctx_auto, emb_mdl)
    }; // MutexGuard dropped here

    // Build recent context
    let recent_turns = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::get_recent_turns(&conn, &session_id, 20)
    };

    // Memory recall: embed user message, find relevant past memories (best-effort)
    let query_emb: Option<Vec<f32>> = if !embedding_model.is_empty() {
        crate::embed::embed_text(&endpoint, &embedding_model, &content)
            .await
            .ok()
    } else {
        None
    };

    let memory_block: Option<String> = query_emb.as_ref().and_then(|emb| {
        let conn = state.db.lock().ok()?;
        let results = memory::search_memories(&conn, emb, 3);
        // Only inject memories with meaningful similarity (> 0.5)
        let relevant: Vec<_> = results.into_iter().filter(|r| r.score > 0.5).collect();
        if relevant.is_empty() {
            None
        } else {
            let lines: Vec<String> = relevant
                .iter()
                .map(|r| format!("• {}: {}", format_age(r.memory.created_at), r.memory.content))
                .collect();
            Some(format!(
                "[MEMORIES — relevant things from past conversations]\n{}\n[/MEMORIES]",
                lines.join("\n")
            ))
        }
    });

    // Build context injection: auto (always-on) or manual sharing
    let context_block = {
        let ctx = state.context.lock().map_err(|e| e.to_string())?;
        let title = if window_context_auto == "true" {
            // Live detection every message — no user action required
            crate::context::get_active_window_title()
        } else if ctx.sharing {
            ctx.window_title.clone()
        } else {
            None
        };
        let note = if ctx.sharing { ctx.custom_note.clone() } else { None };

        if title.is_some() || note.is_some() {
            let mut parts = Vec::new();
            if let Some(ref t) = title { parts.push(format!("Active window: {}", t)); }
            if let Some(ref n) = note  { parts.push(format!("User note: {}", n)); }
            Some(format!(
                "[CONTEXT — what the user is working on right now]\n{}\n[/CONTEXT]",
                parts.join("\n")
            ))
        } else {
            None
        }
    }; // MutexGuard dropped

    let mut system_prompt = personality_prompt(&personality, &name);
    if let Some(ref mem) = memory_block {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(mem);
    }
    if let Some(ref ctx) = context_block {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(ctx);
    }

    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "system".into(),
        content: system_prompt,
    }];
    for (role, text) in &recent_turns {
        messages.push(ChatMessage { role: role.clone(), content: text.clone() });
    }

    // Stream from Ollama
    let full_response = crate::llm::client::stream_chat(&app, &endpoint, &model, messages)
        .await
        .map_err(|e| {
            let _ = app.emit("chat:error", e.to_string());
            e.to_string()
        })?;

    // Append safety suffix if needed
    let final_response = if let Some(suffix) = safety::companion_suffix(distress) {
        format!("{}{}", full_response, suffix)
    } else {
        full_response.clone()
    };

    // Save assistant turn
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::save_turn(&conn, &session_id, "assistant", &final_response)
            .map_err(|e| e.to_string())?;
    }

    let _ = app.emit("chat:done", ());

    // Emit safety panel signal if distress detected
    if distress != safety::DistressLevel::None {
        let _ = app.emit("safety:show", ());
    }

    // Background: summarize + embed + store memory every 4 turns (4, 8, 12, ...)
    {
        let app_bg    = app.clone();
        let ep_bg     = endpoint.clone();
        let m_bg      = model.clone();
        let em_bg     = embedding_model.clone();
        let sid_bg    = session_id.clone();
        tokio::spawn(async move {
            if em_bg.is_empty() { return; }

            let (turn_count, recent_turns) = {
                let state = app_bg.state::<AppState>();
                let conn = match state.db.lock() { Ok(c) => c, Err(_) => return };
                let count = db::get_turn_count(&conn, &sid_bg).unwrap_or(0);
                let turns = db::get_recent_turns(&conn, &sid_bg, 8);
                (count, turns)
            };

            if turn_count < 4 || turn_count % 4 != 0 { return; }

            let summary = match crate::summarize::summarize_session(&ep_bg, &m_bg, &recent_turns).await {
                Ok(s) => s,
                Err(_) => return,
            };

            let embedding = match crate::embed::embed_text(&ep_bg, &em_bg, &summary).await {
                Ok(e) => e,
                Err(_) => return,
            };

            let state = app_bg.state::<AppState>();
            let conn = match state.db.lock() { Ok(c) => c, Err(_) => return };
            let _ = memory::store_memory(&conn, &sid_bg, &summary, &embedding, "session_summary");
        });
    }

    // Speak via Piper TTS (best-effort, errors are non-fatal)
    if !piper_binary.is_empty() {
        let speed  = voice_speed.parse::<f32>().unwrap_or(1.0);
        let expr   = voice_expressiveness.parse::<f32>().unwrap_or(0.667);
        let app_clone    = app.clone();
        let voice_clone  = piper_voice.clone();
        let binary_clone = piper_binary.clone();
        let text_clone   = final_response.clone();
        tokio::spawn(async move {
            let _ = crate::tts::piper::speak(&app_clone, &binary_clone, &voice_clone, &text_clone, speed, expr).await;
        });
    }

    Ok(())
}

#[tauri::command]
pub async fn get_greeting(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let (session_id, model, endpoint, personality, name, piper_binary, piper_voice,
         voice_speed, voice_expressiveness) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let sid = db::ensure_session(&conn).map_err(|e| e.to_string())?;
        let model    = db::get_setting(&conn, "model").unwrap_or_else(|| "llama3.2:8b".into());
        let endpoint = db::get_setting(&conn, "endpoint").unwrap_or_else(|| "http://localhost:11434".into());
        let persona  = db::get_setting(&conn, "personality").unwrap_or_else(|| "gentle".into());
        let name     = db::get_setting(&conn, "companion_name").unwrap_or_else(|| "Amy".into());
        let piper    = db::get_setting(&conn, "piper_binary").unwrap_or_default();
        let voice    = db::get_setting(&conn, "piper_voice").unwrap_or_else(|| "en_US-amy-medium".into());
        let speed    = db::get_setting(&conn, "voice_speed").unwrap_or_else(|| "1.0".into());
        let expr     = db::get_setting(&conn, "voice_expressiveness").unwrap_or_else(|| "0.667".into());
        (sid, model, endpoint, persona, name, piper, voice, speed, expr)
    };

    let turns_today = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::get_recent_turns(&conn, &session_id, 1)
    };

    let greeting_instruction = if turns_today.is_empty() {
        "Send a brief, warm greeting to the user — just one or two sentences. \
         Be natural, not overly cheerful. Don't ask multiple questions at once."
    } else {
        "Welcome the user back with one warm sentence. Keep it short."
    };

    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: personality_prompt(&personality, &name),
        },
        ChatMessage {
            role: "user".into(),
            content: greeting_instruction.into(),
        },
    ];

    let full_response = crate::llm::client::stream_chat(&app, &endpoint, &model, messages)
        .await
        .map_err(|e| {
            let _ = app.emit("chat:error", e.to_string());
            e.to_string()
        })?;

    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::save_turn(&conn, &session_id, "assistant", &full_response)
            .map_err(|e| e.to_string())?;
    }

    let _ = app.emit("chat:done", ());

    if !piper_binary.is_empty() {
        let speed = voice_speed.parse::<f32>().unwrap_or(1.0);
        let expr  = voice_expressiveness.parse::<f32>().unwrap_or(0.667);
        let app_clone = app.clone();
        tokio::spawn(async move {
            let _ = crate::tts::piper::speak(&app_clone, &piper_binary, &piper_voice, &full_response, speed, expr).await;
        });
    }

    Ok(())
}

#[tauri::command]
pub fn start_new_session(state: State<'_, AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::ensure_session(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_speaking() -> Result<(), String> {
    // In M0 the subprocess-based TTS doesn't support mid-stream interrupt.
    // CPAL-based interrupt arrives in M1.
    Ok(())
}

#[tauri::command]
pub async fn speak_text(
    state: State<'_, AppState>,
    app: AppHandle,
    text: String,
    voice: String,
) -> Result<(), String> {
    let (piper_binary, speed, expr) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let binary = db::get_setting(&conn, "piper_binary").unwrap_or_default();
        let s = db::get_setting(&conn, "voice_speed").unwrap_or_else(|| "1.0".into());
        let e = db::get_setting(&conn, "voice_expressiveness").unwrap_or_else(|| "0.667".into());
        (binary, s.parse::<f32>().unwrap_or(1.0), e.parse::<f32>().unwrap_or(0.667))
    };
    crate::tts::piper::speak(&app, &piper_binary, &voice, &text, speed, expr)
        .await
        .map_err(|e| e.to_string())
}
