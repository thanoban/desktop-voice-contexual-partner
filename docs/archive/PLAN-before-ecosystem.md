# VoicePartner — Architecture & Implementation Plan

## UI Design Decision (Voice-Only)
**No text input box.** The product is voice-first and voice-only. The user speaks; the companion speaks back. A scrollable transcript shows what was said by both sides. The PTT (push-to-talk) button is the only interaction control. A developer affordance (Shift+click on transcript) opens a native dialog for typing during M0 development — removed in M1 when STT is wired.


## Mission (clarified 2026-07-13)

VoicePartner is a **Windows contextual partner**: a companion that talks with you while you work and knows your screen for conversational context. It is **not** a task assistant — it does not help you code, does not operate your computer, does not offer step-by-step instructions unless explicitly asked. Screen awareness exists so the conversation has context ("I see you've been in that spreadsheet a while"), not so it can do things for you. This distinguishes it from Windows Copilot Vision, Siri, and ChatGPT voice mode (all task-first), and positions it alongside emerging ambient-companion competitors (Desktop Companion, CielChan, Razer AVA) — VoicePartner's edge is fully local + professional tone + safety-aware, a combination none of them have.

## Implementation Status Corrections (as of 2026-07-13)

This document is the original technical plan; two decisions described below were **not** carried into the shipped implementation and should be treated as open work, not done:

- **Silero VAD**: planned in the audio pipeline (§ Audio Pipeline, Milestone 1) but never implemented — `audio/mod.rs` only contains `capture` and `stt`. Recording today is manual push-to-talk toggle only, no voice-activity auto-segmentation. See ROADMAP.md "Improvement Plan v2" for the current plan to add this.
- **sqlite-vec**: planned as the vector store (§ Vector Store) but the shipped `memory/mod.rs` does brute-force cosine similarity in Rust over BLOB-stored `Vec<f32>` embeddings instead of a loaded `vec0` virtual table. Fine at current scale (~hundreds of memories), but not what this document specifies.

Both are tracked as real gaps in [ROADMAP.md](ROADMAP.md).

## Context

VoicePartner addresses chronic low-grade loneliness for solo workers by providing a local-first, voice-native desktop companion. All processing runs on the user's machine — no cloud, no subscriptions, no surveillance. The product must feel warm, not transactional. Memory is the core differentiator.

This document is the canonical technical plan: stack decisions, module architecture, database schema, and a sequenced 6-month roadmap. Revisit before making any structural decision.

---

## 1. Technology Stack

### App Framework: **Tauri v2 (Rust + TypeScript/React)**

| Option | Decision | Reason |
|---|---|---|
| Tauri v2 | **CHOSEN** | ~5 MB binary overhead vs ~150 MB for Electron. Native OS APIs in Rust. Webview for UI. Best performance-to-developer-effort ratio for cross-platform desktop. |
| Electron | Rejected | 150–300 MB overhead blows the <500 MB memory budget before any ML models load. Node.js is poor for audio/ML pipelines. |
| Python + Qt/PySide6 | Rejected | Slow startup, harder cross-platform packaging, weaker async story, poor voice ecosystem. |
| Flutter Desktop | Rejected | Immature audio/native plugin ecosystem. Dart adds cognitive overhead for a solo dev. |

### Language Split

| Layer | Language | Reason |
|---|---|---|
| UI | TypeScript + React | Fastest iteration for chat/settings UI. Tauri webview handles rendering. |
| Backend / Core | Rust | Audio capture, STT, TTS, vector ops, SQLite, IPC — all need native performance and no GC pauses. |
| ML inference | Rust via `ort` crate | ONNX Runtime bindings — runs embedding models and Silero VAD without Python. |
| No Python dependency | — | Keeps startup fast, packaging simple, and footprint small. Python sidecar is a last resort. |

### Audio Pipeline

```
Mic → CPAL capture → ring buffer → Silero VAD (ONNX/ort)
                                        ↓ speech detected
                               whisper.cpp (whisper-rs crate)
                                        ↓ transcript
                               LLM context assembly → Ollama stream
                                        ↓ response text
                               Piper TTS (subprocess, piped audio)
                                        ↓ PCM samples
                               CPAL playback → speaker
```

- **CPAL**: Cross-platform audio I/O (Windows WASAPI, macOS CoreAudio, Linux ALSA/PipeWire). Handles device enumeration and hot-plug.
- **Silero VAD**: ONNX model (~1.8 MB), run via `ort`. No Python. Real-time on CPU.
- **whisper-rs**: Rust bindings to whisper.cpp. Ship `small.en` (~244 MB) as default, `base.en` (~141 MB) as fast option.
- **Piper TTS**: Invoke as a subprocess (`piper --model voice.onnx --output_raw`), pipe PCM to CPAL. Bundled voices as ONNX files.
- **Kokoro TTS** (optional): Same subprocess pattern, higher quality voices.
- **XTTS-v2** (opt-in): Requires a Python sidecar. Isolated, user must explicitly enable. Consent checkbox mandatory.

### Vector Store: **sqlite-vec**

| Option | Decision | Reason |
|---|---|---|
| sqlite-vec | **CHOSEN** | Runs inside SQLite as a loadable extension. Zero extra process, zero extra dependency, atomic writes with the rest of the DB. Handles tens of thousands of vectors easily. |
| ChromaDB | Rejected | Python dependency, separate process, ~200 MB overhead. |
| LanceDB | Rejected | Less mature Rust API, overkill for this scale. |
| FAISS | Rejected | No persistence built-in, complex cross-platform builds, no SQLite integration. |
| Qdrant | Rejected | Separate server process — wrong for a local desktop app. |

### Embedding Model: **nomic-embed-text-v1.5 (ONNX)**

- 137 MB ONNX model, 768-dim vectors (or 256-dim Matryoshka for speed)
- Runs on CPU via `ort`, GPU if available
- Strong semantic quality for English text
- Fallback: `all-MiniLM-L6-v2` (90 MB, 384-dim) if RAM is constrained
- Downloaded on first run; cached in app data dir

### UI Stack

- **React 18** + **TypeScript** via Vite
- **Zustand** for state (lightweight, no boilerplate, works well with Tauri IPC events)
- **Tailwind CSS v4** for styling
- **Framer Motion** for voice activity animations
- **Tauri v2 events** for streaming LLM tokens and audio state updates (server-sent style)

### Database: **SQLite + SQLCipher + sqlite-vec**

- `rusqlite` crate with bundled SQLite
- `sqlite-vec` as a loadable extension for vector search
- `SQLCipher` for at-rest encryption (AES-256)
- Encryption key stored in OS keychain: Windows DPAPI via `keyring` crate, macOS Keychain, Linux Secret Service
- Versioned migrations using `rusqlite_migration` crate

---

## 2. Module Architecture

```
voicepartner/
├── src-tauri/                        # Rust backend (Tauri)
│   ├── Cargo.toml
│   ├── build.rs
│   ├── src/
│   │   ├── main.rs                   # App entry, Tauri builder, plugin registration
│   │   ├── lib.rs                    # App state, shared handles
│   │   │
│   │   ├── commands/                 # Tauri #[command] handlers (IPC bridge)
│   │   │   ├── mod.rs
│   │   │   ├── chat.rs               # send_message, stream_response, interrupt
│   │   │   ├── audio.rs              # list_devices, set_device, mute, start/stop_listen
│   │   │   ├── memory.rs             # list_memories, update_memory, delete_memory, export, wipe
│   │   │   ├── documents.rs          # ingest_document, list_documents, delete_document
│   │   │   ├── settings.rs           # get_settings, update_settings, reset_defaults
│   │   │   ├── context.rs            # share_window_title, capture_region, stop_sharing
│   │   │   └── system.rs             # get_version, check_update (opt-in), get_ollama_status
│   │   │
│   │   ├── audio/
│   │   │   ├── mod.rs
│   │   │   ├── capture.rs            # CPAL mic capture, ring buffer, device hot-swap
│   │   │   ├── vad.rs                # Silero VAD via ort, speech start/end events
│   │   │   ├── stt.rs                # whisper-rs: transcribe PCM → String
│   │   │   ├── tts.rs                # Piper subprocess manager, voice selection
│   │   │   ├── playback.rs           # CPAL audio output, interrupt flag (AtomicBool)
│   │   │   └── devices.rs            # Cross-platform device enumeration
│   │   │
│   │   ├── llm/
│   │   │   ├── mod.rs
│   │   │   ├── client.rs             # Ollama HTTP client: /api/chat streaming, model list
│   │   │   ├── context_builder.rs    # Assembles: system prompt + profile + memories + docs + session
│   │   │   ├── summarizer.rs         # Post-session: calls LLM to extract facts/emotions/topics
│   │   │   └── safety.rs             # Distress keyword scan before sending to LLM
│   │   │
│   │   ├── memory/
│   │   │   ├── mod.rs
│   │   │   ├── db.rs                 # SQLite schema, migrations, CRUD
│   │   │   ├── embedder.rs           # ONNX embedding inference (ort), batching
│   │   │   ├── vector_store.rs       # sqlite-vec: insert, cosine search, delete
│   │   │   ├── retrieval.rs          # Top-K retrieval, MMR re-ranking
│   │   │   ├── profile.rs            # Structured user profile: name, facts, preferences
│   │   │   └── session.rs            # In-session turn buffer, context window management
│   │   │
│   │   ├── rag/
│   │   │   ├── mod.rs
│   │   │   ├── parser.rs             # PDF (pdf-extract), DOCX (docx-rs), TXT, MD → plain text
│   │   │   ├── chunker.rs            # Recursive character splitter, configurable size/overlap
│   │   │   ├── ingestion.rs          # Parse → chunk → embed → store pipeline
│   │   │   └── retrieval.rs          # Document-namespaced vector search, citation tracking
│   │   │
│   │   ├── safety/
│   │   │   ├── mod.rs
│   │   │   ├── distress.rs           # Keyword/pattern detection, returns SeverityLevel
│   │   │   └── helplines.rs          # Region-keyed crisis helpline data (static)
│   │   │
│   │   └── platform/
│   │       ├── mod.rs
│   │       ├── keychain.rs           # keyring crate: store/retrieve DB encryption key
│   │       ├── screen.rs             # Active window title (accessibility APIs), region screenshot
│   │       └── tray.rs               # System tray icon, context menu, quick controls
│   │
│   └── resources/                    # Bundled at build time
│       ├── models/
│       │   ├── silero_vad.onnx
│       │   ├── whisper-small.en.bin  # Default STT model
│       │   └── piper/
│       │       ├── en_US-amy-medium.onnx
│       │       ├── en_US-lessac-medium.onnx
│       │       ├── en_GB-alan-medium.onnx
│       │       └── en_US-ryan-medium.onnx
│       └── helplines.json            # Crisis helpline data by region
│
├── src/                              # React frontend
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── Chat/
│   │   │   ├── ChatTranscript.tsx    # Turn list, timestamps, citations
│   │   │   ├── ChatTurn.tsx          # User vs companion styling
│   │   │   ├── VoiceVisualizer.tsx   # Animated waveform during speech
│   │   │   └── TypingIndicator.tsx   # Token stream animation
│   │   ├── Controls/
│   │   │   ├── VoiceButton.tsx       # PTT hold, continuous mode toggle
│   │   │   ├── MuteButton.tsx
│   │   │   └── InterruptButton.tsx
│   │   ├── Panels/
│   │   │   ├── MemoryPanel.tsx       # List/edit/delete memories
│   │   │   ├── DocumentPanel.tsx     # Upload, list, delete docs
│   │   │   ├── SettingsPanel.tsx     # All settings, tabbed
│   │   │   └── SafetyPanel.tsx       # Crisis info, non-modal overlay
│   │   ├── Layout/
│   │   │   ├── CompactMode.tsx       # Always-on-top minimal window
│   │   │   ├── FullMode.tsx          # Standard window layout
│   │   │   └── StatusBar.tsx         # Connection status, sharing indicator
│   │   └── Onboarding/
│   │       ├── AgeGate.tsx           # 18+ confirmation
│   │       └── Disclosure.tsx        # AI companion disclosure
│   │
│   ├── store/
│   │   ├── chatStore.ts              # Messages, streaming state
│   │   ├── audioStore.ts             # Mic state, VAD state, TTS playing
│   │   ├── settingsStore.ts          # All user settings
│   │   ├── memoryStore.ts            # Memory list (for panel)
│   │   ├── documentStore.ts          # Document list
│   │   └── sessionStore.ts           # Current session context sharing state
│   │
│   ├── hooks/
│   │   ├── useVoice.ts               # PTT logic, VAD events
│   │   ├── useStream.ts              # LLM token stream → chatStore
│   │   ├── useOllamaStatus.ts        # Poll/event Ollama connection
│   │   └── useBreakReminder.ts       # 3-hour timer logic
│   │
│   └── lib/
│       ├── tauri.ts                  # Type-safe invoke wrappers
│       └── helplines.ts              # Region selector helper
│
├── tauri.conf.json
├── package.json
├── vite.config.ts
└── PLAN.md                           # This file
```

---

## 3. Database Schema

```sql
-- migrations/001_initial.sql

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE sessions (
    id         TEXT PRIMARY KEY,  -- UUID
    started_at INTEGER NOT NULL,  -- Unix ms
    ended_at   INTEGER,
    summary    TEXT               -- Post-session LLM summary JSON
);

CREATE TABLE turns (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL REFERENCES sessions(id),
    role        TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
    content     TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    audio_ms    INTEGER           -- TTS duration for assistant turns
);

CREATE TABLE memories (
    id          TEXT PRIMARY KEY,
    type        TEXT NOT NULL,    -- 'fact' | 'emotion' | 'topic' | 'followup'
    content     TEXT NOT NULL,
    source_session TEXT REFERENCES sessions(id),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    deleted     INTEGER NOT NULL DEFAULT 0  -- soft delete, hard-purge on wipe
);

-- sqlite-vec virtual table
CREATE VIRTUAL TABLE memory_vectors USING vec0(
    memory_id TEXT,
    embedding float[768]
);

CREATE TABLE user_profile (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE documents (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    path        TEXT NOT NULL,
    size_bytes  INTEGER NOT NULL,
    chunk_count INTEGER NOT NULL,
    ingested_at INTEGER NOT NULL
);

CREATE VIRTUAL TABLE document_vectors USING vec0(
    chunk_id   TEXT,
    doc_id     TEXT,
    content    TEXT,
    embedding  float[768]
);
```

**Migration strategy**: `rusqlite_migration` crate. Each migration is a numbered `.sql` file. Run on every startup before app window shows. Migrations are append-only — never edit an applied migration.

---

## 4. Key Design Decisions

### Context Assembly Order (FR-5 critical path)

Every LLM call assembles context in this order:

```
[SYSTEM PROMPT (personality preset)]
[USER PROFILE (structured facts, injected always)]
[RELEVANT MEMORIES (top-5 from vector search on current input)]
[RELEVANT DOCUMENT CHUNKS (top-3 if docs uploaded)]
[SHARED CONTEXT (window title / screen region if active)]
[SESSION HISTORY (last N turns, N = floor(model_ctx * 0.4 / avg_tokens_per_turn))]
[CURRENT USER INPUT]
```

N adapts dynamically: query the model's context window from Ollama's `/api/show`, reserve 40% for history, the rest for response generation headroom.

### Post-Session Summarization

Triggered when the user closes the session or after 30 minutes of inactivity:

1. Send last session turns to local LLM with a structured extraction prompt
2. LLM returns JSON: `{ facts: [], emotions: [], topics: [], followups: [] }`
3. Each item embedded and stored in `memory_vectors`
4. User profile updated if new persistent facts found
5. Session `summary` field updated

This runs as a background Tokio task — never blocks the UI.

### Memory Deletion (FR-5 — must be complete)

"Delete" = remove from `memories` table + remove row from `memory_vectors` virtual table. sqlite-vec supports direct row deletion by `memory_id`. On "wipe all": `DELETE FROM memories; DELETE FROM memory_vectors; DELETE FROM document_vectors; DELETE FROM documents; DELETE FROM turns; DELETE FROM sessions;` — then `VACUUM` to reclaim space.

### Audio Interrupt

TTS playback holds an `Arc<AtomicBool>` interrupt flag. When the user speaks (VAD detects onset) or presses the interrupt key:
1. Flag set to `true`
2. Piper subprocess receives SIGTERM
3. CPAL playback buffer drains and stops
4. STT begins immediately

Round-trip from "user starts speaking" to "mic is live" target: < 200 ms.

### Safety Detection (FR-9 — non-negotiable)

Distress check runs on **every user input** before it reaches the LLM:

```rust
fn check_distress(input: &str) -> SeverityLevel {
    // SeverityLevel: None | Mild | Moderate | Severe
    // Pattern match against curated keyword list + phrase patterns
    // Returns Severe for explicit self-harm/suicidal ideation
}
```

On `Severe`:
1. LLM still gets the message (companion responds warmly)
2. A non-modal `SafetyPanel` slides up in the UI with helpline info
3. Companion's response includes a gentle encouragement to contact a human
4. Panel persists until user dismisses it

The companion never attempts to provide therapy. It acknowledges, validates, and redirects.

---

## 5. Implementation Roadmap

### Milestone 0 — Project Foundation (Week 1–2)
**Goal**: Scaffold compiles and runs on all 3 platforms. Text chat works.

- [ ] `pnpm create tauri-app voicepartner` (React + TypeScript template)
- [ ] Add Rust dependencies: `tokio`, `rusqlite`, `rusqlite_migration`, `serde`, `reqwest`
- [ ] SQLite schema + migration runner (runs on startup)
- [ ] Ollama HTTP client: `/api/tags` (model list), `/api/chat` streaming via SSE
- [ ] Basic React chat UI: input box, send button, streaming token display
- [ ] Settings struct + persistence (JSON via `tauri-plugin-store`)
- [ ] Connection status indicator (polling /api/tags every 5s)
- [ ] Graceful error states: Ollama not running, model not loaded

**Deliverable**: Text chat with a local LLM works. Settings persist across restarts.

---

### Milestone 1 — Voice Pipeline (Week 3–5)
**Goal**: Full voice I/O loop working. The core interaction model is usable.

- [ ] CPAL mic capture: enumerate devices, start/stop stream, ring buffer
- [ ] Silero VAD: load ONNX model via `ort`, real-time speech detection
- [ ] Push-to-talk: hold space bar → capture audio → release → transcribe
- [ ] whisper-rs STT: transcribe PCM buffer → String, async via Tokio task
- [ ] Piper TTS: spawn subprocess, pipe text in, read PCM out, play via CPAL
- [ ] Bundle 4 Piper voice models + JSON configs
- [ ] Continuous mode: VAD-triggered recording with silence timeout
- [ ] Mute button: `AtomicBool` flag, fully stops CPAL stream
- [ ] Interrupt: `AtomicBool` flag, kills Piper subprocess, stops playback
- [ ] Voice visualizer component in React (amplitude from audio buffer)
- [ ] Configurable audio devices (input + output) in settings

**Deliverable**: Hold space, speak, release → companion speaks back. Fully voice-driven.

**Risk**: whisper.cpp cross-platform build (especially Windows). Prototype this in Week 1 of this milestone before other work. Use pre-built whisper.cpp binaries if needed.

---

### Milestone 2 — Memory System (Week 6–9)
**Goal**: Companion remembers across sessions. This is the product's core value.

- [ ] ONNX embedding model: download `nomic-embed-text-v1.5.onnx` on first run, cache in app data
- [ ] `embedder.rs`: tokenize → run inference via `ort` → return `Vec<f32>`
- [ ] `sqlite-vec` extension: load into SQLite connection, create virtual tables
- [ ] `vector_store.rs`: insert embeddings, cosine similarity search (top-K), delete by ID
- [ ] Session turn storage: every turn saved to `turns` table during conversation
- [ ] Post-session summarizer: background Tokio task, structured JSON extraction prompt
- [ ] Memory CRUD: list by type/date, edit content, delete (hard purge from DB + vectors)
- [ ] Top-K retrieval integrated into context builder
- [ ] User profile: structured key-value, injected into every system prompt
- [ ] Memory panel UI: list view, edit modal, delete with confirmation
- [ ] Export all data as JSON (single file, all tables)
- [ ] Wipe all memory: confirmed destructive action, VACUUM after

**Deliverable**: Companion references things from previous days. Remembers the user's name, job, ongoing projects. Memory panel shows what it knows.

**Risk**: Embedding model size + inference speed on low-end hardware. Profile on a CPU-only machine early. Consider 256-dim Matryoshka embeddings as fallback.

---

### Milestone 3 — RAG + Personality (Week 10–12)
**Goal**: Companion can discuss uploaded documents. Personality feels distinct.

- [ ] Document parser: PDF via `pdf-extract`, DOCX via `docx-rs`, TXT/MD as-is
- [ ] Recursive character chunker: 512 token chunks, 64 token overlap
- [ ] Ingestion pipeline: parse → chunk → embed → store in `document_vectors`
- [ ] Document retrieval: top-3 chunks by semantic similarity, with source tracking
- [ ] Citation display: UI shows which doc(s) were referenced per response
- [ ] Document panel: drag-drop upload, list with size/date, delete (removes all chunks)
- [ ] 4 personality presets: system prompts + default voice + response pacing config
- [ ] Preset selector UI with preview text
- [ ] Advanced: editable system prompt textarea
- [ ] Companion naming: user sets name, stored in profile, used in system prompt
- [ ] Response pacing: configurable delay between TTS sentences for "thinking" feel

**Deliverable**: Drop in a PDF, ask questions about it. Companion has a distinct personality that feels consistent.

---

### Milestone 4 — Safety + Onboarding + Context Sharing (Week 13–15)
**Goal**: All safety requirements met. First-run experience is clear. Context sharing works.

- [ ] Distress keyword detection: curated pattern list, `SeverityLevel` enum
- [ ] SafetyPanel component: non-modal slide-up, region-appropriate helplines
- [ ] Helplines JSON: US, UK, CA, AU, EU regions (configurable in settings)
- [ ] First-run flow: age gate (18+), AI disclosure, data storage explanation
- [ ] Break reminder: Tokio timer, 3 continuous hours → gentle suggestion in UI
- [ ] Active window title sharing: platform API (Windows: `GetForegroundWindow` + `GetWindowText`, macOS: Accessibility API, Linux: `xdotool` / `wnck`)
- [ ] Screen region capture: user draws rectangle, screenshot → send to vision model (LLaVA / Llama 3.2 Vision via Ollama)
- [ ] "Tell it what I'm doing" text input: appends to session context
- [ ] Sharing indicator: persistent status bar badge, color-coded
- [ ] One-click stop sharing: clears all shared context from session

**Deliverable**: Product is ethically complete. Safety system works. Sharing context is explicit and revocable.

---

### Milestone 5 — Polish, Accessibility, Platform (Week 16–19)
**Goal**: Feels like a finished product. Keyboard-navigable. Tray integration works.

- [ ] System tray: icon, right-click menu (mute, pause, open, quit)
- [ ] Compact always-on-top mode: minimal window (~300px wide), click to expand
- [ ] Keyboard navigation: all primary functions reachable without mouse
- [ ] Configurable hotkeys: PTT key, interrupt key, compact toggle
- [ ] Transcript font size: slider in settings (12–24px range)
- [ ] High-contrast theme: pure black/white palette, no mid-tones
- [ ] Ollama model selector: dropdown populated from `/api/tags`
- [ ] Custom endpoint: settings field for LM Studio / llama.cpp server URL
- [ ] Opt-in update check: single HTTP request to GitHub releases API
- [ ] Cross-platform testing: Windows CI, macOS CI, Ubuntu CI via GitHub Actions
- [ ] Database encryption: SQLCipher integration, key in OS keychain

**Deliverable**: App feels complete. Ready for user testing.

---

### Milestone 6 — Advanced Voice + Packaging (Week 20–24)
**Goal**: Higher-quality voice options. Distributable installers.

- [ ] Kokoro TTS: subprocess integration, voice selection in settings
- [ ] XTTS-v2 voice cloning: Python sidecar (isolated), consent checkbox, voice sample upload
- [ ] Speaking rate slider: affects Piper/Kokoro pitch-speed parameters
- [ ] Adjustable TTS output device (separate from system default)
- [ ] Performance profiling: end-to-end latency measurement, optimize bottlenecks
- [ ] Windows installer: NSIS/WiX via Tauri bundler
- [ ] macOS DMG: code signing setup, notarization guide
- [ ] Linux AppImage + .deb via Tauri bundler
- [ ] First-run model download: progress bar UI for whisper + embedding model
- [ ] Final memory: stress test with 10,000 memory chunks, verify retrieval quality

**Deliverable**: v1.0 release candidates for all 3 platforms.

---

## 6. Critical Path & Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| whisper.cpp cross-platform build fails | Medium | High | Prototype in Week 1 of M1. Use pre-built binaries as fallback. |
| Embedding inference too slow on CPU-only | Medium | High | Profile early (M2 Week 1). Fall back to 256-dim Matryoshka embeddings or smaller model. |
| sqlite-vec doesn't support deletion cleanly | Low | High | Test deletion + cosine search correctness before building memory UI. |
| Piper subprocess latency > 3s | Medium | Medium | Measure first sentence latency. Consider streaming PCM from Piper before full text is ready. |
| XTTS-v2 Python sidecar adds startup time | Low | Low | XTTS is opt-in. Sidecar starts only when enabled. |
| SQLCipher + sqlite-vec compatibility | Medium | Medium | Test combination early in M5. May need to use unencrypted SQLite + file-system-level encryption as fallback. |
| Tauri v2 WebView inconsistency cross-platform | Low | Medium | Test UI on all 3 platforms at end of each milestone, not just at end. |

### Prototype First (before committing to M1 work)
These must be proven before building the full pipeline around them:
1. `whisper-rs` compiling and transcribing on Windows
2. `ort` loading `silero_vad.onnx` and detecting speech onset correctly
3. `sqlite-vec` inserting + querying float vectors from Rust

---

## 7. Rust Crate List

```toml
[dependencies]
# Tauri
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-store = "2"
tauri-plugin-dialog = "2"
tauri-plugin-global-shortcut = "2"
tauri-plugin-notification = "2"

# Async
tokio = { version = "1", features = ["full"] }

# HTTP (Ollama client)
reqwest = { version = "0.12", features = ["json", "stream"] }

# Database
rusqlite = { version = "0.31", features = ["bundled", "sqlcipher"] }
rusqlite_migration = "1"

# ML / Audio
ort = { version = "2", features = ["cuda"] }    # ONNX Runtime
whisper-rs = "0.11"                              # whisper.cpp bindings
cpal = "0.15"                                    # Cross-platform audio

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }

# OS integration
keyring = "3"                                    # OS keychain
screenshots = "0.8"                              # Screen capture

# Document parsing
pdf-extract = "0.7"
docx-rs = "0.4"

# Utilities
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## 8. Licensing Decision

**Recommendation: AGPL-3.0**

Reason: If someone forks VoicePartner and ships a cloud-hosted version (removing the "local-first" guarantee), AGPL forces them to open-source their changes. This protects the core privacy promise. MIT would allow proprietary forks that could damage user trust in the brand.

If you want maximum adoption / permissive ecosystem: use MIT. Decide before first commit.

---

## 9. Scalability Considerations (Post-v1)

The architecture is designed to extend without rewrites:

| Future Feature | How the Architecture Supports It |
|---|---|
| Multi-model support (Claude, GPT via opt-in API key) | `llm/client.rs` is already behind a trait — add new implementors |
| Plugin system for personality packs | Personality presets are JSON configs — external files are trivial to add |
| Mobile companion (read-only memory sync) | SQLite DB is portable — export/import path already exists |
| Team/family companion (separate profiles) | Add `profile_id` FK to all tables — migrations handle this cleanly |
| Web UI (browser-based access) | Tauri backend is pure Rust — can add an HTTP server alongside Tauri |
| Fine-tuned companion models | Model selection in settings already supports any Ollama-served model |
| Multilingual STT | whisper-rs supports multilingual whisper models — add language selector |
| Vector store upgrade (e.g. DuckDB VSS) | `vector_store.rs` is behind a trait — swap implementation |

---

## 10. Development Environment Setup

```powershell
# Prerequisites
winget install Rustlang.Rustup
winget install OpenJS.NodeJS.LTS
winget install pnpm.pnpm
winget install Microsoft.VisualStudio.2022.BuildTools  # Windows only

# Rust targets for cross-platform (CI)
rustup target add x86_64-pc-windows-msvc
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
rustup target add x86_64-unknown-linux-gnu

# Project init (run once)
pnpm create tauri-app@latest voicepartner -- --template react-ts
cd voicepartner
pnpm install
pnpm tauri dev
```

**Ollama for local dev**:
```powershell
winget install Ollama.Ollama
ollama pull llama3.2:8b      # Main companion model
ollama pull llava:7b          # Vision model for screen sharing
ollama pull nomic-embed-text  # Fallback if local ONNX embedding is too slow
```

---

## 11. Definition of Done (v1.0)

- [ ] All FR-1 through FR-10 implemented and manually tested
- [ ] End-to-end voice turn latency < 3s measured on RTX 3060 equivalent
- [ ] Cold start < 5s on SSD-equipped machines
- [ ] Memory footprint (app only) < 500 MB with 8B model loaded separately
- [ ] 1,000 memory chunk stress test passes (retrieval quality verified)
- [ ] Distress detection triggers correctly on test phrases, never on normal conversation
- [ ] Signed installers for Windows and macOS
- [ ] Database encryption verified: DB file is unreadable without the keychain key
- [ ] All user data operations (delete, wipe, export) verified correct
- [ ] No outbound network calls during normal operation (verified via proxy/firewall)
