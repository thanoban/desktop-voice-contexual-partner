# VoicePartner — Development Roadmap

> What's been built, what comes next, and the long-term vision.  
> Ordered by priority. Each item includes a "why it matters" and rough implementation notes.

---

## Current State — v1.0.0 ✅

Everything below is **shipped and working** in the current release:

| Feature | Status |
|---|---|
| Text chat with Ollama (streaming) | ✅ |
| 6 personality presets (Gentle, Calm, Playful, Energetic, Mentor, Caring) | ✅ |
| Voice input (Space bar) via whisper.cpp | ✅ |
| Piper TTS (offline, .onnx voices) | ✅ |
| Windows SAPI TTS (Indian English: Heera/Ravi) | ✅ |
| Kokoro neural TTS (10 high-quality voices) | ✅ |
| Custom voice upload (.onnx) | ✅ |
| Persistent memory (Ollama embeddings + cosine search) | ✅ |
| Document RAG (PDF/TXT/DOCX → ask questions) | ✅ |
| System tray + widget (400×58px overlay) | ✅ |
| Alt+Space global push-to-talk shortcut | ✅ |
| Window context sharing (active app title) | ✅ |
| Safety panel (distress detection + resources) | ✅ |
| Export conversation to Markdown | ✅ |
| 4-step onboarding flow | ✅ |
| Auto download Piper/Whisper/Kokoro from Settings | ✅ |
| About dialog with keyboard shortcuts | ✅ |
| SQLite migrations (versioned, safe upgrades) | ✅ |
| NSIS installer (currentUser, no admin required) | ✅ |

---

## Near-Term — v1.1

These are the most-requested features and smallest gaps. Target: 1-2 months.

### 1. Conversation History Browser

**Why:** Users have no way to read past conversations. The `sessions` and `turns` tables store everything, but there's no UI to browse it.

**Implementation:**
- New `HistoryPanel.tsx` component (slide-in drawer, like SettingsPanel)
- Rust command: `list_sessions()` — return `(id, started_at, turn_count)` for each session
- Rust command: `get_session_turns(session_id)` — return all turns in order
- In history panel: show sessions as a list (grouped by day), click to expand and read
- "Load into chat" button: imports an old session's turns into the current view
- Settings button opens history panel (add a "History" tab to the settings footer)

### 2. Interrupt TTS Mid-Speech

**Why:** If the companion is speaking a long response, there's no way to stop it. Users must wait.

**Implementation:**
- Store the CPAL output stream handle in `AppState`
- When `stop_speaking` command is called: drop the stream or write silence
- Add `stop_speaking` button to the UI (appears while `isSpeaking = true`)
- Keyboard shortcut: `Escape` during TTS playback

### 3. Voice Activity Detection (VAD) — Auto-Send

**Why:** Push-to-talk requires two Space presses. Hands-free operation would let the user just speak and the app auto-sends after silence.

**Implementation:**
- During recording, compute RMS of audio buffer chunks
- If RMS < threshold for 1.5 seconds: automatically stop recording and send
- `voice_threshold_db` setting already exists in the DB (default: -30dB)
- Add a toggle in Settings: "Auto-send after silence"
- This removes the need for the second Space press

### 4. Emotion-Aware Voice Speed

**Why:** If the LLM's response is particularly warm or excited, TTS at a fixed speed sounds flat.

**Implementation:**
- After getting the LLM response, prompt a quick classification: "What is the emotional tone: calm, warm, excited, serious?"
- Map tone → speed multiplier (calm: 0.95×, excited: 1.05×, warm: 1.0×)
- Apply the multiplier to the user's base `voice_speed` setting
- Can be toggled off in Settings ("Match voice to response mood")

### 5. Multiple Companion Profiles

**Why:** Some users might want a work companion (Mentor personality, professional voice) and a personal one (Caring personality, warm voice). Currently there's only one.

**Implementation:**
- New `profiles` table: `(id, name, personality, piper_voice, model, custom_system_prompt)`
- Profile switcher in the title bar (dropdown)
- Each profile has its own settings; global settings (whisper path, etc.) remain shared
- On switch: reload settings store, restart greeting

### 6. Memory Panel Improvements

**Why:** The current Memory panel shows a raw list. Users can't search or understand what the companion remembers.

**Implementation:**
- Search box: filter memories by text content
- Memory categories: distinguish session summaries from document chunks visually
- Memory editor: let users manually add/edit memories ("Remember that I prefer dark mode")
- Memory age visualization: color older memories more dimly

---

## Medium-Term — v1.2 to v1.5

Larger features requiring more design work. Target: 3-6 months.

### 7. Wake Word Detection

**Why:** Rather than a keyboard shortcut, users could say "Hey Amy" to activate voice input. True hands-free.

**Implementation:**
- Use Whisper in streaming/continuous mode, or a lightweight wake word model (e.g., `rustpotter` or `openWakeWord` Python binary)
- Background audio thread always running at low CPU
- On wake word detected: emit `shortcut:ptt:toggle`, start full recording
- Privacy: wake word detection runs 100% locally; no audio is transmitted until wake word fires
- Toggle in Settings (off by default due to CPU cost)

### 8. Real-Time Voice Conversation

**Why:** The current flow is push-to-talk with a clear user/companion turn structure. Fluid conversation — where both sides can interrupt — would feel much more natural.

**Implementation:**
- Requires streaming STT (process audio as it comes in, not at end of recording)
- Whisper's `--step` mode or continuous mode via WebSockets
- Interrupt current TTS when user starts speaking
- Much more complex state machine; likely a major refactor of audio/

### 9. Long-Term Memory Management (Recall & Forgetting)

**Why:** Over months of use, the memories table will grow to thousands of entries. The current 200-entry scan will miss old but important memories.

**Implementation:**
- **Memory consolidation**: periodically merge similar memories (cosine > 0.9) into one
- **Importance scoring**: track how many times a memory is recalled; prune low-recall memories after 90 days
- **Explicit learning**: when the user says "Remember that..." — create a manually-tagged high-priority memory
- **Memory timeline**: UI view showing when memories were created and from which session

### 10. Structured Notes & Goals

**Why:** The companion can hear about the user's goals but can't track them. A lightweight goal-tracking feature would give the companion a persistent to-do context.

**Implementation:**
- New `notes` table: `(id, content, note_type, created_at, completed_at)`
- Types: `goal`, `reminder`, `note`
- Commands: `add_note`, `list_notes`, `complete_note`
- Inject pending goals into system prompt: "User's current goals: ..."
- UI: Notes panel in Settings drawer
- Companion can suggest creating a note: "Want me to remember that?"

### 11. Ambient Sound / Background Music

**Why:** Many solo workers use lo-fi music or ambient sounds. Integrating this into the companion creates a more immersive experience.

**Implementation:**
- Bundled ambient sounds: rain, café, white noise, lo-fi
- CPAL audio output stream runs alongside TTS (duck TTS over ambient)
- Volume knob in the widget
- Can also play external audio files

### 12. Calendar / Daily Summary

**Why:** At the start of the day, the companion could summarize what the user was working on yesterday and remind them of their goals.

**Implementation:**
- `daily_summary` command: runs on app startup if last session was yesterday
- Pulls recent memories + goals, asks LLM for a brief summary
- Shows as the first message of the day
- Optional: hook into system calendar (Windows Calendar API) for actual meetings

---

## Long-Term Vision — v2.0+

These are larger architectural changes or major new capabilities.

### 13. Plugin System

**Why:** Different users want different integrations — some want VS Code context, others want browser tab titles or email summaries. A plugin system lets the community extend VoicePartner without changing core code.

**Design:**
- Plugins are Rust libraries (`.dll`/`.so`) loaded at startup
- Or simpler: plugins are shell scripts/Python scripts in a `plugins/` directory
- Plugin API: a set of hooks (`on_message`, `on_session_start`, `on_memory_save`)
- Example plugins: VS Code active file, browser page title, Spotify current track

### 14. End-to-End Encrypted Sync

**Why:** Users with multiple machines (work laptop + home desktop) want their memories and settings to sync.

**Design:**
- Zero-knowledge sync: data is encrypted with a user-held key before leaving the device
- Sync via a small relay server (user-hosted or opt-in cloud service)
- The local-first principle is maintained — the relay never sees plaintext
- Conflict resolution: last-write-wins for settings; append-only for memories

### 15. On-Device STT (replace whisper subprocess)

**Why:** The subprocess approach works but has a 500ms-2s startup latency on each recording. Embedding whisper directly would make STT feel instant.

**Implementation:**
- Use `whisper-rs` crate (Rust bindings for whisper.cpp)
- Load the model once at startup, keep in AppState
- Transcription becomes a function call instead of a subprocess
- Blocked by: LLVM build toolchain requirement on Windows. Solvable with a pre-built static library.

### 16. Multimodal Input (Images/Screenshots)

**Why:** If the companion can see what's on the screen (not just the window title), it can help much more specifically — "I see you're getting this error in your terminal".

**Implementation:**
- Screenshot capture: Windows GDI or DXGIOutputDuplication
- Pass image to a multimodal Ollama model (llava, bakllava)
- User controls: "Share my screen" button (not automatic — privacy)

### 17. Local Emotion Recognition

**Why:** If the companion could sense when the user sounds stressed from their voice, it could adapt its tone without being asked.

**Implementation:**
- Small ONNX emotion classifier running on WAV input
- Outputs: neutral / happy / stressed / tired
- Passed to system prompt: "User sounds stressed today — be extra gentle"
- Must be opt-in and clearly disclosed

### 18. Mobile Companion App

**Why:** The core experience (a voice companion that knows you) would work well on a phone, even if the LLM runs on a separate home server.

**Implementation:**
- Same Rust/Tauri stack; Tauri supports iOS and Android (experimental)
- LLM offloaded to home machine's Ollama via local network
- Memories synced via encrypted sync (see item 14)
- Simplified UI for small screens

---

## Maintenance & Quality

These apply continuously across all versions:

### Testing
- Unit tests for `memory::cosine_similarity`, `rag::chunk_text`, `safety::check`
- Integration test: `send_message` end-to-end with a real SQLite in-memory DB
- Frontend: Playwright e2e test for the onboarding flow
- Snapshot tests for Transcript rendering

### Performance
- Profile startup time (currently ~2s cold start) — most is Tauri WebView init
- Profile `send_message` latency breakdown: DB lock / embed / search / LLM first token
- Memory search: BTree index on `created_at`, but not on vectors — 200-item scan is fast; revisit at 10,000+ memories

### Accessibility
- Add `aria-label` to all icon buttons
- Ensure all panels are keyboard-navigable (Tab/Shift-Tab)
- High-contrast theme option
- Font size setting

### i18n / Localization
- UI strings are hardcoded English — extract to a `strings.ts` file
- Support RTL languages (Arabic, Hebrew) in Transcript

---

## Feature Wishlist (Community Ideas)

Items not yet prioritized but worth considering:

- **Note-taking mode**: dictate notes, save as Markdown files to a folder
- **Custom commands**: "Hey Amy, open my task list" → runs a shell command
- **Companion "moods"**: companion remembers if yesterday was rough and checks in today
- **Conversation search**: full-text search across all past sessions
- **Multiple LLM support**: OpenAI-compatible API option (for users with API keys who want GPT-4)
- **WebRTC**: optionally talk to companion over phone/browser from another device on local network
- **Companion journal**: end-of-day summary written to a Markdown file by the companion
- **Distress escalation**: after multiple high-distress signals, proactively offer to notify a trusted contact
- **Whisper language selection**: currently defaults to English; expose `--language` flag

---

## Release Process

1. **Develop** in a feature branch
2. **Test** locally with `cargo tauri dev`
3. **Build** installer with `cargo tauri build`
4. **Increment** version in `package.json`, `Cargo.toml`, `tauri.conf.json`
5. **Tag** the release: `git tag v1.x.x`
6. **Push** to GitHub: `git push && git push --tags`
7. **Attach** the NSIS installer from `src-tauri/target/release/bundle/nsis/` to the GitHub Release

---

*Last updated: v1.0.0 — May 2026*
