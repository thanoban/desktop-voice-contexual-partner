# VoicePartner — end-to-end product plan

Decision date: 2026-09-27. Status: approved direction; application implementation pending.

VoicePartner is a familiar personal partner for people who spend their working day at a PC. It provides warm conversation, continuity, quiet company, and useful help. Reducing the feeling of working alone is the central product goal. Connected automation platforms extend what the partner can accomplish.

This plan replaces the previous voice-only, local-only, conversation-only restrictions. Original documents, including the user's existing local notes, are preserved in [the historical archive](docs/archive/README.md). Historical claims are not current release evidence.

## Documentation and authority

| Read | Purpose |
|---|---|
| [Architecture](ARCHITECTURE.md) | Current implementation, target services, runtime and data flows |
| [Project structure](docs/project-structure.md) | Complete target file/folder map and migration ownership |
| [Contracts and data](docs/data-and-contracts.md) | Interfaces, events, schema, and migrations |
| [Integrations](docs/integrations.md) | MCP, OpenClaw, n8n, Zapier, adapter behavior |
| [Voice and quality](docs/voice-and-quality.md) | Audio pipeline, model selection, measurable evaluation |
| [Security and privacy](docs/security-and-privacy.md) | Enforced permissions and data boundaries |
| [Feature catalog](docs/feature-catalog.md) | User journeys, priorities, acceptance criteria |
| [Roadmap](ROADMAP.md) | Dependency-ordered implementation work packages |
| [Release and operations](docs/release-and-operations.md) | CI, packaging, pilot, distribution and support |
| [Decision register](docs/decisions.md) | Settled choices and upstream references |

This file owns product intent; the specialist documents own implementation details; ROADMAP owns delivery status. Update the decision register before changing a settled choice. Planned paths are not claims that files already exist.

## 1. Audience and outcomes

Primary audience: solo developers, freelancers, remote workers, students doing sustained PC work, and other adults who want company while working. The first evaluation cohort is adult PC workers, with developers and freelancers represented.

- Feel accompanied without losing control of attention or privacy.
- Speak naturally, interrupt easily, and receive a pleasant, intelligible response.
- Resume a shared project or conversation without repeating all its context.
- Delegate useful work through connected tools with visible progress and results.
- Choose fully local processing or explicitly enable paid providers.
- Keep personal conversations and separate client projects appropriately isolated.

The product does not promise clinical outcomes. Companionship is assessed through user research; superiority and loneliness-reduction claims require comparative evidence.

## 2. A consistent partner

| Mode | Behavior | Initiative |
|---|---|---|
| Keep me company | Listen, converse, brainstorm, share humour, remember continuity | Respond when addressed; optional configured check-ins |
| Help me work | Explain, research, draft, plan, execute authorized tasks | Progress toward the requested outcome; report meaningful changes |
| Quiet focus | Stay available while minimizing interruptions | No unsolicited speech; completion cards remain available |

Mode changes preserve identity and memory; they never silently grant tools or export personal context. Users can customize name, voice, tone, response length, and pronunciation. Text input and transcript correction support silent work and accessibility.

The partner listens before prescribing, remembers accurately, acknowledges uncertainty, and ties encouragement to real progress. It does not guilt users for leaving, claim exclusivity, or infer permanent emotional traits from momentary frustration. End-of-day reflection, shared goals, and break prompts are optional.

Check-ins are off by default. On opt-in, start with no more than one every 90 minutes during selected working hours. Editable quiet hours default to 20:00–09:00 local time. Quiet focus and manual meeting/presentation suppression take precedence. Automatic meeting detection is an additional signal, never the only control. Missed check-ins expire instead of accumulating.

## 3. Settled defaults

| Decision | Default |
|---|---|
| Stack | Existing Tauri v2, Rust, React/TypeScript, Zustand |
| First supported release | Windows 11 x64; other operating systems are separate expansion gates |
| Reference local PC | 16 GB RAM, provisional 6 GB NVIDIA GPU; publish exact CPU/GPU before benchmarking |
| Language | English first, with Sri Lankan/Indian and other regional accents in evaluation |
| Input | Push-to-talk plus text; hands-free opt-in after acoustic qualification |
| Local inference | Ollama reasoning, Whisper recognition, Kokoro quality candidate, Piper lightweight speech |
| Paid inference | OpenAI text and compatible endpoints, ElevenLabs speech, OpenAI Realtime; Gemini Live next |
| Integrations | Optional guided connection to user-owned tools and accounts; no bundled automation engine initially |
| Persistence | Local versioned SQLite; encryption, lexical search, indexed vectors, provenance and deletion before beta |
| Business infrastructure | No mandatory account, hosted control plane, billing backend, or hosted public API initially |
| Licensing | Retain repository AGPL basis; reconcile existing metadata/header wording and audit bundled assets before distribution |

Hardware is a reference target, not a guarantee for every 16 GB PC. Reserve resources for a browser and editor. CPU/low-VRAM profiles disclose their limits. Select release models by measured quality and latency.

### Processing profiles

- **Private local:** local models and only integration routes whose local execution and downstream egress restrictions are verified. Setup downloads and update checks are separately authorized operations. Offline operation is tested.
- **Hybrid:** explicitly chosen local/paid stages with visible destinations for audio, text, images, project context, and tool results.
- **Cloud voice:** paid native-audio conversation; local policy still controls actions and context disclosure.

Never silently switch from local to cloud. A localhost service may call cloud models: connection location and downstream processing are distinct facts.

## 4. Build the relationship; integrate execution

VoicePartner owns voice, identity, personal memory, attention etiquette, context selection, routing, the permission interface, and activity history.

Use a generic MCP client for tools; an OpenClaw Gateway adapter for delegated jobs; n8n for repeatable workflows; Zapier MCP for cloud applications; and editor/browser integrations for work context. Retain a small Windows toolset for approved application/file opening, window switching, selected text, and emergency stop.

Distinguish bounded tool calls, predefined workflows, and agent jobs that plan intermediate steps. Shared interfaces do not imply identical cancellation, cost, authorization, or status support. One executor owns each operation.

Approved routine reads and reversible actions may run within their scope. Consequential actions require a concrete review or an explicit saved-workflow grant. If a delegated engine cannot enforce required boundaries, expose narrower tools or drafts. A prompt is not a permission boundary.

## 5. Feature priorities

Public beta must contain a usable complete core: reliable local partner conversation, optional paid providers, editable memory, project retrieval, MCP connections, a qualified OpenClaw adapter, n8n/Zapier setup recipes, action review, unified activity, and installation/recovery support.

Initial useful workflows: explain an error, draft a proposal or client update, resume a project, start a work session, run an approved report, and summarize background job outcomes. Return evidence such as a diff, document, citation, execution reference, or observed application state.

Later work: meetings, deeper native automation, more workflow packs, Sinhala/Tamil, broader languages, mobile access, encrypted sync, additional platforms, optional character visuals, and licensed custom voices. Each has its own gate. The [feature catalog](docs/feature-catalog.md) makes dependencies and acceptance explicit.

## 6. Proposed release targets

| Dimension | Target |
|---|---|
| Warm local conversation | End of speech to meaningful audible response: median <=1.5 s, p95 <=3 s on declared reference PC |
| Paid real-time conversation | Median <=1 s, p95 <=2 s under documented network conditions |
| Stop-button playback halt | p95 <=150 ms |
| Spoken interruption | p95 <=300 ms after detected speech onset |
| Recognition | WER <=8% clean and <=15% on defined accent/noise set; report subgroups |
| Technical vocabulary | >=95% exact recognition on declared corpus |
| Voice experience | Mean >=4/5 naturalness and comfort in blinded tests |
| Grounded answers | >=95% supported-answer correctness, including missing-answer cases |
| Supported workflows | >=95% verified completion on declared suite |
| Permissions | Zero unauthorized actions in declared adversarial release suite |
| Sustained operation | Eight-hour soak without crashes, orphan workers, or sustained memory growth |

These are targets, not achieved measurements. Cold starts, automation duration, acknowledgements, and completed answers are reported separately.

Run a consenting 20–30 person pilot across repeated PC-work sessions. Measure companionship, interruption burden, voice preference, task success, corrections, and cost. Compare against versioned alternatives and the original app under matched conditions. Do not optimize for chatting time at the expense of the user's work or relationships.

## 7. Delivery rules

Use stage gates rather than calendar promises. Each work package needs user outcome, dependencies, interface changes, tests, migration impact, and rollback notes. Keep code proof, automated tests, live services, installed-device verification, and pilot evidence separate.

Unqualified integrations remain disabled. Individual adapters can be disabled without breaking companionship. Current publication is documentation only; application implementation follows in a separate change. No stage is complete because its design is written.
