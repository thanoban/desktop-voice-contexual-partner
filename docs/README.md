# VoicePartner documentation

The current design is the personal partner plus automation ecosystem plan approved on 2026-09-27. This publication contains plans only. Application code remains at the inspected baseline.

| Document | Read it for |
|---|---|
| [Product plan](../PLAN.md) | Audience, experience, defaults, release targets |
| [Architecture](../ARCHITECTURE.md) | Observed code and target runtime boundaries |
| [Project structure](project-structure.md) | Proposed folders/files, responsibilities and migration map |
| [Contracts and data](data-and-contracts.md) | Types, API, events, schema, migrations |
| [Integrations](integrations.md) | MCP/OpenClaw/n8n/Zapier, adapter lifecycle and setup |
| [Voice and quality](voice-and-quality.md) | Speech pipeline, latency, voice warmth, evaluation |
| [Security and privacy](security-and-privacy.md) | Grants, approvals, credentials, local mode, retention |
| [Feature catalog](feature-catalog.md) | Complete prioritized user-facing capability inventory |
| [Roadmap](../ROADMAP.md) | Implementation work packages and stage gates |
| [Release and operations](release-and-operations.md) | CI, installer, updates, support, pilot |
| [Decisions](decisions.md) | Locked choices and verified upstream references |
| [Historical archive](archive/README.md) | Previous documents and preserved user notes |

## Implementation handoff

Start with S0 in ROADMAP. Work in dependency order. For each package record its owned paths, contract version impact, test evidence, runtime qualification, and rollback. Do not mark planned platform/model support as implemented. A complete design, passing build, passing tests, live integration, installed release, and pilot outcome are different evidence levels.

The product plan wins on intent; specialist documents win on implementation detail; ROADMAP owns status. Historical documents are never an active source of requirements where they conflict with these documents.
