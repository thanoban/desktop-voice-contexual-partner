# VoicePartner — Architecture Guide

> A complete, beginner-friendly walkthrough of every file, module, and data flow in the codebase.  
> If you're new to Tauri, Rust, or React — start here.

---

## Table of Contents

1. [What VoicePartner Does](#1-what-voicepartner-does)
2. [Technology Stack Explained](#2-technology-stack-explained)
3. [Project Folder Structure](#3-project-folder-structure)
4. [How Tauri Works — The Bridge](#4-how-tauri-works--the-bridge)
5. [Backend — Rust (`src-tauri/`)](#5-backend--rust-src-tauri)
   - [App Entry Point](#51-app-entry-point--librs)
   - [Database Layer](#52-database-layer--db)
   - [Commands Layer](#53-commands-layer--commands)
   - [LLM Client](#54-llm-client--llm)
   - [Text-to-Speech](#55-text-to-speech--tts)
   - [Audio Capture & STT](#56-audio-capture--stt--audio)
   - [Memory System](#57-memory-system--memory--embed--summarize)
   - [RAG — Document Q&A](#58-rag--document-qa--rag)
   - [Safety System](#59-safety-system--safety)
   - [Context Sharing](#510-context-sharing--context)
6. [Frontend — React (`src/`)](#6-frontend--react-src)
   - [Stores (State)](#61-stores--state)
   - [Components](#62-components)
   - [Tauri Bridge (`lib/tauri.ts`)](#63-tauri-bridge--libtaurits)
7. [Database Schema](#7-database-schema)
8. [Key Data Flows](#8-key-data-flows)
   - [Sending a Chat Message](#81-sending-a-chat-message)
   - [Recording a Voice Message](#82-recording-a-voice-message)
   - [Memory Recall](#83-memory-recall)
   - [TTS Routing](#84-tts-routing)
9. [Settings Reference](#9-settings-reference)
10. [Key Design Decisions](#10-key-design-decisions)

---

## 1. What VoicePartner Does

VoicePartner is a **desktop application** that acts as a local AI companion. Here is the full journey of a single voice message:

```
You press Space
  → Microphone records your voice
    → whisper.cpp converts speech to text
      → Rust builds a context (your memories, active window, recent chat)
        → Ollama (local LLM) streams a reply
          → TTS engine speaks the reply aloud
            → Memory is saved for future sessions
```

**Nothing leaves your computer.** Every AI model runs locally. The app talks to Ollama (an open-source LLM runner) on `localhost:11434`, and uses local binaries for speech recognition and text-to-speech.

---

## 2. Technology Stack Explained

| Layer | Technology | What it does |
|---|---|---|
| **Desktop framework** | **Tauri v2** | Wraps a Rust backend + web frontend into a native desktop app |
| **Backend language** | **Rust** | Fast, safe systems language — runs all AI/audio/DB logic |
| **Frontend language** | **TypeScript + React** | The UI you see — buttons, chat, settings |
| **UI state** | **Zustand** | Lightweight React state manager (like a mini Redux) |
| **UI styling** | **Tailwind CSS v4** | Utility-class CSS — no separate `.css` files needed |
| **Local AI** | **Ollama** | Runs LLMs (llama3.2, mistral, etc.) locally via HTTP |
| **Speech-to-text** | **whisper.cpp** | C++ binary — converts your voice to text |
| **Text-to-speech** | **Piper / SAPI / Kokoro** | Three different TTS engines (see §5.5) |
| **Database** | **SQLite via rusqlite** | Stores all chats, memories, settings — single file |
| **Embeddings** | **Ollama (nomic-embed-text)** | Converts text to vectors for memory search |
| **Build tool** | **Vite** | Bundles the React frontend |

### What is Tauri?

Tauri is like Electron but smaller and faster. Instead of bundling a full browser, it uses your OS's built-in web renderer (WebView2 on Windows). Your React app runs inside that WebView, and it talks to the Rust backend through a typed IPC bridge. The Rust code has access to native OS APIs (file system, audio, system tray, global shortcuts), while React handles the visual interface.

---

## 3. Project Folder Structure

```
VoicePartner/
│
├── src/                        ← Frontend (React + TypeScript)
│   ├── App.tsx                 Main app shell, event wiring
│   ├── Widget.tsx              Mini overlay widget (400×58px)
│   ├── main.tsx                React entry point
│   ├── components/             All UI components (see §6.2)
│   ├── store/                  Zustand state stores (see §6.1)
│   └── lib/
│       └── tauri.ts            All calls to Rust backend (typed)
│
├── src-tauri/                  ← Backend (Rust)
│   ├── Cargo.toml              Rust dependencies
│   ├── tauri.conf.json         Window config, app name, version
│   ├── icons/                  App icon files
│   └── src/
│       ├── main.rs             Rust binary entry (calls lib.rs::run)
│       ├── lib.rs              App setup: DB, tray, shortcuts, commands
│       │
│       ├── commands/           Tauri commands (callable from frontend)
│       │   ├── mod.rs          Exports all command modules
│       │   ├── chat.rs         send_message, get_greeting, speak_text
│       │   ├── voice.rs        start_listening, stop_listening
│       │   ├── context.rs      Window context sharing
│       │   ├── memory.rs       get/delete memories
│       │   ├── rag.rs          Document ingestion & search
│       │   ├── settings.rs     get_settings, update_setting
│       │   ├── setup.rs        Download Piper/Whisper/Kokoro
│       │   └── system.rs       Ollama status, list models, export chat
│       │
│       ├── db/
│       │   ├── mod.rs          SQLite helpers (get/set settings, sessions, turns)
│       │   └── migrations.rs   Schema versioned migrations (v1→v6)
│       │
│       ├── llm/
│       │   ├── mod.rs          Module export
│       │   └── client.rs       Ollama HTTP streaming client
│       │
│       ├── tts/
│       │   ├── mod.rs          Shared helpers (play_wav, temp_wav_path)
│       │   ├── piper.rs        Piper subprocess + SAPI routing + voice router
│       │   └── kokoro.rs       Kokoro neural TTS via Python subprocess
│       │
│       ├── audio/
│       │   ├── mod.rs          Module export
│       │   ├── capture.rs      CPAL microphone recording (16kHz mono WAV)
│       │   └── stt.rs          whisper.cpp subprocess STT
│       │
│       ├── memory/
│       │   └── mod.rs          Store & search memories (cosine similarity)
│       │
│       ├── embed/
│       │   └── mod.rs          Ollama embedding client
│       │
│       ├── summarize/
│       │   └── mod.rs          LLM-based session summarization
│       │
│       ├── rag/
│       │   └── mod.rs          Document chunking + vector search
│       │
│       ├── safety/
│       │   └── mod.rs          Distress keyword detection
│       │
│       └── context/
│           └── mod.rs          Active window title detection
│
├── package.json                Frontend dependencies + npm scripts
├── README.md                   User-facing setup guide
├── ARCHITECTURE.md             This file
└── ROADMAP.md                  Future development plans
```

---

## 4. How Tauri Works — The Bridge

Tauri connects Rust and React using two mechanisms:

### 4.1 Commands (Frontend → Backend)

React calls Rust functions using `invoke()`:

```typescript
// Frontend (TypeScript)
import { invoke } from "@tauri-apps/api/core";
const result = await invoke("send_message", { content: "Hello!" });
```

```rust
// Backend (Rust) — registered in lib.rs invoke_handler
#[tauri::command]
pub async fn send_message(state: State<'_, AppState>, app: AppHandle, content: String)
    -> Result<(), String>
```

Every function marked `#[tauri::command]` must be registered in `lib.rs` inside `tauri::generate_handler![...]`.

### 4.2 Events (Backend → Frontend)

For things that happen over time (like streaming LLM tokens), Rust emits events:

```rust
// Backend — emit a token as it arrives
let _ = app.emit("chat:token", token);
```

```typescript
// Frontend — listen for tokens
import { listen } from "@tauri-apps/api/event";
const unlisten = await listen<string>("chat:token", (event) => {
  appendToken(event.payload);
});
```

### 4.3 App State

`AppState` (defined in `lib.rs`) is shared across all commands using Tauri's state management:

```rust
pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,      // SQLite connection
    pub recording: Mutex<Option<ActiveRecording>>, // current mic recording
    pub context: Mutex<SharedContext>,         // active window title info
}
```

`Mutex` wraps each field because multiple Tauri commands can run concurrently (async), and Rust requires exclusive access to mutable data. The pattern is: lock the mutex, do the work, drop the guard (so others can use it).

---

## 5. Backend — Rust (`src-tauri/`)

### 5.1 App Entry Point — `lib.rs`

This is the heart of the Rust side. When the app starts:

1. **Database opens** — SQLite file at `%APPDATA%\com.voicepartner.app\voicepartner.db`
2. **Migrations run** — ensure the schema is up-to-date
3. **Widget window created** — 400×58px transparent always-on-top window
4. **System tray created** — icon + menu (Show Widget, Open VoicePartner, Quit)
5. **Close → Hide wired** — closing the main window hides it instead of quitting
6. **Global shortcut registered** — `Alt+Space` shows the main window and emits `shortcut:ptt:toggle`
7. **All commands registered** — the `invoke_handler` lists every callable function

```rust
// Shortcut registration (from lib.rs)
let ptt = Shortcut::new(Some(Modifiers::ALT), Code::Space);
app.global_shortcut().on_shortcut(ptt, |app_handle, _shortcut, event| {
    if event.state == ShortcutState::Pressed {
        show_window(app_handle);
        let _ = app_handle.emit("shortcut:ptt:toggle", ());
    }
})?;
```

---

### 5.2 Database Layer — `db/`

**`db/migrations.rs`** — The schema is versioned. Each migration block adds new tables or settings and increments the version:

| Version | What was added |
|---|---|
| v1 | `settings`, `sessions`, `turns` tables; default settings |
| v2 | Audio input device, whisper binary/model, window context, voice threshold |
| v3 | Voice speed and expressiveness |
| v4 | `memories` table; embedding model setting |
| v5 | `source_file` column on memories; custom system prompt setting |
| v6 | `window_context_allowed` setting |

**Why versioned migrations?** Once users install the app, you can never change the database destructively. Each new version adds things without breaking existing data. The rule is: **never edit a migration that already ran**.

**`db/mod.rs`** — Thin helpers that do one thing each:

- `get_setting(conn, key)` — Read one setting by key
- `set_setting(conn, key, value)` — Write/update one setting
- `get_all_settings(conn)` — Read all settings (used at startup)
- `ensure_session(conn)` — Return the current session ID, or create a new one if the last session ended more than 4 hours ago
- `save_turn(conn, session_id, role, content)` — Save one message (user or assistant)
- `get_recent_turns(conn, session_id, limit)` — Get the N most recent messages (in chronological order)

---

### 5.3 Commands Layer — `commands/`

Commands are the public API that React can call.

#### `chat.rs` — Core conversation logic

**`send_message`** is the most complex command. Step by step:

```
1. Check for distress keywords (safety::check)
2. Lock DB, get/create session, save user turn, read all settings
3. Drop DB lock (important! can't hold lock across async)
4. Embed the user's message (Ollama embeddings)
5. Search memories for relevant past context (cosine similarity)
6. Read window context (if sharing is on)
7. Build system prompt = personality + memories + context
8. Build message history (last 20 turns)
9. Stream response from Ollama → emit chat:token events
10. Save assistant turn to DB
11. Emit chat:done
12. Emit safety:show if distress was detected
13. Spawn background task: every 4 turns, summarize + embed + save memory
14. Spawn TTS task: speak the response
```

**`get_greeting`** — Runs when Ollama first connects. Sends a one-line instruction asking the LLM to greet the user. Saves the reply and speaks it.

**`start_new_session`** — Forces a new session (clears the 4-hour reuse window).

**`speak_text`** — Speaks arbitrary text through TTS (used when user clicks a replay button).

#### `voice.rs` — Microphone recording

**`start_listening`**:
1. Pick the right audio device (CPAL)
2. Create a temp WAV file path
3. Start recording on a background thread (Arc<AtomicBool> stop flag)
4. Store the recording handle in `AppState`

**`stop_listening`**:
1. Set the stop flag to true
2. Wait briefly for capture to flush
3. Run `whisper.cpp` as a subprocess on the WAV file
4. Return the transcribed text

#### `settings.rs` — App settings

**`get_settings`** — Returns a big `Settings` struct with all values from the DB.

**`update_setting`** — Writes one key/value pair to the DB.

#### `setup.rs` — Download manager

Handles downloading Piper, Whisper model, and Kokoro model files. Uses Reqwest for HTTP downloads. `check_setup()` returns a `SetupStatus` struct saying what's installed.

#### `system.rs` — Utility commands

- `get_ollama_status` — GETs `http://localhost:11434/api/tags` to check if Ollama is running
- `list_models` — Returns the list of available Ollama models
- `export_conversation` — Opens a native save dialog and writes a Markdown file
- `expand_to_main` — Shows the main window (called from the widget)

#### `memory.rs` — Memory management

- `get_memories` — Returns memories sorted by creation date (for the Memory panel UI)
- `get_memory_count` — Returns count
- `delete_memory` — Deletes one memory by ID
- `forget_all` — Deletes all memories

#### `rag.rs` — Document question-answering

- `pick_document` — Opens a file picker dialog
- `ingest_document` — Extracts text, chunks it, embeds each chunk, saves to memories table with `source_file`
- `list_documents` — Returns list of ingested document filenames
- `delete_document` — Deletes all memory chunks for a document

#### `context.rs` — Window context sharing

- `get_window_title` — Returns the current active window title
- `start_sharing_context` — Sets `sharing = true` in AppState, captures current title
- `stop_sharing_context` — Sets `sharing = false`
- `set_context_note` — Adds a custom text note to the context

---

### 5.4 LLM Client — `llm/`

**`client.rs`** contains `stream_chat()`. This function:

1. Builds an HTTP POST to `{endpoint}/api/chat` with the Ollama streaming format
2. Sets `temperature: 0.85`, `num_predict: 512`
3. Reads the response as a byte stream
4. Each line is a JSON chunk: `{"message": {"content": "token"}, "done": false}`
5. For each non-done chunk: emits `chat:token` event to the frontend
6. When `done: true`: returns the full assembled response string

The frontend's `Transcript.tsx` component listens for `chat:token` events and appends them one by one, creating the streaming "typewriter" effect.

---

### 5.5 Text-to-Speech — `tts/`

There are three TTS engines. The `piper_voice` setting determines which one runs:

| Setting value | Engine | Example |
|---|---|---|
| `en_US-amy-medium` | **Piper** | Default, downloads a .onnx voice model |
| `sapi:Microsoft Heera` | **Windows SAPI** | Uses Windows built-in voices |
| `kokoro:af_bella` | **Kokoro Neural** | High-quality Python-based neural TTS |

**`tts/piper.rs`** — `speak()` is the router:

```rust
pub async fn speak(app, piper_binary, voice, text, speed, expressiveness, 
                   kokoro_model, kokoro_voices) {
    if voice.starts_with("sapi:") {
        speak_sapi(voice_name, text)
    } else if voice.starts_with("kokoro:") {
        kokoro::speak_kokoro(voice_id, text, kokoro_model_path, voices_bin_path)
    } else {
        // Piper subprocess
        piper_process = spawn(piper_binary, ["--model", voice_path, "--output_raw"])
        pipe text → stdin
        read raw PCM from stdout → convert to WAV → play
    }
}
```

**`tts/mod.rs`** — Shared helpers used by both Piper and Kokoro:
- `temp_wav_path()` — Returns a temp directory path like `%TEMP%/vp_tts_XXXXX.wav`
- `play_wav(app, path)` — Emits `speak:start`, plays the WAV file using CPAL (audio output), emits `speak:end`

**`tts/kokoro.rs`** — Runs Kokoro via a Python subprocess. A short Python script is embedded directly as a Rust string constant. The script:
1. Loads the Kokoro ONNX model
2. Loads `voices.bin` (voice embeddings)
3. Calls `kokoro_onnx.generate(text, voice=voice_id)`
4. Writes the audio to a WAV file
5. Exits

This avoids needing a running Python server — each TTS call is a fresh subprocess.

---

### 5.6 Audio Capture & STT — `audio/`

**`capture.rs`** — Records audio using CPAL (Cross-Platform Audio Library):

1. Lists available input devices, picks the user's selection or default
2. Targets 16kHz mono (what Whisper needs)
3. If the device provides a different sample rate (e.g. 44100Hz), linearly interpolates to 16kHz
4. Stores samples in a buffer, writes to WAV when the `Arc<AtomicBool>` stop flag is set
5. The stop flag allows `stop_listening()` to halt recording from a different thread

**`stt.rs`** — Runs whisper.cpp as a subprocess:

```
whisper-cli.exe -m ggml-base.bin -f recording.wav --output-txt --no-timestamps
```

Reads stdout, strips leading/trailing whitespace, returns the transcribed text. If whisper is not configured, returns an empty string.

---

### 5.7 Memory System — `memory/` + `embed/` + `summarize/`

VoicePartner remembers things across sessions using semantic memory search.

**How it works:**

1. Every 4 turns, `summarize/mod.rs` asks the LLM: *"Summarize this conversation in 2-3 sentences"*
2. The summary is converted to a vector (array of 768 floats) by `embed/mod.rs` via Ollama's `nomic-embed-text` model
3. The vector is saved to the `memories` table as raw bytes (BLOB)

**Recall at message time:**

1. The user's new message is embedded into a vector
2. `memory/mod.rs::search_memories()` scans the 200 most recent memories
3. Computes **cosine similarity** between the query vector and each memory vector
4. Returns the top 3 memories with similarity > 0.5
5. These are injected into the system prompt as context

**Cosine similarity** measures how "pointing in the same direction" two vectors are. Two texts about similar topics produce similar vectors, giving high similarity (close to 1.0). Unrelated texts give low similarity (close to 0).

**`embed/mod.rs`** — Simple POST to `{endpoint}/api/embeddings`:
```json
{ "model": "nomic-embed-text", "prompt": "text to embed" }
```
Returns a `Vec<f32>` (array of floats).

---

### 5.8 RAG — Document Q&A — `rag/`

RAG = Retrieval-Augmented Generation. It lets you ask questions about your own documents.

**Ingestion** (`ingest_document`):
1. Pick a file (PDF, TXT, DOCX)
2. Extract raw text
3. Split into ~500-character chunks with ~100-character overlap (so context isn't cut mid-sentence)
4. Embed each chunk
5. Save to `memories` table with `source_file = filename`

**At chat time**, the same memory search that finds past conversation summaries also finds relevant document chunks — they're stored in the same table with the same search algorithm.

**`chunk_text()`** splits on sentence/paragraph boundaries when possible, with overlap to ensure context continuity at chunk edges.

---

### 5.9 Safety System — `safety/`

**`safety/mod.rs`** runs before every LLM call. It checks the user's message for distress signals using keyword matching:

- **DistressLevel::None** — Normal message, no special handling
- **DistressLevel::Mild** — Keywords like "sad", "lonely" — companion acknowledges and gently suggests human connection
- **DistressLevel::High** — Keywords like "suicidal", "hurt myself" — companion adds a clear signposting suffix and emits `safety:show` to display the Safety Panel

The safety panel shows emergency contact information and mental health resources.

---

### 5.10 Context Sharing — `context/`

**`context/mod.rs`** — `get_active_window_title()` uses Windows API (via `winapi` crate) to get the title of the currently focused window. For example: `"main.rs - VoicePartner - VS Code"`.

This is injected into the system prompt so the companion knows what you're working on:

```
[CONTEXT — what the user is working on right now]
Active window: main.rs - VoicePartner - VS Code
[/CONTEXT]
```

---

## 6. Frontend — React (`src/`)

### 6.1 Stores (State)

Zustand stores are like global React state. Any component can read from or write to them without prop-drilling.

**`store/chatStore.ts`**:
```typescript
{
  messages: Message[],        // All displayed messages
  isProcessing: boolean,      // Waiting for LLM response?
  isSpeaking: boolean,        // TTS playing?
  
  // Actions:
  addMessage(msg)             // Add a complete message
  appendToken(token)          // Add streaming token to last assistant message
  finalizeStream()            // Mark streaming as complete
  setProcessing(bool)
  setSpeaking(bool)
  clearMessages()
}
```

**`store/settingsStore.ts`**:
```typescript
{
  settings: Settings,         // All app settings (flat key/value)
  loaded: boolean,            // Has loadSettings() completed?
  
  // Actions:
  load()                      // Call get_settings Tauri command, populate settings
  update(key, value)          // Call update_setting, update local state
}
```

**`store/ollamaStore.ts`**:
```typescript
{
  status: "connecting" | "connected" | "disconnected",
  models: OllamaModel[],
  
  // Actions:
  poll()                      // Check Ollama status + list models
}
```

`ollamaStore` polls every 5 seconds in the background using `setInterval`.

---

### 6.2 Components

| Component | File | What it does |
|---|---|---|
| **App** | `App.tsx` | Main shell: layout, event listeners, panel state |
| **Widget** | `Widget.tsx` | 400×58px floating overlay (always on top) |
| **Transcript** | `Transcript.tsx` | Chat message list with auto-scroll |
| **VoiceButton** | `VoiceButton.tsx` | Big Space-bar button — starts/stops recording |
| **VoiceVisualizer** | `VoiceVisualizer.tsx` | 20-bar animated waveform during recording |
| **StatusBar** | `StatusBar.tsx` | Bottom bar: Ollama status, model name, voice name |
| **SettingsPanel** | `SettingsPanel.tsx` | Full settings drawer (voice, model, memory, etc.) |
| **VoiceGallery** | `VoiceGallery.tsx` | Browse/download/select voice models |
| **MemoryPanel** | `MemoryPanel.tsx` | View and delete stored memories |
| **DocumentPanel** | `DocumentPanel.tsx` | Upload documents for RAG |
| **ModelSetupPanel** | `ModelSetupPanel.tsx` | First-time Piper + Whisper setup wizard |
| **SafetyPanel** | `SafetyPanel.tsx` | Emergency resources overlay |
| **AboutDialog** | `AboutDialog.tsx` | Version, keyboard shortcuts, license |
| **Onboarding** | `Onboarding.tsx` | 4-step first-run setup wizard |
| **ContextPermissionDialog** | `ContextPermissionDialog.tsx` | Asks permission before sharing window title |

**How the chat UI works:**

`App.tsx` wires up Tauri event listeners on mount:

```typescript
onChatToken((token) => appendToken(token))    // Build up streaming reply
onChatDone(() => { finalizeStream(); setProcessing(false) })
onSpeakStart(() => setSpeaking(true))
onSpeakEnd(() => setSpeaking(false))
```

`Transcript.tsx` renders `chatStore.messages` and auto-scrolls to the bottom on each new token. Empty state shows instructions.

`VoiceButton.tsx` listens for `Space` keydown and `shortcut:ptt:toggle` events (from Alt+Space global shortcut). On press: calls `start_listening`. On release/second press: calls `stop_listening` → gets transcript → calls `send_message`.

`VoiceVisualizer.tsx` animates 20 bars using `requestAnimationFrame` with a sinusoidal pattern — purely visual, not connected to actual audio amplitude.

---

### 6.3 Tauri Bridge — `lib/tauri.ts`

This file is the **typed contract** between frontend and backend. Every Rust command and event has a TypeScript wrapper here:

```typescript
// Typed wrapper for a command
export async function sendMessage(content: string): Promise<void> {
  return invoke("send_message", { content });
}

// Typed wrapper for an event listener
export async function onChatToken(cb: (token: string) => void) {
  return listen<string>("chat:token", (e) => cb(e.payload));
}
```

Having this layer means the rest of the frontend never imports from `@tauri-apps/api` directly — everything goes through `lib/tauri.ts`, making it easy to find all Rust/frontend interactions in one place.

---

## 7. Database Schema

**`settings`** — Key-value store for all app configuration:
```sql
settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)
```

**`sessions`** — A conversation session (reused within 4 hours):
```sql
sessions (id TEXT PRIMARY KEY, started_at INTEGER, ended_at INTEGER)
```

**`turns`** — Individual messages:
```sql
turns (
  id TEXT PRIMARY KEY,
  session_id TEXT REFERENCES sessions(id) ON DELETE CASCADE,
  role TEXT CHECK(role IN ('user', 'assistant')),
  content TEXT,
  created_at INTEGER   -- Unix timestamp in milliseconds
)
```

**`memories`** — Semantic memories (session summaries + document chunks):
```sql
memories (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  content TEXT,        -- Human-readable summary text
  embedding BLOB,      -- Raw f32 bytes of the vector
  memory_type TEXT,    -- 'session_summary' or 'document_chunk'
  created_at INTEGER,
  source_file TEXT     -- Set for document chunks; NULL for session summaries
)
```

**`schema_version`** — Tracks migration state:
```sql
schema_version (version INTEGER PRIMARY KEY)
```

---

## 8. Key Data Flows

### 8.1 Sending a Chat Message

```
User types and presses Enter (or voice input is transcribed)
  │
  ▼
VoiceButton / Transcript → chatStore.setProcessing(true)
                         → invoke("send_message", { content })
  │
  ▼
Rust: send_message()
  ├─ safety::check(content)           → DistressLevel
  ├─ db::ensure_session()             → session_id
  ├─ db::save_turn("user", content)
  ├─ embed::embed_text(content)       → query_embedding
  ├─ memory::search_memories()        → top 3 relevant memories
  ├─ context::get_active_window()     → window title (if sharing)
  ├─ Build system_prompt (personality + memories + context)
  ├─ db::get_recent_turns(20)         → conversation history
  ├─ llm::stream_chat(messages)
  │   └─ For each token: app.emit("chat:token", token)
  │                                   → Frontend appends to UI
  ├─ db::save_turn("assistant", response)
  ├─ app.emit("chat:done")
  ├─ If distress: app.emit("safety:show")
  ├─ tokio::spawn(summarize + embed + store memory every 4 turns)
  └─ tokio::spawn(tts::piper::speak(response))
      └─ app.emit("speak:start") → TTS plays → app.emit("speak:end")
```

### 8.2 Recording a Voice Message

```
User presses Space
  │
  ▼
VoiceButton → invoke("start_listening")
  │
  ▼  
Rust: start_listening()
  ├─ CPAL: find audio device
  ├─ Create temp WAV file
  ├─ Start recording thread (Arc<AtomicBool> stop=false)
  └─ Store ActiveRecording in AppState

User presses Space again
  │
  ▼
VoiceButton → invoke("stop_listening")
  │
  ▼
Rust: stop_listening()
  ├─ Set stop_flag = true
  ├─ Wait for recording thread to flush
  ├─ Run: whisper-cli.exe -m model.bin -f recording.wav
  └─ Return transcribed text

Frontend receives transcript
  └─ Calls send_message(transcript) → same flow as §8.1
```

### 8.3 Memory Recall

```
Every 4 turns (background task):
  │
  ├─ Get last 8 turns from DB
  ├─ Ask LLM: "Summarize this in 2-3 sentences"
  ├─ Embed the summary → Vec<f32> (768 dimensions)
  └─ Save to memories table (BLOB bytes)

On next send_message():
  │
  ├─ Embed user's message → query vector
  ├─ Load 200 most recent memories from DB
  ├─ For each memory: compute cosine_similarity(query, memory.embedding)
  ├─ Keep top 3 with score > 0.5
  └─ Inject as "[MEMORIES]" block in system prompt
```

### 8.4 TTS Routing

```
piper_voice setting determines which engine:

"en_US-amy-medium"    →  Piper subprocess
                           piper.exe --model voices/en_US-amy-medium.onnx
                           stdin: text → stdout: raw PCM
                           Convert PCM → WAV → play via CPAL

"sapi:Microsoft Heera" →  Windows SAPI via PowerShell
                           powershell -c "Add-Type -Assembly System.Speech; ..."
                           Writes WAV to temp path → play via CPAL

"kokoro:af_bella"     →  Kokoro via Python subprocess
                           python -c "from kokoro_onnx import Kokoro; ..."
                           Writes WAV to temp path → play via CPAL
```

---

## 9. Settings Reference

All settings are stored as string key/value pairs in the `settings` table:

| Key | Default | Meaning |
|---|---|---|
| `endpoint` | `http://localhost:11434` | Ollama server URL |
| `model` | `llama3.2:8b` | Which Ollama model to use |
| `companion_name` | `Amy` | Name of the AI companion |
| `personality` | `gentle` | One of: gentle, calm, playful, energetic, mentor, caring |
| `piper_binary` | `` | Full path to `piper.exe` |
| `piper_voice` | `en_US-amy-medium` | Voice ID or `sapi:Name` or `kokoro:id` |
| `whisper_binary` | `` | Full path to `whisper-cli.exe` |
| `whisper_model` | `` | Full path to `.bin` model file |
| `audio_input_device` | `default` | Microphone device name |
| `voice_speed` | `1.0` | TTS playback speed (0.5–2.0) |
| `voice_expressiveness` | `0.667` | Piper expressiveness/noise scale |
| `embedding_model` | `nomic-embed-text` | Ollama model for embeddings |
| `custom_system_prompt` | `` | Overrides personality system prompt when set |
| `window_context_auto` | `false` | Auto-inject active window title on every message |
| `window_context_allowed` | `unset` | User consent for window title access |
| `onboarding_done` | `false` | Whether first-run setup is complete |
| `custom_voices` | `[]` | JSON array of `{label, path}` custom Piper voices |
| `kokoro_model` | `` | Path to Kokoro ONNX model file |
| `kokoro_voices` | `` | Path to Kokoro voices.bin file |
| `voice_threshold_db` | `-30` | Silence threshold for VAD (dB) |

---

## 10. Key Design Decisions

### Why Tauri instead of Electron?

Electron bundles a full Chromium browser (~200MB). Tauri uses the OS's built-in WebView (~5-10MB overhead). VoicePartner's installer is under 10MB. Tauri also gives direct access to native Rust APIs for audio, system tray, and global shortcuts.

### Why SQLite instead of a cloud database?

Local-first is a core principle. All data stays on the machine. SQLite is a single file, zero-config, and fast enough for thousands of memories. No server means no subscription, no outage, no data leak.

### Why subprocess for Piper/Whisper/Kokoro instead of native libraries?

**Practical reasons:**

- `whisper.cpp` as a native Rust library (`whisper-rs`) requires a LLVM-based C++ build toolchain on Windows that many users won't have
- Kokoro's Python dependencies (kokoro_onnx, onnxruntime) are impractical to embed in Rust
- Piper's ONNX runtime has complex cross-platform build requirements

Subprocesses are slower to start but simpler to ship and maintain. The user installs the binary once, and the app just calls it.

### Why Zustand instead of Redux?

Redux requires boilerplate: actions, reducers, action creators. Zustand is a ~1KB library where the store is just a function. For a small desktop app with 3 stores, this is the right trade-off.

### Why store embeddings as raw bytes (BLOB)?

Ollama returns embeddings as a `Vec<f32>` (768 floats for nomic-embed-text). Storing them as raw bytes is the most compact format — 768 × 4 = 3072 bytes per memory. There's no need for a vector database like Chroma for hundreds of memories; a simple cosine similarity scan over 200 memories is under 1ms.

### Why Tokio spawn for TTS and memory?

Both TTS playback and memory summarization are "fire and forget" — the user shouldn't wait for them. Spawning them as background tasks means the chat UI updates immediately while audio plays in parallel.

### Why is `piper_voice` the TTS gate, not `piper_binary`?

Early versions checked `if !piper_binary.is_empty()` before doing TTS. This meant Windows SAPI and Kokoro voices (which don't need `piper_binary`) were silenced unless Piper was also configured. The fix: check `piper_voice` instead, since every valid TTS setup requires a voice selection.
