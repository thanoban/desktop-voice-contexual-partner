# VoicePartner

> A local-first AI voice companion for solo workers.  
> No cloud. No subscriptions. All AI runs on your machine.

---

## What is VoicePartner?

VoicePartner is a desktop companion that listens, talks, and remembers. It's built for people who work alone and want a warm presence nearby — not a productivity tool, not a therapist, just a companion that knows you over time.

Every conversation stays on your machine. Your memories, documents, and voice never leave your computer.

---

## Features

| Feature | Details |
|---|---|
| **Voice input** | Push-to-talk (Space bar or Alt+Space from anywhere) |
| **Multiple TTS voices** | Piper (offline), Windows SAPI (Indian English, neural), Kokoro neural TTS |
| **Persistent memory** | Remembers key facts across sessions via Ollama embeddings |
| **Document RAG** | Upload PDFs/text, ask questions grounded in your own files |
| **6 personalities** | Gentle · Calm · Playful · Energetic · Mentor · Caring |
| **System tray** | Runs quietly, pops up with Alt+Space |
| **Window context** | Optionally shares your active window title for context-aware replies |
| **Export conversations** | Save any session to Markdown |
| **100% local** | Ollama LLM · Piper TTS · whisper.cpp STT — nothing hits the internet |

---

## Requirements

| Component | Where to get it |
|---|---|
| **Ollama** | [ollama.com](https://ollama.com) — run `ollama pull llama3.2` |
| **Piper TTS** *(auto-downloaded)* | Via Settings › Voice Setup |
| **whisper.cpp** | [github.com/ggerganov/whisper.cpp/releases](https://github.com/ggerganov/whisper.cpp/releases) |
| **Kokoro TTS** *(optional, high quality)* | `pip install kokoro-onnx` + download models in Voice Gallery |

Windows 10/11 x64. macOS and Linux builds are untested but the code is cross-platform.

---

## Quick start

1. Install [Ollama](https://ollama.com) and pull a model:
   ```
   ollama pull llama3.2
   ```
2. Run the VoicePartner installer (`VoicePartner_1.0.0_x64-setup.exe`).
3. Launch VoicePartner from the Start Menu or system tray.
4. Complete the 4-step onboarding (name, personality, safety disclosure, Ollama connect).
5. Open **Settings › Voice Setup** to download Piper TTS and the Whisper model.
6. Press **Space** to speak. Press **Space** again to send.

---

## Keyboard shortcuts

| Key | Action |
|---|---|
| `Space` | Toggle voice input (start / stop & send) |
| `Alt + Space` | Global PTT — works even when the window is hidden in the tray |
| `Esc` | Close any open panel |
| `?` | Open About dialog (keyboard shortcuts reference) |

---

## Voice options

### Piper TTS (offline, recommended)
Download voices from the Voice Gallery in Settings. Available voices:
- **Amy** · **Lessac** · **Ryan** — US English
- **Alan** — UK English
- **Priyamvada** · **Pratham** — Hindi

### Windows SAPI
Your installed Windows voices appear automatically in the Voice Gallery.  
For Indian English: Windows Settings → Time & Language → Language & Region → Add **English (India)** to get Microsoft Heera (F) and Ravi (M).

### Kokoro Neural TTS (high quality)
1. Install Python: [python.org](https://python.org)
2. `pip install kokoro-onnx`
3. Open Settings › Voice Gallery › Kokoro Voices → Download model files
4. Choose any of the 10 Kokoro voices (US/UK, male/female)

### Custom voices
Drop any Piper-compatible `.onnx` model into the Voice Gallery via **+ Add custom voice**.

---

## Building from source

```bash
# Prerequisites: Rust (stable), Node.js 18+, Tauri CLI v2
git clone https://github.com/thanoban/desktop-voice-contexual-partner.git
cd desktop-voice-contexual-partner

npm install
cargo tauri dev          # development mode
cargo tauri build        # release installer → src-tauri/target/release/bundle/
```

---

## Architecture

```
VoicePartner/
├── src/                   React + TypeScript frontend
│   ├── components/        UI components
│   ├── store/             Zustand state (chat, settings, ollama)
│   └── lib/tauri.ts       Typed wrappers for all Tauri commands
└── src-tauri/             Rust backend
    └── src/
        ├── commands/      Tauri command handlers
        │   ├── chat.rs    LLM streaming, TTS dispatch
        │   ├── setup.rs   Download manager (Piper, Whisper, Kokoro)
        │   ├── memory.rs  Persistent memory CRUD
        │   └── rag.rs     Document ingestion (PDF, text)
        ├── tts/
        │   ├── piper.rs   Piper subprocess + Windows SAPI routing
        │   └── kokoro.rs  Kokoro neural TTS via Python subprocess
        ├── audio/
        │   ├── capture.rs CPAL microphone recording
        │   └── stt.rs     whisper.cpp subprocess STT
        ├── embed/         Ollama embedding client
        ├── memory/        Semantic memory search (cosine similarity)
        ├── rag/           Document chunking + vector search
        └── db/            SQLite schema + migrations
```

**Stack:** Tauri v2 · Rust · React 18 · TypeScript · Zustand · Tailwind CSS v4  
**LLM:** Ollama (local HTTP streaming)  
**TTS:** Piper subprocess · Windows SAPI · Kokoro ONNX  
**STT:** whisper.cpp subprocess  
**Memory:** SQLite + cosine similarity over Ollama embeddings  
**License:** AGPL-3.0

---

## Data & privacy

All user data is stored in:
- **Windows:** `%APPDATA%\com.voicepartner.app\`

Includes: SQLite database (conversations, memories, settings), downloaded model files. Nothing is ever transmitted to any external server.

---

## License

[AGPL-3.0](LICENSE) — free to use, modify, and redistribute under the same license.
