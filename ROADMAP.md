# VoicePartner — implementation roadmap

Status date: 2026-09-28. This roadmap is an implementation ledger, not a calendar promise. A stage is complete only when its exit evidence exists. Planned support is not shipped support.

## Current baseline

The plan was published at `bc734ce` on `main`, with earlier local documents preserved at `docs/archive/`. S0 is now in progress: shared Rust/TypeScript contracts, canonical fixtures, timing primitives, Windows CI, and a local verification script are implemented. Nine Rust tests pass, including session lifecycle, run-state, policy-default, timing, and fixture tests. Existing local voice, memory, RAG, and widget features still require runtime qualification.

### Current implementation evidence

| Work | Status | Evidence |
|---|---|---|
| End-to-end plan and architecture | implemented | Planning commit `bc734ce` |
| Contract foundations | unit-tested | Rust domain types, TypeScript mirrors, JSON schemas and fixtures |
| Session creation semantics | unit-tested | A new session transaction closes prior open sessions; legacy reuse remains tested |
| Verification pipeline | implemented | `scripts/verify.ps1` and `.github/workflows/verify.yml` |
| Baseline automated suite | unit-tested | 15 passed, 0 failed on 2026-09-28 |
| Recording completion and input formats | unit-tested | Explicit worker completion replaces fixed delay; f32/i16/u16 capture and bounded buffers compile; resampling tests pass |
| Ollama stream framing | unit-tested | Split JSON, split UTF-8, multi-frame chunks, malformed frames and final frames are covered |
| Typed text fallback | implemented | Keyboard-accessible composer supports Enter to send and Shift+Enter for multiline drafts |
| Live voice, provider, integration and installed-device behavior | planned | Requires the later stage gates below |

## Stage gates

| Stage | Outcome | Main work | Exit evidence |
|---|---|---|---|
| S0 | A reproducible, inspectable foundation | Documentation, contracts, CI, baseline metrics, license/model inventory, fake transports | Clean checkout build, contract fixtures, dependency report, baseline latency/resource report |
| S1 | A reliable local partner | Session controller, stream parser, audio completion, cancellation, shared widget state, text input, memory scope | Unit/integration tests; no duplicate/stuck turns; manual local conversation and interruption record |
| S2 | Natural local voice | Persistent recognition/synthesis workers, VAD, phrase streaming, native playback, pronunciation, companion modes/check-ins | Reference-PC voice report, WER corpus, listener study, eight-hour soak |
| S3 | Explicit provider and MCP foundation | Provider capabilities, local/hybrid/cloud routing, credential store, generic MCP stdio/HTTP, discovery, grants, activity UI | Fake MCP conformance, auth/expiry tests, one qualified local and remote server, data-route review |
| S4 | OpenClaw delegated jobs | Versioned Gateway adapter, session mapping, progress, approvals, cancel/reconcile, artifacts | Pinned OpenClaw compatibility report and scoped task suite with duplicate/unknown-outcome tests |
| S5 | Workflow ecosystem | n8n MCP recipes, Zapier recipe, webhooks, routines, schedules, cost/usage, unified activity | Representative freelancer workflows, provider failure recovery, single schedule ownership proof |
| S6 | Context and Windows depth | Project workspaces, source-grounded retrieval, VS Code/browser context, small native Windows tools, local API/MCP server | Retrieval and permission suite, clean local API security test, verified desktop postconditions |
| S7 | Public beta | Signed installer/update/rollback, guided repair, diagnostics, adapter docs, pilot | Clean-machine install/upgrade, eight-hour soak, 20–30 person pilot, release review |
| S8 | Expansion | Meetings, more workflow packs, Sinhala/Tamil, broader languages, mobile, encrypted sync, additional OS | Independent gate for each addition; no regression of S1–S7 |

Stages may run parallel only after their contract dependencies are complete. S2 voice work and S3 integration work must not make local companionship unavailable.

## S0 — foundation and truth

1. Replace conflicting product/architecture assumptions with the current documents and preserve prior notes in the archive.
2. Add canonical schemas and fixtures for sessions, turns, events, provider capabilities, grants, capabilities, runs, approvals, results, projects, and usage.
3. Add `scripts/verify.ps1` and CI for formatting, clippy/check, Rust tests, TypeScript build, contract validation, and documentation link checks.
4. Inventory current downloads and dependencies. Record exact source URLs, versions, hashes, licenses, size, runtime prerequisites, and supported platforms. Add atomic download/checksum/archive extraction behavior.
5. Capture baseline hardware, cold/warm state, stage timings, memory, GPU use, current WER sample, and known failures. Never call the baseline "production-ready."
6. Decide SQLCipher/vector extension feasibility on Windows with a small compatibility spike before migrating user databases. Back up existing v6 DBs in test fixtures.

## S1 — reliable personal partner

1. Introduce backend session controller and make main/widget frontends read the same snapshot/events. Remove duplicate event subscriptions that can append tokens twice.
2. Make `start_new_session` close/create explicitly. Preserve old sessions and show the active project/session.
3. Fix streaming decoder for split byte/UTF-8/JSON frames and provider error/terminal events. Add request IDs and stale-event rejection.
4. Replace the fixed WAV flush delay with a recording completion channel and cleanup guard. Correct supported CPAL sample-format handling and bound capture buffers.
5. Implement real cancellation: generation, synthesis queue, playback, and late-result suppression. `stop_speaking` must not report success until local playback is stopped; return distinct remote cancellation states.
6. Add typed text composer and transcript correction. Keep voice-first UI, but do not block work on missing microphone/model setup.
7. Scope memory/document retrieval by project and source provenance. Fix the 200-entry arbitrary search limit through indexed/relevance-bounded retrieval while vector migration is prepared.

## S2 — natural local voice

1. Package/qualify persistent Whisper and Kokoro workers with readiness, request framing, model hash, bounded channels, restart, shutdown, and crash cleanup.
2. Add VAD and endpointing with PTT fallback, initial-speech buffer, user-visible listening state, device unplug/recovery, and noise/non-speech tests.
3. Add phrase segmenter and native playback queue; start speech before full generation completes. Exclude code blocks, tool JSON, hidden reasoning, and long URLs from spoken output.
4. Add echo/noise qualification, barge-in, consumed-output offsets, and interrupted-turn context.
5. Add partner modes, quiet hours, opt-in check-ins, voice audition, pronunciation dictionary, pacing, reduced-motion/accessibility behavior, and silence mode.
6. Run the declared latency, WER, listener, and soak evaluations. Only then select release defaults.

## S3 — provider and MCP foundation

1. Move Ollama transport behind provider capability interface; add explicit paid text adapters without requiring Ollama. Store provider keys via Windows credential store references.
2. Add local/hybrid/cloud profile UI with visible data destinations, usage and budget fields, and fail-closed routing.
3. Implement generic MCP stdio and Streamable HTTP client with negotiated protocol, discovery cache, schema hash, pagination, auth, output limits, process ownership, and transport tests.
4. Add capabilities, grants, effect classifications, reviews, approval expiry, and activity ledger. Remote annotations are hints only.
5. Add an authenticated loopback API only after local policy service exists. Keep it disabled by default and prevent recursive delegation.

## S4 — OpenClaw

1. Pin a tested OpenClaw Gateway version and record the exact RPC/event subset required. Do not import private plugin SDK paths.
2. Build guided local/WSL/remote connection setup with pairing/token-file guidance, project/session mapping, health, version, and downstream locality disclosure.
3. Implement ledger-first agent submission, `agent.wait`/event mapping, pending timeouts, approval binding, cancellation states, reconnect/reconcile, and artifact references.
4. Qualify one read-only and one reversible scoped workflow before mutation. Keep generic tool-invoke routes disabled unless their permission boundary is verified.

## S5 — n8n, Zapier and routines

1. Add n8n production MCP recipe, stable workflow ID/revision, input/output schemas, workflow execution/status behavior, and management-action deny defaults.
2. Add Zapier Streamable HTTP recipe, token reference handling, discovery/enable flow, app scope display, and usage provenance.
3. Add fixed authenticated webhook adapter with replay protection and declared effects. Never follow model-provided callback URLs.
4. Add saved routines and schedules with one owner, reviewed revision, timezone, external schedule ID, disable/reconcile behavior, and quiet-hours notification rules.
5. Test duplicate submission, connection loss, provider limits, unknown billing, external approval and recovery.

## S6 — project context and Windows depth

1. Add projects/client boundaries, stable document identity/content hash/revisions, FTS5 plus qualified vector retrieval, citations, deletion and rebuild.
2. Add selected editor/VS Code context and browser context through maintained integrations; capture only granted selections/windows.
3. Add native Windows actions using semantic application APIs/UI Automation where possible: open/switch, approved files, selected text, window arrangement, saved launch routine, emergency stop.
4. Verify postconditions and stop on focus/layout changes. Do not treat coordinates or screenshots as sufficient evidence for consequential changes.
5. Expose scoped local API/MCP functions for task submission/status, approved project search, and notifications. Personal memory is off by default.

## S7 — public beta and operations

1. Restore restrictive CSP and least-privilege capabilities. Add signed installer/update config, rollback, clean uninstall, model/worker repair, and migration backup/restore.
2. Add redacted diagnostics, version/compatibility matrix, support runbook, crash/worker cleanup, provider outage messaging, and usage/cost reporting.
3. Run static tests, fake adapters, installed Windows E2E, live qualification, security adversarial suite, performance/soak, accessibility review, and pilot.
4. Release integrations disabled until setup/qualification. Publish current capability list and limitations. Keep local partner functional with no accounts.

## S8 — expansion gates

Meeting capture requires explicit start/stop, participant/recording disclosure, retention/deletion, and transcript quality tests. Sinhala/Tamil requires separate speech/LLM/TTS corpora and listener review. Mobile/sync requires a new identity/encryption/threat model. Custom voices require consent and redistribution review. New automation engines require the adapter checklist and an actual compatibility record.

## Work-item template

```text
ID / user outcome:
Owned paths:
Depends on:
Contract/schema impact:
Data migration/retention impact:
Permission and threat model:
Automated tests:
Live/device qualification:
Rollback/recovery:
Evidence links and status:
```

## Completion vocabulary

Use `planned`, `implemented`, `unit-tested`, `integration-tested`, `live-qualified`, `installed-verified`, `pilot-observed`, or `released`. Do not use "done" for a plan, successful compile, or pushed commit alone.
