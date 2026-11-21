# VoicePartner system review and remaining work

Reviewed: **2026-11-21 (Asia/Colombo)**. Source baseline: `61e483f` on `main`.

## Verdict and scope

The project has a substantial working foundation, but is **not ready for public beta**. Provider routing and indexed lexical retrieval are now implemented, so earlier documents understated progress. Privacy enforcement across auxiliary inference requests, provider-neutral UI readiness, release security, and real-device qualification remain blockers.

This review covers the tracked main checkout: desktop/frontend, conversation/audio ownership, provider transports, memory/retrieval/retention, setup, contracts, CI, and release configuration. Three independent read-only agents reviewed voice/providers, retrieval/frontend, and security/release. Nested `.claude/worktrees` are separate checkouts, not delivered main-branch integrations. No private account, microphone recording, real model, external automation service, or installed release was exercised. Findings below are source-review conclusions unless a command result is explicitly recorded.

## Current capabilities

| Area | Present in main | Still required |
|---|---|---|
| Partner interface | Company/Work/Focus modes, text composer, transcript, history, project switching, tray/widget | Opt-in considerate check-ins, quiet hours, transcript correction, accessibility qualification |
| Conversation reliability | Exclusive turn admission, revisioned project/session snapshots, hydration, cancellation and failed partials | Real two-WebView/device/restart qualification |
| Voice | CPAL bounded capture, cancellable Whisper child, Piper/SAPI/Kokoro paths, owner cleanup and playback epochs | Persistent workers, VAD, phrase streaming, native playback, echo handling and measured interruption |
| Text providers | Ollama, OpenAI-compatible and Anthropic adapters; SSE/NDJSON parsing; selected-route checks; settings UI | Entire-pipeline privacy boundaries, provider-neutral readiness, paid-service qualification and usage/budgets |
| Credentials | Windows Credential Manager backend, destination-bound references, secret-safe views | Real account lifecycle/expiry qualification and destination-change review |
| Retrieval | Schema v9, FTS5/BM25 lexical candidates fused with cosine candidates; model/dimension filtering, citations | Vector scalability qualification; ANN if justified; authoritative model revision identity |
| Documents | Stable database IDs within project/source-name identity, content hashes, staged revisions and active revision publication | Same-name source disambiguation, migration/recovery and large-import qualification |
| Retention | Startup/manual purge, separate memory/history limits, scoped history deletion, pending-write invalidation | Continuous long-session expiry policy, encrypted migration/restore and deletion semantics qualification |
| Automation | Domain contracts and architectural plan | Runtime MCP client, grants/approvals, execution ledger, OpenClaw, n8n/Zapier, Windows actions, local API |

FTS5 is indexed lexical retrieval. Vector ranking remains an exact paged scan with bounded candidates and short database lock acquisitions; it is not an ANN index. Provider code and Windows keyring selection are implemented; this does not establish live-provider or installed-device support.

## Findings and repair gates

| ID / priority | Evidence and impact | Required repair and acceptance evidence |
|---|---|---|
| R01 / High | `src-tauri/src/commands/chat.rs:219` embeds user text at the raw saved Ollama endpoint before generation route validation. `embed/mod.rs` and `summarize/mod.rs` construct their own clients. A Local profile with a remote saved endpoint can disclose text before the selected chat route is rejected. Document imports use the embedding path too. | Resolve and authorize every stage before any request; validate auxiliary destinations under the chosen profile. Fake servers must prove zero disallowed requests for chat recall, imports, summaries and summary embeddings. Do not describe the current profile as proof of complete private-local execution. |
| R02 / High | `llm/client.rs::build_client`, `embed/mod.rs` and `summarize/mod.rs` leave default redirects enabled. Ollama uses that client. A loopback server can redirect a private POST to a different destination. Paid adapters already use a separate redirect-disabled client. | Disable redirects or reauthorize every hop across all inference transports. Test 307/308 redirects and prove the second server receives no payload. Verify downstream model locality separately from loopback address. |
| R03 / High | `src/App.tsx` disables Composer/VoiceButton and schedules greeting based on Ollama connectivity. `src/Widget.tsx` also uses Ollama status. Cloud text selection therefore does not provide an independent usable frontend when Ollama is absent. | Add selected-provider readiness independent of optional embedding/summary readiness. With Ollama stopped, configured cloud text must send successfully; unavailable optional stages must degrade explicitly. Test main and widget. |
| R04 / Medium | `llm/router.rs::stream_chat` reloads provider settings after chat has prepared context. Configuration may change the destination of an already admitted turn. | Bind the resolved route and disclosure decision at turn admission; changes apply to future turns or require renewed disclosure. Test settings changes while context preparation is paused. |
| R05 / Medium | Initial `embed_text(...).await` in `commands/chat.rs` has a 30-second request timeout but no turn cancellation. Stop does not interrupt this preparation promptly. | Pass cancellation through auxiliary inference and preparation. Test stop during stalled embedding, no later generation, and promptly released turn ownership. |
| R06 / Medium | `commands/context.rs::start_sharing_context` and `set_context_note` lack an expected-project argument. A delayed old-view request can apply its note after a workspace switch. Revocation also cannot retract content already sent. | Bind context mutations and queued disclosure to project/revision grants. Reject stale-view writes; test project switches and revocation before dispatch. Clearly state that already transmitted data cannot be recalled. |
| R07 / Medium | `llm/ollama.rs` and `embed/mod.rs` read and return raw error bodies without a size/redaction boundary. A server can return excessive data or echo private inputs into diagnostics/UI. | Bound response/error bodies and redact messages. Test oversized and prompt-echoing failures; inspect logs and UI. |
| R08 / High release gate | `src-tauri/tauri.conf.json` has `csp: null`; capabilities are configured for `main` only although `lib.rs` creates `widget`. Widget permission behavior has not been installed-qualified. | Define restrictive CSP and per-window least privilege; explicitly audit the widget's required event/window APIs. Verify permitted and rejected IPC in both installed windows. Configuration alone is not evidence of an exploit. |
| R09 / High release gate | `lib.rs` opens ordinary SQLite; database encryption is absent. `commands/setup.rs::fetch_with_progress` writes directly to destination without expected hashes, size/deadline policy or atomic publication. Setup can mistake partial files for installed assets. | Qualify encrypted storage, backup/restore and key loss; pin asset versions/hashes/licenses, bounded downloads, staging and atomic activation. Test interruption, bad hash, corrupt archive, retry and retained working assets. |
| R10 / Medium | Installer `bundle.longDescription` still promises “no cloud” and all-local AI despite configurable paid providers. Provider UI also describes auxiliary Ollama stages as local although the saved endpoint can be remote. | Align packaging/onboarding/profile disclosures with actual stage destinations and R01/R02 enforcement. Audit all user-facing privacy claims. |
| R11 / High dependency maintenance | `npm audit --json` reports **8 affected packages: 5 high, 2 moderate, 1 low** on this review date. Packages: Vite, esbuild, PostCSS, nanoid, source-map-js, browserslist, baseline-browser-mapping and Babel core. | Upgrade through reviewed compatible versions, regenerate lockfile and rerun build/regressions/Tauri dev and packaged checks. Separate development/build exposure from shipped runtime exposure. Do not apply a forced major upgrade blindly. Rust dependency advisory/license scanning remains unperformed. |
| R12 / Medium verification gap | `scripts/verify-contracts.ps1` parses five JSON files and checks a few fixture fields; it does not validate all fixtures against JSON Schema. Current verification lacks installed E2E, dependency advisory gates and documentation link checking. | Add real schema validation and malformed fixtures, advisory/license checks and installed permission/voice recovery tests. Preserve the existing automated gate. |

Priorities express repair order, not a claim that every issue has a demonstrated exploit. R01/R02 follow directly from request order and client configuration; a live disclosure reproduction was intentionally not performed.

## Verification record

The 2026-11-21 gate **did not complete**. Contract syntax/basic-invariant checks, TypeScript/Vite production build, eight frontend regressions, Rust formatting and strict Clippy passed. Rust test compilation/linking then failed with `no space on device` / `LNK1180`; no current Rust test execution result was obtained, and the subsequent native application build was not reached. D: had approximately 1.8 MB free after failure. Restore disk headroom and rerun the gate before claiming backend qualification. Earlier 57-Rust-test evidence from October is historical and must not be used as qualification for the newer provider/retrieval commits.

Commands for this review:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1
npm audit --json
git fetch origin
git diff --check
```

The full dependency audit returned the findings above; `npm audit --omit=dev --json` reported zero affected production dependencies on this date. The eight findings are in the development/build dependency graph, which still needs repair and exposure review. This is not a Rust advisory audit. Local Markdown links in all seven review/status documents resolve, and `git diff --check` passed. No dev server or external engine was started for this review. Passing portions of the gate do not clear the findings or demonstrate latency/quality targets. A policy block prevented removal of generated incremental cache; caches, private settings and nested worktrees were left untouched.

## Remaining delivery sequence

1. **Close privacy and usability blockers:** R01–R07 and misleading disclosures. Introduce a single admitted per-stage route contract; cover it with fake HTTP tests and provider-neutral frontend tests.
2. **Qualify the reliable companion:** main/widget convergence during capture/STT/speech, rapid stop, microphone unplug, sleep/resume, project isolation, model failures and child-process cleanup on exit. Record hardware, model revisions and actual outcomes.
3. **Improve voice and considerate companionship:** persistent inference workers, VAD/endpointing, phrase-level speech, native playback/consumed offsets, echo/noise/barge-in, voice audition/pronunciation/pacing, silent mode, opt-in check-ins and quiet hours.
4. **Build automation policy before executors:** capability discovery, effect classification, scoped grants, bound approvals/expiry, durable ledger, cancellation/unknown outcomes, usage/budgets and recursion protection.
5. **Implement and qualify ecosystem routes:** generic MCP stdio and Streamable HTTP with auth/discovery/schema changes; then pinned OpenClaw Gateway with reconcile/cancel; n8n/Zapier recipes, webhooks, saved routines and single-owner schedules. Separate worktrees containing experiments do not satisfy this gate.
6. **Expand context and native usefulness:** editor/browser selection, scoped Windows launch/switch/files/selected-text actions, observed postconditions, emergency stop; authenticated optional local API/MCP server only behind policy.
7. **Harden distribution:** R08–R12, restrictive capabilities, encryption/recovery, download inventory, diagnostics, signed installer/update/rollback, clean install/upgrade/uninstall and adapter compatibility matrix.
8. **Prove the product:** reference-PC cold/warm median/p95 latency, playback stop, accent/noise WER, blinded voice comfort, grounded correctness, workflow success/cost, eight-hour resource soak and 20–30-worker pilot. Loneliness reduction and superiority remain hypotheses until evaluated.
9. **Qualify expansion independently:** meetings, Sinhala/Tamil, mobile, encrypted sync and additional operating systems.

Use [ROADMAP](../ROADMAP.md) for stage ownership, [PLAN](../PLAN.md) for product intent and [DEVELOPMENT_HANDOFF](../DEVELOPMENT_HANDOFF.md) for the next starting point. Keep conversation usable with every integration disconnected.
