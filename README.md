# VoicePartner

VoicePartner is planned as a local-first personal voice partner for people who work at a PC. It combines warm conversation, continuity, quiet company, and useful work assistance. Optional MCP, OpenClaw, n8n, Zapier, editor, browser, and Windows integrations extend what it can do.

This checkout contains the companion, reliability groundwork, schema v9 retrieval/retention, and text-provider adapters for Ollama, OpenAI-compatible services and Anthropic. Automation integrations remain planned in main. The [2026-11-21 system review](docs/system-review-2026-11-21.md) records current capabilities, privacy and usability blockers, verification limits, and the remaining delivery sequence. The project is not ready for public beta.

Start with [PLAN.md](PLAN.md), then read [ARCHITECTURE.md](ARCHITECTURE.md), [docs/project-structure.md](docs/project-structure.md), [docs/integrations.md](docs/integrations.md), and [ROADMAP.md](ROADMAP.md). Historical documents and prior local notes are preserved under [docs/archive](docs/archive/README.md).

## Current baseline

The app includes a Tauri desktop shell, React interface, streaming text-provider abstraction, CPAL/whisper.cpp voice input, Piper/SAPI/Kokoro speech paths, SQLite history, FTS5 plus paged exact vector retrieval, document revisions/citations, retention controls, window context, tray/widget behavior, and setup downloads. Warm inference, VAD, phrase streaming, native playback, encryption, enforceable automation permissions, and integrations remain open. The current Local profile does not enforce locality for every auxiliary inference request; see review findings R01–R02 before relying on it for private processing.

Run `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1` for contract validation, the frontend production build, Rust formatting and lints, and Rust tests. These checks do not qualify live audio, cloud providers, integrations, installer behavior, or the companionship experience.

## Development principles

- Keep companionship usable when every integration is disconnected.
- Treat private local, hybrid, and cloud processing as explicit modes with visible destinations.
- Store credentials in the OS credential store and never commit them.
- Use one execution ledger and permission service for Tauri, local API, MCP, schedules, and external agents.
- Persist an external run identity before presenting a job as accepted; reconcile after disconnect before retrying.
- Report unknown outcomes honestly. Stopping local speech does not prove a remote task stopped.
- Preserve user data and append database migrations; never edit applied migrations.
- Separate code/build proof, automated tests, live service qualification, installed-device verification, and pilot evidence.

The repository keeps its existing AGPL basis. Before distributing new workers, voices, models, runtimes, or adapters, record their source, version, hash, and redistribution license in the release catalog.

## Existing local setup

The legacy local companion path uses Tauri v2, Rust, React/TypeScript, SQLite, Ollama, whisper.cpp, Piper, SAPI, and optional Kokoro. See the archived README for the original setup instructions while the new onboarding and provider-neutral setup are implemented.

## Links

- [Product plan](PLAN.md)
- [Architecture](ARCHITECTURE.md)
- [Documentation index](docs/README.md)
- [Roadmap](ROADMAP.md)
- [GitHub repository](https://github.com/thanoban/desktop-voice-contexual-partner)
