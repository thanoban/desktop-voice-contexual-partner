# Project file and folder architecture

This is the **target** layout. Only the current paths listed in the migration table exist today. Create planned modules when their roadmap work package is implemented, not as empty placeholder scaffolding. Keep the current Tauri repository; no monorepo conversion is required.

## 1. Repository map

```text
VoicePartner/
├── README.md                         Current setup and honest capability status
├── PLAN.md                           Product intent and accepted requirements
├── ARCHITECTURE.md                   Current and target system architecture
├── ROADMAP.md                        Work packages, dependencies, release gates
├── LICENSE                          Existing project license
├── package.json / package-lock.json Frontend dependencies and quality commands
├── index.html                       WebView entry
├── vite.config.ts                   Frontend build configuration
├── tsconfig*.json                   TypeScript boundaries
├── .github/workflows/               Planned CI; no credentials in repository
│   ├── verify.yml                   Rust/frontend checks and adapter contract tests
│   └── release.yml                  Qualified signed installer release
├── docs/
│   ├── README.md                    Reading order and authority
│   ├── project-structure.md         This module/file ownership map
│   ├── data-and-contracts.md        IPC, adapter, worker and storage contracts
│   ├── integrations.md             Engine-specific integration design
│   ├── voice-and-quality.md         Audio, models, metrics, evaluation
│   ├── security-and-privacy.md      Enforced permissions and data lifecycle
│   ├── feature-catalog.md           Feature journeys and acceptance
│   ├── release-and-operations.md    CI, release, installation and support
│   ├── decisions.md                Architectural decisions and sources
│   └── archive/                    Preserved historical documents
├── src/
│   ├── main.tsx                    Existing entry; select main or widget surface
│   ├── App.tsx                     Main shell and panel navigation
│   ├── Widget.tsx                  Compact partner/status surface
│   ├── styles.css                  Shared design tokens, focus and contrast
│   ├── components/
│   │   ├── Transcript.tsx          Final/partial turns, citations, interruption
│   │   ├── VoiceButton.tsx         PTT intent and truthful audio state
│   │   ├── VoiceVisualizer.tsx     Reduced-motion-aware input/output indication
│   │   ├── VoiceGallery.tsx        Voice audition, capabilities, pronunciation
│   │   ├── SettingsPanel.tsx       Preferences and privacy navigation
│   │   ├── Onboarding.tsx          Local/API choice and device setup
│   │   ├── ModelSetupPanel.tsx     Verified downloads, warm-up and repair
│   │   ├── MemoryPanel.tsx         Inspect/edit/pin/forget memories
│   │   ├── DocumentPanel.tsx       Project documents and ingestion state
│   │   ├── HistoryPanel.tsx        Session history and search
│   │   ├── ContextPermissionDialog.tsx
│   │   ├── SafetyPanel.tsx         Optional support resources
│   │   ├── StatusBar.tsx           Audio/provider/data-route readiness
│   │   ├── AboutDialog.tsx         Version, shortcuts and notices
│   │   ├── Composer.tsx           Typed input, draft correction, send/stop
│   │   ├── PartnerModeSwitch.tsx   Company/work/focus
│   │   ├── ProjectSwitcher.tsx     Client/project boundary
│   │   ├── ActivityPanel.tsx       Runs, approvals, artifacts, known outcomes
│   │   ├── ApprovalCard.tsx        Exact target and change review
│   │   ├── IntegrationsPanel.tsx   Connected tools, capabilities, health
│   │   ├── IntegrationWizard.tsx  Provider recipes, auth and read-only test
│   │   ├── ProviderPanel.tsx       Local/hybrid/cloud and usage controls
│   │   ├── RoutinePanel.tsx        Approved reusable workflows/schedules
│   │   └── DiagnosticsPanel.tsx   Redacted readiness and recovery
│   ├── hooks/
│   │   ├── useSessionEvents.ts    Snapshot + ordered backend events
│   │   ├── useRunEvents.ts        Execution updates and deduplication
│   │   └── useAccessibility.ts    Focus return and announcement handling
│   ├── store/
│   │   ├── chatStore.ts           View of backend session state, input draft
│   │   ├── settingsStore.ts       Typed preferences; cross-window refresh
│   │   ├── providerStore.ts       Provider readiness; replaces Ollama-only gating
│   │   ├── integrationStore.ts    Connection/capability metadata
│   │   ├── activityStore.ts       Run/event read models
│   │   └── projectStore.ts        Selected project and source list
│   └── lib/
│       ├── tauri.ts               Typed invoke/listen facade, legacy wrappers
│       ├── contracts.ts           Types generated from canonical schemas
│       ├── eventReducer.ts        Deterministic dedupe/stale-event handling
│       └── presentation.ts        Durations, costs, status wording
├── src-tauri/
│   ├── Cargo.toml / Cargo.lock    Native dependencies and pinned lockfile
│   ├── build.rs                  Tauri/resource integration
│   ├── tauri.conf.json            Window, bundling, CSP and updater config
│   ├── capabilities/             Least-privilege main/widget capabilities
│   ├── icons/                    Existing product assets
│   ├── resources/
│   │   ├── model-catalog.json     Versions, hashes, sizes, licenses, hardware
│   │   └── integration-recipes/   Tested setup metadata; no embedded secrets
│   ├── src/
│   │   ├── main.rs / lib.rs       Composition root, app state, tray, shutdown
│   │   ├── commands/             Thin Tauri handlers; service delegation only
│   │   │   ├── chat.rs / voice.rs / context.rs / memory.rs / rag.rs
│   │   │   ├── settings.rs / setup.rs / system.rs / history.rs
│   │   │   ├── providers.rs       Provider discovery, configuration, key setup
│   │   │   ├── integrations.rs    Connection CRUD, discovery, disable
│   │   │   ├── executions.rs      Submit, review, cancel, inspect
│   │   │   ├── projects.rs        Workspace and source boundaries
│   │   │   └── routines.rs        Saved workflow and schedule controls
│   │   ├── domain/
│   │   │   ├── session.rs         Session/turn IDs, modes, lifecycle
│   │   │   ├── provider.rs        Provider capability/request/result types
│   │   │   ├── integration.rs     Connection/capability types
│   │   │   ├── execution.rs       Requests, grants, approvals, results
│   │   │   ├── context.rs         Provenance and destination grants
│   │   │   └── events.rs          Versioned event envelopes and errors
│   │   ├── conversation/
│   │   │   ├── controller.rs      Foreground ownership and cancellation
│   │   │   ├── context_builder.rs Token budget and permitted context
│   │   │   ├── companion.rs       Personality/mode prompt composition
│   │   │   ├── checkins.rs        Opt-in eligibility and suppression
│   │   │   └── phrases.rs         Speakable clause segmentation
│   │   ├── audio/
│   │   │   ├── capture.rs         Callback-safe bounded capture
│   │   │   ├── devices.rs         Device enumeration/hot-plug
│   │   │   ├── resample.rs        Validated conversion
│   │   │   ├── preprocessing.rs   Noise and echo handling
│   │   │   ├── vad.rs             Speech boundaries and endpointing
│   │   │   ├── playback.rs        Native output and consumed sample offsets
│   │   │   └── stt.rs             Recognition adapter dispatch
│   │   ├── tts/
│   │   │   ├── mod.rs             Shared speech request and playback queue
│   │   │   ├── piper.rs / kokoro.rs / sapi.rs
│   │   │   ├── elevenlabs.rs      Paid streaming speech
│   │   │   └── pronunciation.rs   User dictionary and speakable normalization
│   │   ├── providers/
│   │   │   ├── mod.rs             Capability registry and profile validation
│   │   │   ├── ollama.rs          Native chat/tool streaming
│   │   │   ├── openai.rs          Official API adapter
│   │   │   ├── compatible.rs      Explicitly qualified compatible endpoints
│   │   │   ├── realtime.rs        Native-audio session abstraction
│   │   │   ├── openai_realtime.rs / gemini_live.rs
│   │   │   └── stream.rs          Incremental NDJSON/SSE decoders
│   │   ├── workers/
│   │   │   ├── supervisor.rs      Readiness, lifecycle, job objects, restart
│   │   │   ├── whisper.rs         Persistent recognition worker client
│   │   │   ├── kokoro.rs          Persistent synthesis worker client
│   │   │   └── resources.rs       RAM/VRAM admission and priority
│   │   ├── integrations/
│   │   │   ├── mod.rs             Versioned adapter interface and registry
│   │   │   ├── capabilities.rs    Namespacing, discovery cache, schema hashes
│   │   │   ├── mcp/
│   │   │   │   ├── client.rs      Negotiation, discovery, bounded tool calls
│   │   │   │   ├── stdio.rs       Owned process transport
│   │   │   │   ├── http.rs        Authenticated Streamable HTTP transport
│   │   │   │   └── auth.rs        OAuth/token references and revocation
│   │   │   ├── openclaw/
│   │   │   │   ├── gateway.rs     Authentication, documented RPC/events
│   │   │   │   ├── sessions.rs    Project/remote session mapping
│   │   │   │   └── runs.rs        Submit/wait/cancel/approval/reconcile
│   │   │   ├── n8n.rs            Workflow metadata and setup recipe
│   │   │   ├── zapier.rs          Setup recipe over generic MCP
│   │   │   ├── webhook.rs        Fixed destination/input adapter
│   │   │   └── fake.rs            Deterministic adapter for qualification
│   │   ├── execution/
│   │   │   ├── router.rs          One executor, explicit workflow precedence
│   │   │   ├── service.rs         Ledger-first orchestration and run limits
│   │   │   ├── approvals.rs       Exact operation binding and expiry
│   │   │   ├── verification.rs    Evidence and outcome normalization
│   │   │   ├── recovery.rs        Reconcile incomplete runs after restart
│   │   │   └── schedules.rs       Single owner and external schedule IDs
│   │   ├── policy/
│   │   │   ├── grants.rs          Source/action/destination scopes
│   │   │   ├── effects.rs         Read/write/send/delete/etc. classification
│   │   │   ├── egress.rs          Profile and endpoint checks
│   │   │   └── recursion.rs       Delegation ancestry and bounded depth
│   │   ├── context/
│   │   │   ├── mod.rs             Context broker
│   │   │   ├── window.rs          Permitted foreground metadata
│   │   │   ├── selection.rs       User-selected text
│   │   │   └── screenshot.rs      Explicit window/region capture
│   │   ├── platform/
│   │   │   ├── windows.rs         Narrow native action implementations
│   │   │   ├── credentials.rs     OS credential store access
│   │   │   └── paths.rs           Canonical allowed paths and app data dirs
│   │   ├── memory/               Profile, recall, corrections, deletion
│   │   ├── rag/                  Parsing, chunking, indexing, citations
│   │   ├── embed/                Versioned local embedding provider
│   │   ├── summarize/            Idle-priority attributed summarization
│   │   ├── safety/               Support responses; no diagnostic claims
│   │   ├── db/
│   │   │   ├── mod.rs / migrations.rs
│   │   │   ├── repositories.rs   Scoped DB operations
│   │   │   ├── encryption.rs     Keyed opening and atomic conversion
│   │   │   └── backup.rs         Backup verification and restore
│   │   ├── api/
│   │   │   ├── local.rs          Opt-in authenticated loopback API
│   │   │   └── mcp_server.rs     Scoped VoicePartner capability exposure
│   │   └── telemetry/
│   │       ├── timing.rs          Content-free stage timings
│   │       ├── usage.rs           Provider/tool cost provenance
│   │       └── diagnostics.rs     Redacted support export
│   └── tests/                    Native integration and migration tests
├── workers/
│   ├── whisper/                  Packaged persistent native worker
│   └── kokoro/                   Managed worker and pinned runtime recipe
├── contracts/
│   ├── schema/                   Canonical public JSON schemas
│   └── fixtures/                 Valid/invalid messages and version examples
├── integrations/
│   ├── recipes/                  Curated setup recipes
│   ├── examples/                 Minimal adapter and sample n8n workflow
│   └── compatibility.json        Tested versions and capabilities, initially empty
├── extensions/
│   └── vscode/                   Selected code/diagnostic bridge, stage 5
├── tests/
│   ├── frontend/                 Event reducer, forms and permission UI
│   ├── desktop/                  Installed Windows E2E
│   ├── adapters/                 Fake MCP/Gateway and protocol fixtures
│   └── fixtures/                 Synthetic projects/documents; no user data
├── evals/
│   ├── voice/                    Consented/licensed audio corpus manifests
│   ├── retrieval/                Grounded questions and expected sources
│   ├── workflows/                Supported tasks and verification assertions
│   └── reports/                  Redacted versioned evaluation summaries
└── scripts/
    ├── verify.ps1                Reproducible local quality commands
    ├── export-contracts.ps1       Generate TS and validate schema fixtures
    ├── package-workers.ps1        Reproducible worker packaging
    └── benchmark.ps1              Repeatable evaluation entry
```

Each Rust directory needs a `mod.rs` when implemented; omitted repeated module files above do not imply special loading. Exact internal helper files may be combined when small, but service responsibilities and public contracts must remain separated.

## 2. Existing-to-target migration

| Existing code | Destination and change | Stage |
|---|---|---|
| `commands/chat.rs` | Keep IPC wrappers; move orchestration to `conversation/controller.rs`; real new-session and cancellation | 1 |
| `llm/client.rs` | Move Ollama transport to providers; shared stream decoder; preserve callers through wrapper until migrated | 1–2 |
| `audio/capture.rs`, `commands/voice.rs` | Bounded capture with completion handshake; backend microphone ownership | 1 |
| `audio/stt.rs` | Persistent-worker adapter with existing CLI fallback shown as degraded | 1 |
| `tts/mod.rs`, `tts/piper.rs`, `tts/kokoro.rs` | Native playback and cancellable synthesis; remove PowerShell playback path | 1–2 |
| `context/mod.rs`, `commands/context.rs` | Permission-checking broker; revoke all active sharing paths | 1, 5 |
| `memory/mod.rs`, `rag/mod.rs`, `embed/mod.rs` | Project scope, provenance, stable source IDs, lexical/vector retrieval | 1, 5 |
| `db/migrations.rs` | Append migrations; never edit applied versions 1–6 | 0 onward |
| `App.tsx`, `Widget.tsx`, `chatStore.ts` | Snapshot/event driven views and shared backend ownership | 1 |
| `ollamaStore.ts`, `StatusBar.tsx`, `Onboarding.tsx` | Provider-neutral readiness; paid/text use must not require Ollama | 2 |
| `lib/tauri.ts` | Generated contracts behind stable typed facade | 0–2 |
| `commands/setup.rs` | Hash-verified downloads, atomic install, manifests and repair | 1, 6 |
| `tauri.conf.json`, capabilities | CSP, separate widget permissions, signed update/bundle configuration | 1, 6 |

## 3. Runtime storage layout

Use the existing application data location returned by Tauri rather than hardcoded user paths:

```text
<app_data>/
├── voicepartner.db              Encrypted store after migration
├── backups/                    Encrypted verified backups and migration manifests
├── models/<model-id>/<version>/ Hash-verified downloaded assets
├── workers/<worker>/<version>/  Qualified immutable worker runtime
├── artifacts/<project>/<run>/   User-approved output files
├── indexes/                    Rebuildable derived indexes when not inside SQLite
├── diagnostics/                Rotating redacted logs, explicit exports
└── temp/<session>/<turn>/       Restricted transient files, cleaned on restart
```

API keys and DB keys live in the OS credential store. Do not duplicate them in settings, recipes, environment dumps, support bundles, or source control. Existing `VoicePartner/models` and `VoicePartner/voices` assets are adopted through a migration inventory; never delete the user's originals before verifying copied assets.

## 4. Dependency rules

UI calls typed commands; commands call services; services depend on domain types and injected adapters/repositories. Adapters do not import UI or write conversation memory directly. Policy is shared across Tauri, API, MCP server, and scheduled routes. Tests inject fake clocks, transport, devices, and credentials where meaningful.

Do not put provider-specific JSON in frontend components. Do not put arbitrary shell execution behind a generic utility helper. Keep protocol version assumptions in their adapter and compatibility record. Never place model weights, credentials, personal transcripts, or generated binaries in Git.
