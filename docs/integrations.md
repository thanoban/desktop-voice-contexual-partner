# Automation ecosystem integration specification

VoicePartner owns the conversation and permission experience. Engines remain replaceable. This document defines proposed behavior; no listed connection is currently implemented or certified in this repository.

## 1. Integration order and role

| Route | First implementation | Why |
|---|---|---|
| MCP stdio | User-approved local executable and pinned args | Local specialist tools without per-app wrappers |
| MCP Streamable HTTP | Authenticated user-configured endpoint | Remote tools, n8n and Zapier |
| OpenClaw Gateway | Dedicated authenticated WebSocket/RPC adapter | Agent job/session lifecycle and progress |
| n8n | Curated workflow tools through MCP Server Trigger | Repeatable business/freelancer workflows |
| Zapier | Setup recipe over generic MCP transport | Existing cloud app connections |
| Webhooks | Fixed authenticated endpoint with declared schema/effects | Specific workflows not exposed through MCP |
| Native Windows/editor | Small scoped tool surface | Desktop functions and selected work context |

OpenClaw documents its Gateway for external apps starting jobs, consuming events, awaiting results, and cancellation. Its MCP bridge documents channel-backed conversation operations; do not assume it is a general substitute for Gateway job control. [Gateway integration](https://docs.openclaw.ai/gateway/external-apps), [MCP bridge](https://docs.openclaw.ai/cli/mcp/serve).

n8n's MCP Server Trigger exposes tool nodes and workflow tools via Streamable HTTP/SSE. Implement Streamable HTTP initially. Zapier documents support for compatible Streamable HTTP clients. [n8n](https://docs.n8n.io/integrations/builtin/core-nodes/n8n-nodes-langchain.mcptrigger.md), [Zapier](https://docs.zapier.com/mcp/get-started/quickstart).

## 2. Guided setup

1. Choose a connection recipe: generic MCP, OpenClaw, n8n, Zapier, or webhook.
2. Detect a user-specified existing service or accept an endpoint; no broad LAN scanning.
3. Show prerequisites and official installation guidance if missing. Installation is a distinct user-approved action; do not bundle an engine initially.
4. Authenticate through supported OAuth/device pairing or securely store a supplied token. Never import credentials by scraping another application's files.
5. Run a read-only handshake/health check. Show actual server/version and supported protocol.
6. Discover capabilities and classify effects. Start with all capabilities disabled until the user chooses a scope.
7. Show connection locality and downstream processing separately. Unknown egress stays unknown.
8. Bind selected capabilities to a personal/project scope, choose review behavior, then enable.
9. Run a harmless qualification example. No surprise email, file edit, charge, or publication during setup.

Connection cards show enabled state, identity/account, destination, tool count, granted scopes, health, tested version, last successful contact, usage provenance, and disconnect. Reconnection does not restore revoked grants automatically.

## 3. MCP host behavior

- Use a maintained SDK where compatible; pin dependencies and conformance-test supported protocol versions. Handle server negotiation rather than hardcoding one version everywhere.
- Support tools first; allow resource retrieval only under a context grant. Server prompts are user-selectable templates, not higher-priority instructions. Server-requested model sampling is disabled initially.
- Namespace capabilities by connection ID. Refresh paginated discovery and retain schema hashes. Schema/effect changes invalidate saved approvals or require requalification.
- Cache discovery outside the voice critical path. Search relevant tools and provide only the selected schemas to the model.
- Stdio servers are explicitly configured executable + argument arrays, launched without shell interpolation. Own the process tree, bounded I/O, timeout and shutdown.
- Remote calls use verified TLS except permitted loopback development. Disable cross-origin credential forwarding and unreviewed redirects. Reject userinfo embedded in URLs and redact token-bearing query parameters.
- HTTP sessions, JSON/SSE framing, timeouts, cancellation, and notifications follow the negotiated protocol. Transport closure is not proof that an action was undone.
- Validate arguments against the approved schema; enforce payload and output size limits. A remote `readOnlyHint` is never sufficient to grant read-only trust.
- Unknown capabilities cannot execute unattended. Add reviewed recipes for common tools rather than exposing every discovered action automatically.

MCP is an interoperability protocol, not a guarantee of sandboxing, authorization, durable jobs, or rollback. [Official architecture](https://modelcontextprotocol.io/docs/learn/architecture).

## 4. OpenClaw adapter

Use the documented external Gateway protocol, not private plugin imports. Connect as a properly paired/authenticated client with minimal permissions. Maintain a tested-version range; unsupported revisions are disabled with a clear upgrade/downgrade message.

Map one VoicePartner project and conversation to dedicated OpenClaw session references. Do not attach to unrelated user sessions or broadcast work into existing messaging channels. The task brief contains only objective, authorized project context, permitted outcomes, and constraints. Personal companionship history is excluded.

Persist a local request/correlation ID before submit. Map documented `agent`/`agent.wait` behavior and events into the execution contract; confirm exact request schemas against the selected release during the S3 compatibility spike. Handle pending/wait timeout without declaring task failure. Preserve cancellation/supersession reasons. [External-app lifecycle reference](https://docs.openclaw.ai/gateway/external-apps).

Display progress and downstream approvals only when exposed by the qualified adapter. Bind each approval to its upstream ID and exact operation. Never auto-approve a Gateway request just because a top-level job was approved. Require enforced downstream tool scopes for autonomous mutation; otherwise admit only constrained tools or draft/read jobs.

Avoid `/tools/invoke` as a generic escape hatch: documentation states it does not add a separate per-call approval boundary. Do not relax upstream deny lists to make a demo work. [Tool invocation policy](https://docs.openclaw.ai/gateway/tools-invoke-http-api).

On reconnect, reconcile stored external identities, restore observable event position, and fetch durable state if events were lost. Cancel requests remain pending until the engine confirms a terminal state. If an engine cannot guarantee stopping, say so while stopping local audio immediately.

Local Windows, WSL, and remote Gateway setups are separate compatibility entries. Resolve path mappings explicitly. A Windows app cannot assume a WSL path or localhost bridge points to the intended project. Existing external engines are not terminated merely because VoicePartner closes.

## 5. n8n and Zapier

### n8n

Expose a small reviewed workflow catalog, beginning with report preparation, task collection, and client-update drafts. Each workflow has a stable identity, revision, input schema, effect description, data destinations, and result format. VoicePartner calls the published production MCP endpoint; test endpoints are only for setup/development.

Use bearer/header auth supported by the chosen endpoint. Keep management capabilities such as workflow creation, credential changes, publishing, and deletion disabled in the first connector. Existing workflow execution and platform administration are different grants.

For long work, a wrapper returns a durable execution ID and supports status inspection through an explicitly enabled API/tool. If the endpoint cannot do that, label it as a bounded synchronous tool; do not claim durable progress/cancellation. Authenticated webhooks use fixed approved URLs, replay protection, and declared effects. Never follow arbitrary callback URLs supplied by model output.

Workflow revision changes invalidate the standing grant. External schedules are owned by n8n and stored locally by external ID; never duplicate them in VoicePartner's timer loop.

### Zapier

Use the generic remote MCP client with a curated recipe. Store the connection token as a credential reference, not a visible URL or project setting. Discovery must not auto-enable mutations. Respect user account/app selection and show cloud data flow.

Record reported task usage where available. Provider task fees and model fees are separate. Unknown provider billing is labeled unknown; do not invent a combined hard cap.

## 6. Routing and work continuity

Precedence: user-named tool/workflow, saved routine, bounded direct tool, then explicitly enabled delegated agent. Chat without an execution request stays in VoicePartner. Ambiguous destinations with materially different accounts/effects require a user choice.

Use one executor per action. Do not nest agents by default or run competing agents on the same desktop. Limit delegation ancestry and reject cycles. Scheduled work uses the same policy and ledger as interactive work.

Background completion produces a card immediately and optional speech only when permitted by focus/quiet state. A failed job provides the last confirmed effect and recovery options, not an automatic resubmission that could duplicate work.

## 7. Adapter SDK and qualification

Ship a minimal example adapter, fake clock/transport helpers, versioned fixtures, and a checklist. Adapters declare supported lifecycle features honestly, connection/effect defaults, credential needs, size/time limits, and compatibility range. An adapter may be read-only or synchronous; that is a valid capability, not an implementation failure.

Required tests: handshake/auth failure, token expiry, schema change, malformed/oversized frames, pagination, server prompts containing malicious instructions, disabled tools, ambiguous dispatch, event gaps, cancellation races, reconnect, downstream approval binding, project separation, and duplicate prevention.

Compatibility records contain engine/version, operating environment, transport, auth method, read/write/approval/progress/cancel/reconcile support, evidence date, and limitations. Empty matrix means unqualified. Passing fake-server tests alone never certifies a live integration.
