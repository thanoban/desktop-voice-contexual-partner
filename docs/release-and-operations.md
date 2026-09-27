# Release, testing and operations plan

The release process must show what is proven at each boundary. A design, successful build, automated test, live integration, installed desktop, and user pilot are separate evidence classes.

## 1. CI and local verification

Create the planned `.github/workflows/verify.yml` and `scripts/verify.ps1` with these lanes:

1. Documentation/link check, generated contract/schema check, and repository secret scan.
2. TypeScript strict build and frontend unit/event-reducer tests.
3. Rust format/check/clippy and unit tests.
4. SQLite migration/backup/restore tests against v6 fixtures and the current schema.
5. Fake provider/MCP/OpenClaw/n8n transport contract tests.
6. Audio algorithm tests using synthetic signals: resampling, endpointing, bounded buffers, phrase segmentation and cancellation state.
7. Windows installed smoke on a signed-build-capable runner: launch, tray, widget, model repair, migration, clean uninstall.
8. Separate opt-in live qualification jobs; credentials are managed outside the repository and never required for pull requests.

Commands must use project-local dependencies and create build artifacts only in ignored locations. A green CI lane is evidence for that lane only.

## 2. Test matrix

| Area | Required scenarios |
|---|---|
| Conversation | Empty/long input, provider error, split frames, reconnecting UI, duplicate/late events, session close/create |
| Audio | Silence, noise, accents, Bluetooth, device unplug, sleep/resume, non-f32 input, clipped speech, feedback |
| TTS | Queue backpressure, synthesis failure, playback failure, stop during every phase, late chunk suppression |
| Memory/RAG | Contradiction, deletion, old revision, same filename, project negative, missing answer, index rebuild |
| MCP | stdio/HTTP negotiation, auth expiry, malformed/oversized frame, schema change, pagination, malicious descriptions/results |
| OpenClaw | Pending wait, event gap, supersession/cancel, reconnect, unknown dispatch, downstream approval, WSL/remote path mapping |
| n8n/Zapier | Read-only setup, token redaction, workflow revision, usage limit, webhook replay, cloud destination disclosure |
| Policy | Wrong project/path/account, expired grant, changed arguments, recursive delegation, denied effect, self-approval |
| Desktop | Wrong focus, UI layout change, file junction/traversal, app crash, external result verification |
| Release | Clean machine, upgrade, rollback, failed download/hash, locked key store, restore backup, orphan worker cleanup |
| Accessibility | Keyboard only, focus order/return, labels, contrast, reduced motion, text-only mode, readable action review |

## 3. Performance and quality runs

Each report records commit/version, model/provider revision, hardware/driver/power mode, OS, context size, warm/cold state, network conditions, and exact corpus/task version.

- **Voice:** p50/p95 end-of-speech to meaningful first audio, first token, first PCM, stop, interruption; WER and entity-match by subgroup; listener comfort/naturalness.
- **Retrieval:** source support, citation correctness, missing-answer behavior, cross-project leakage.
- **Automation:** task completion, verified postcondition, duration, corrections, duplicate rate, cancellation/reconciliation, cost/usage.
- **Resource:** RAM, VRAM, CPU, battery/power profile, model load/unload, orphan processes, eight-hour soak.
- **Companionship:** consenting pilot surveys and diary/exit interviews; do not treat engagement time as a proxy for reduced loneliness.

Reports are redacted before committing to `evals/reports/`. Never commit raw voices, private transcripts, tokens, client data or screenshots.

## 4. Packaging and updates

Before beta:

- Build reproducible Windows installer artifacts with version, commit, dependency/model manifest, hashes and license inventory.
- Sign installer/update metadata when signing credentials are available; verify signature and hash on a clean machine.
- Package only qualified workers/models or provide hash-verified user downloads. Never ship unreviewed dynamic executables.
- Run migration backup/restore and failed-update rollback tests. Do not make an update that can strand the user's only database copy.
- Make repair/re-download idempotent and atomic. Preserve user models/voices until the replacement is verified.
- Ensure uninstall removes application-managed temporary/worker data according to the disclosed retention choice, while user-selected project artifacts remain user-controlled.
- Keep feature flags for integrations, local API, check-ins, and hands-free mode. Default unqualified or privacy-sensitive features off.

## 5. Operational behavior

Health is split into app, database, audio device, worker, provider, integration, and run states. One failure must not make all states look offline.

Redacted diagnostics include versions, capability flags, error codes, stage timings, process health, and migration state. Do not include transcripts, window titles, file contents, URLs with credentials, or raw provider bodies.

User-visible recovery:

- Provider unavailable: retain local text/partner mode and show exact affected capability.
- Worker crash: stop owned audio, clean files, bounded restart, then offer setup/repair.
- Integration disconnect: retain stored run identity, reconcile on reconnect, show unknown outcome when necessary.
- Key-store unavailable: offer recovery instructions; never create an empty replacement DB silently.
- Migration failure: preserve old DB and backup; block destructive switch until verified.
- Emergency stop: stop local capture/playback, block new dispatch, request external cancellation, list jobs not confirmed stopped.

## 6. Support and compatibility

Publish a compatibility matrix containing engine/version, OS/environment, transport, auth, read/write, progress, approval, cancel, reconcile, limitations, and evidence date. Empty means unqualified.

Keep a pinned adapter version range for OpenClaw, MCP protocol revisions and tested n8n/Zapier transport behavior. Re-run fake and live smoke tests before updating a supported range. A changed third-party schema disables affected capability revisions until requalified.

Support documentation should ask for the redacted diagnostics export, app version, adapter version, hardware profile, and reproduction steps. Never ask users to send API keys or private transcripts.

## 7. Public beta gate

All of the following are required:

- S0–S6 stage evidence and current capability matrix.
- Core voice/latency/WER/listener/soak targets reported honestly, with failed subgroups visible.
- Zero unauthorized actions in the declared adversarial suite.
- Clean-install, update, rollback, migration and repair evidence.
- Local mode works without account and integration mode is explicitly enabled.
- 20–30 participant PC-worker pilot completed with consent and retention controls.
- License/source review for every bundled/downloaded asset and adapter.
- A current README that does not call planned integrations or unqualified features shipped.

Public beta release is a product decision after this evidence review, not an automatic consequence of a passing CI build.
