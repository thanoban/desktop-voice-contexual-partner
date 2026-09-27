# Feature catalog and acceptance criteria

Priorities: P0 is required for a useful local partner, P1 is required for the public beta ecosystem, P2 is later expansion. Every feature has a privacy boundary and a measurable completion condition.

## P0 — partner core

| Feature | User experience | Acceptance |
|---|---|---|
| Companion modes | Keep me company, Help me work, Quiet focus with shared identity | Mode changes are immediate, persisted, and do not grant tools or export personal memory |
| Voice + text | PTT, optional hands-free after qualification, typed message, transcript correction | Text works without audio setup; only final/corrected text can trigger an action |
| Warm conversation | Natural concise replies, humour, listening, brainstorming | Blinded listener and PC-worker pilot thresholds in voice plan |
| Continuity | Sessions, project context, editable memories, "where were we?" | Provenance, project scope, correction, forget, and deletion tests pass |
| Quiet company | Opt-in check-ins, focus mode, quiet hours, break prompts | No check-in during suppression; no guilt/exclusivity language in evaluation |
| Interruptibility | Stop voice and barge-in during response | Local playback stop and interrupted context targets pass |
| Local profile | Ollama/Whisper/local speech with visible setup and no account | Offline smoke works after models are installed; no silent cloud fallback |
| Accessibility | Keyboard, focus return, labels, contrast, reduced motion, text fallback | Keyboard/screen-reader review has no blocker findings |
| Safety | Supportive distress response and resources without diagnosis | Test cases show appropriate acknowledgement and no false clinical claim |

## P1 — work partner and ecosystem

| Feature | User experience | Acceptance |
|---|---|---|
| Provider profiles | Private local, hybrid, cloud voice with data destinations | Route is shown before first disclosure; missing locality fails closed in private mode |
| Integrations panel | Connect, inspect, discover, grant, test, disable, disconnect | Read-only test, capability list, health, version and scopes are visible |
| MCP client | Use selected local/remote tools and workflows | Fake and one live server pass protocol, auth, schema, limits and failure tests |
| OpenClaw jobs | Delegate a scoped task, see progress/approvals, cancel/reconcile | Pinned-version task suite has no duplicate submission or false cancellation |
| n8n workflows | Run approved repeatable reports/drafts | Workflow revision, input scope, result, external ID and owner are recorded |
| Zapier MCP | Discover and run enabled cloud actions | Token never appears in UI/logs; cloud destinations and usage are visible |
| Activity history | One view of local/remote runs, approvals, artifacts and failures | Correlation IDs link every visible result to stored evidence |
| Project workspaces | Separate clients, sources, memories, tools and output paths | Cross-project retrieval/dispatch adversarial tests are zero-leak |
| Developer assistance | Selected error, code context, diff, scoped tests, explanation | No write runs without review/grant; returned tests/evidence are linked |
| Freelancer assistance | Proposal, client update, report and handover drafts | External sending remains a separate reviewable operation |
| Windows basics | Open/switch apps/files, window arrangement, saved launch routine | Target app/path and postcondition are verified; wrong focus stops |
| Local API/MCP server | Approved task submit/status/project search/notification | Loopback auth, grant scope and recursion tests pass; off by default |

## P2 — expansion

Meetings; richer browser automation; background standing workflows; source-controlled workflow packs; Sinhala/Tamil and more languages; encrypted multi-device sync; mobile companion; additional desktop operating systems; visual character; licensed voice customization; advanced local emotion/pause adaptation. Each needs a separate threat model, licensing review, quality corpus and release gate.

## Cross-feature quality rules

- A feature that sends data must name the destination, retention uncertainty, and user control.
- A feature that mutates the desktop must expose effect, target, approval/grant, progress and verification.
- A feature that runs remotely must preserve external identity and reconnect behavior.
- A feature that speaks must be cancellable and avoid reading structured artifacts in full.
- A feature that remembers must show provenance and provide correction/forget controls.
- A feature that schedules must have one owner and an explicit disable path.
