# VoicePartner development handoff

Updated: **2026-11-21, Asia/Colombo**.

## Start here

Read [the full system review](docs/system-review-2026-11-21.md), then [ROADMAP](ROADMAP.md). The review records current capabilities, twelve prioritized findings, acceptance gates and the remaining end-to-end work. The approved product direction is unchanged: a warm personal voice partner for PC workers with optional automation services.

## Current source and publication

The reviewed implementation baseline is `61e483f` on `main`. Recent committed work includes shared conversation/audio recovery, deletion guards, schema v9 hybrid retrieval/document revisions/retention, and Ollama/OpenAI-compatible/Anthropic text providers with Windows credential references. The earlier October handoff was stale and has been replaced with this current entry point.

At review start, main was seven commits ahead of `origin/main`; a fresh fetch confirmed this. Only `.claude/` and the older handoff were untracked. This review adds documentation and corrects current-status descriptions. Publication is authorized by the user's 2026-11-21 instruction; verify the actual push result in Git before claiming publication. Existing local agent worktrees/settings under `.claude/` are excluded from staging and remain untouched.

## Verification

The full repository gate was attempted against the current implementation on 2026-11-21 and exited 1 during Rust test compilation/linking because D: ran out of space. October's 57-test result is historical, not current evidence.

- Contract script: five JSON files parsed with basic fixture invariants; not complete JSON Schema conformance.
- Frontend: TypeScript/Vite production build passed; eight Node regression tests passed.
- Rust: formatting and strict Clippy passed. Rust tests failed to compile/link with `no space on device` / `LNK1180`, so no current execution result exists; the subsequent native application build was not reached. D: had approximately 1.8 MB free afterward. Restore disk headroom before rerunning.
- `npm audit --json`: eight affected packages (five high, two moderate, one low). Review R11 for packages and remediation scope. No automatic dependency changes were applied.
- `npm audit --omit=dev --json`: zero affected production dependencies; development/build findings still require repair. Rust advisories were not audited.
- Local links in seven review/status documents resolve and `git diff --check` passed. Execution policy blocked removal of generated incremental cache; it was left untouched.
- Toolchain: Node v22.19.0, npm 10.9.3, rustc/cargo 1.93.1, Windows MSVC.
- No development server, external automation engine, microphone capture, real-model session, paid account call, installer or eight-hour soak was started by this review.

Run the gate when code changes or diagnosis justify it:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1
```

Do not run parallel Cargo builds against the same target/cache. Passing automated checks does not establish private-local execution, live-provider support, audio quality or release readiness.

## Next development slice

1. Restore disk headroom, then close review R01/R02 first: authorize every inference stage before transmission, constrain auxiliary Ollama endpoints and disable/revalidate redirects. Fake destination servers must prove disallowed requests receive no private payload.
2. Bind provider destinations at admission; carry cancellation through preparation; make main/widget readiness independent of Ollama when cloud text is selected (R03–R05).
3. Bind context mutations to the expected project and disclosure revision; bound/redact inference errors (R06–R07).
4. Qualify real main/widget/audio behavior and process cleanup, then progress persistent workers/VAD/phrase speech and the MCP permission/ledger foundation in separate owned paths.
5. Complete release hardening and dependency repairs before public beta; record measured quality and pilot evidence before superiority or loneliness-reduction claims.

See the system review for all later work: MCP/OpenClaw/n8n/Zapier, routines/schedules, Windows/editor/browser tools, local API, encryption/download integrity/signing/recovery, benchmarking and expansion. The complete project remains unfinished.

## Preserve

Preserve project/client isolation, monotonic snapshots, turn/audio leases, cancellation epochs, cancellable bounded Whisper execution, source revision guards, atomic document activation, model/dimension provenance, FTS5 index cleanup and secret-only credential storage. Do not reset local worktrees or publish private settings. Schema v9 is implemented; no runtime MCP/policy subsystem is merged into main merely because it exists in a separate experimental checkout.
