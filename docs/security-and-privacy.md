# Permissions, privacy and recovery requirements

This is a product engineering specification for the approved automation scope. It describes planned controls; current code does not yet implement all of them. Apply the same policy service to Tauri, local API, MCP server, scheduled work, and external-agent delegation.

## 1. Authority model

Only explicit user intent and stored user grants authorize actions. Web pages, files, screenshots, tool descriptions/results, model output, and external-agent messages are data. None can add tools, broaden filesystem access, approve an action, change a provider, or disable policy.

Each grant includes subject/client, project, capability, effect, target scope, allowed data destinations, lifetime, and revocation state. One-use approval binds to exact normalized arguments and capability/workflow revision. Changing an argument or target invalidates it.

| Effect | Default handling |
|---|---|
| Read approved project/source | Allowed within explicit scope |
| Open/switch an approved app or file | Routine reversible scope may cover it |
| Create/edit a file | Review a diff or apply a previously approved narrow routine |
| Run shell/code | Explicit executable/working-directory/effect scope; never treat unrestricted shell as read-only |
| Send email/message, publish, buy, delete, elevate | Exact consequential review unless an explicit narrowly bounded saved workflow already authorizes it |
| Install plugin/tool, change credentials or permissions | Separate setup/administration action |
| Unknown/opaque external effect | Review-required; autonomous mutation disabled until classified and enforced |

Avoid repetitive approval for work already covered by a valid grant. Show concrete recipient, path, command, content, diff, or destination before review. A vague task label cannot authorize all downstream effects.

For delegated agents, require enforceable upstream restrictions and/or verified approval hooks. If unavailable, expose read/draft tools or a bounded workflow. Telling an unrestricted agent to ask permission is not a security boundary.

## 2. Data disclosure and private mode

Separate personal conversations from client projects. External task briefs receive only selected project context and relevant user-approved preferences. No whole-history forwarding by default. Personal memories are excluded from the public local API/MCP surface unless explicitly granted.

Connection locality, model locality, and downstream destinations are stored separately. Private-local mode requires qualified local processing and enforceable egress restrictions for connected engines. Unknown routes are unavailable in private mode. Switching to hybrid requires visible user choice, never an automatic fallback.

External platforms can retain their own task/session copies. The connection UI must state that local deletion cannot guarantee remote deletion. Where an adapter supports remote deletion, show it as a distinct action and verify its result. Do not claim remote erasure without provider confirmation.

Context grants are checked before capture, before storage/retrieval, and immediately before transmission. Revocation cancels queued disclosures. Stopping sharing clears the manual context and disables auto-capture; old UI state is not authority.

## 3. Credentials and storage

- Keys/tokens and the database key live in the Windows credential store; SQLite holds references only.
- Verify the chosen keyring backend actually uses the platform store; adding a crate is insufficient.
- Use SQLCipher for retained sensitive DB content, including indexes, after Windows compatibility tests. Local encryption is not protection from a compromised logged-in OS session.
- Back up before conversion, write a new encrypted database, validate integrity and record counts, and atomically switch. A failed conversion keeps the original usable.
- Handle key-store locked/unavailable states with recovery UI; never silently create a new empty database over existing data.
- Backups and support exports have explicit privacy handling. Plaintext legacy backups remain labeled and require a retention choice.
- Credentials never appear in prompts, browser storage, source control, URLs shown to users, process arguments where avoidable, logs, or telemetry.
- Remote TLS certificate verification is on. Do not forward auth across redirects; endpoint validation rejects embedded credentials and unexpected schemes.

## 4. Retention and deletion

| Data | Default |
|---|---|
| Microphone PCM and screenshots | In-memory/transient; no archive |
| Compatibility temporary audio | Restricted app temp path, remove after use and on startup recovery |
| Conversation text | Local history enabled, user can disable, delete session, or wipe history |
| Personal memory | Visible and editable; disable future retention independently of history |
| Project documents | Only selected sources; user controls removal and project deletion |
| Execution history | Bounded summaries/evidence; default 90 days, configurable |
| Redacted diagnostic logs | Rotate, default seven days |
| Remote copies | Provider-specific; disclosed, never silently called deleted |

Forget-memory, delete-conversation, remove-document, and delete-project are separate clear actions. Offer a combined deletion flow when requested. Derived summaries, embeddings, indexes, queued memory jobs, and caches must honor deletion. A background task must not recreate content after deletion; use tombstone/revision checks.

File/source revocation immediately prevents future reads and invalidates cached context. Document replacement swaps active revision only after successful ingestion. Explain that filesystem secure erasure is not guaranteed by ordinary deletion; do not claim more than the implemented storage behavior.

## 5. Desktop and executable boundaries

- Restore a restrictive Tauri CSP. Separate main and widget capabilities; commands still validate backend scope.
- Keep raw shell/FS authority out of the WebView. Validate paths canonically, including Windows junctions/symlinks, UNC paths, and traversal before access.
- Do not use model-generated strings as shell commands for routine native tools. Prefer native APIs and explicit executable/argument arrays.
- UI automation verifies the target application/control and relevant postcondition. A stale coordinate or changed focus must halt or re-observe.
- Screen capture is explicit per window/region/session. Password fields and sensitive app areas are excluded where detectable; detection limitations must not be presented as perfect redaction.
- Native workers/stdio servers are owned processes with hidden windows, bounded outputs, timeouts, and shutdown cleanup. User-owned external services are not killed on app exit.
- Model/tool downloads require hashes, bounded archive extraction, atomic installation, and recorded source/license/version. No unverified package execution from discovered descriptions.

## 6. Local API and recursion

Loopback binding alone is not authentication. Use per-client credentials, narrow grants, origin/host checks where relevant, request-size limits, rate limits, and replay protection for mutating requests. Default off; remote network access is a separate future design.

Caller cannot approve its own elevated operation. Incoming task ancestry is preserved. Reject self-delegation cycles and impose a maximum depth. Tools cannot transform untrusted external input into privileged user instructions.

## 7. Failure behavior

Local audio stop is immediate; remote cancellation is separately acknowledged. A disconnect may leave a remote job running. Show surviving/unknown jobs after disconnect and startup; reconcile before retrying. Never automatically repeat a possibly completed send, purchase, file write, or schedule creation.

An emergency stop blocks new dispatch, requests cancellation for active runs, and stops local capture/playback. It cannot reverse completed external actions and must say which jobs could not be confirmed stopped.

Private mode, grants, and cost restrictions fail closed when required facts are missing. Local companionship and text input remain available when optional integrations fail.

## 8. Required adversarial verification

Test injected instructions in documents, web pages, screenshots, MCP descriptions/results, and external-agent status. Include changed tool schemas, expired approvals, account switching, wrong-project paths, Unicode/path tricks, malicious redirect targets, OAuth expiry, secret-bearing error bodies, duplicate callbacks, and late worker results.

Release gate: zero unauthorized actions in the declared test suite, with scope and limitations recorded. This is a test result, not a universal security guarantee. See [integration qualification](integrations.md) and [release operations](release-and-operations.md).
