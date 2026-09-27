# VoicePartner decision register

These decisions are current as of 2026-09-27. Changes require updating this file and the affected specialist document. Upstream links describe external protocol/product behavior at the time of planning; compatibility must be rechecked during implementation.

| ID | Decision | Reason and consequence |
|---|---|---|
| D-001 | Product is a personal partner for PC workers | Companionship, continuity, attention etiquette and work usefulness are co-equal; avoid task-only framing |
| D-002 | Voice-first with text | Voice stays central while text supports correction, accessibility and structured artifacts |
| D-003 | Local-first plus explicit paid/cloud profiles | Users need privacy and quality choices; no silent cloud fallback |
| D-004 | Keep Tauri/Rust/React | Existing investment and native audio/desktop boundary; migrate behind services rather than rewrite |
| D-005 | Integrate rather than bundle automation engines | OpenClaw/n8n/Zapier evolve separately; guided setup lowers installer/support scope |
| D-006 | Generic MCP is an adapter surface, not the policy boundary | MCP standardizes interoperability; VoicePartner must enforce grants, effects, approvals and provenance |
| D-007 | OpenClaw Gateway is the primary delegated-job route | Its external-app docs cover sessions, task lifecycle, events and cancellation; its MCP bridge is not assumed to be a generic job API |
| D-008 | One execution ledger and one executor per operation | Prevent duplicate mutations and unify progress, approvals, reconciliation and evidence |
| D-009 | Private/local mode fails closed on unknown egress | Loopback location does not prove downstream locality |
| D-010 | Persistent workers and phrase streaming are quality priorities | Per-request process startup and full-reply TTS are the main current perceived-latency problems |
| D-011 | Native playback/cancellation is required | Stopping a UI flag or PowerShell process is not reliable interruption evidence |
| D-012 | SQLCipher/vector migration is gated by Windows spike | Planned dependencies are not assumed compatible with current bundled SQLite until tested |
| D-013 | Personal memory is separated from project context | Client isolation and disclosure minimization are essential for freelancers |
| D-014 | No hosted public API in initial release | Avoid account/billing/control-plane scope until local core and adapters are qualified |
| D-015 | Evidence levels stay separate | Build/test/live/device/pilot proof answer different readiness questions |

## External references used

- [OpenClaw external apps and Gateway](https://docs.openclaw.ai/gateway/external-apps)
- [OpenClaw as MCP server](https://docs.openclaw.ai/cli/mcp/serve)
- [OpenClaw tool invocation policy](https://docs.openclaw.ai/gateway/tools-invoke-http-api)
- [n8n MCP Server Trigger](https://docs.n8n.io/integrations/builtin/core-nodes/n8n-nodes-langchain.mcptrigger.md)
- [Zapier MCP quickstart](https://docs.zapier.com/mcp/get-started/quickstart)
- [MCP architecture](https://modelcontextprotocol.io/docs/learn/architecture)
- [OpenAI Realtime conversations](https://developers.openai.com/api/docs/guides/realtime-conversations)
- [whisper.cpp](https://github.com/ggml-org/whisper.cpp)
- [Kokoro model card](https://huggingface.co/hexgrad/Kokoro-82M)

These references support planning assumptions only. Protocol versions, providers, prices, licenses, and product behavior can change; S0 and adapter qualification must record the versions actually tested.
