# Voice pipeline, latency and quality plan

Status: proposed engineering and evaluation specification. No latency, recognition, comfort, or companionship target below has been demonstrated by the current app.

## 1. Audio pipeline

```text
Microphone -> bounded capture -> resample/preprocess -> VAD -> warm recognition
                                                               |
                                                  partial / final transcript
                                                               |
                                      permitted context -> streamed reasoning
                                                               |
                         phrase segmentation -> warm synthesis -> native playback
                                                               |
                                                     consumed audio offsets
```

Native-audio APIs are a second route behind the same session and cancellation contracts. Do not force their audio through an unnecessary STT->text->TTS round trip. Keep transcripts, context permission, tool approval, and consumed-output state consistent across routes.

## 2. Implementation requirements

### Capture and endpointing

- Enumerate actual default devices, supported formats, and rates. Convert f32/i16/u16 correctly; don't assume the default stream is f32.
- Bound capture to a configurable utterance duration; initial limit 120 seconds with visible warning before cutoff. Dictation/meeting recording becomes a separate long-form feature later.
- Capture callbacks push into preallocated bounded buffers. No DB access, file writes, or mutex contention with model work inside callbacks.
- Signal readiness, failure, stop, and completion explicitly. Eliminate the fixed sleep used to guess when a WAV finished writing.
- Resample with a validated anti-aliasing implementation. Verify 44.1/48 kHz and multichannel inputs against known signals.
- Silero VAD is the initial local candidate; pin a model/runtime after license and Windows packaging checks. Endpointing starts at 500 ms trailing silence, editable within a tested range, and includes initial-speech buffering to avoid clipping first phonemes.
- Support PTT and an explicit listening-session toggle. Wake word is later and separately consented. An open microphone always has an unmistakable indicator and stop control.
- Handle unplug, default-device change, sleep/resume, Bluetooth mode changes, and input errors without panics or false listening state.

### Recognition

- Use persistent whisper.cpp inference with explicit worker readiness. Start benchmark candidates with base.en and small.en; a faster/larger option is selected only through the release evaluation rule.
- Partial transcripts are provisional and can revise. Only final input, or an explicit typed correction, enters action routing.
- Supply user-approved names/technical vocabulary to supported recognizers. Do not silently replace speech with a guessed command.
- If an important action argument is ambiguous (recipient, path, amount), resolve it before execution. Show editable transcription before consequential operations.
- Silence/music/noise must not become tool requests. Test hallucinated transcription on non-speech inputs.
- Initial CLI fallback may remain for recovery but is labeled as a slower route and does not satisfy warm-worker qualification by itself.

### Generation and synthesis

- Parse NDJSON/SSE incrementally across arbitrary byte boundaries, including split UTF-8. Handle provider errors and incomplete terminal frames explicitly.
- Keep HTTP/WebSocket sessions reusable; warm selected models within the user's resource budget.
- Stream phrase-sized speech as soon as a coherent clause is available. Preserve abbreviations, decimals, punctuation and sentence continuity. Don't speak tool JSON, hidden reasoning, code blocks or long URLs.
- Use a single bounded speech queue. Synthesize a small amount ahead, not the whole remaining conversation. Keep prosody consistent at chunk boundaries.
- Kokoro worker loads its model once. A packaged runtime may be used initially; users must not need to install Python manually. Piper is the lightweight alternative; installed Windows voices are recovery options.
- Voice speed and expressive controls are mapped per engine. Piper length scale is not the same as a generic playback-rate value. Unsupported controls are disabled.
- Native playback reports the consumed sample offset and first audible sample. Temporary WAV files are transitional compatibility paths, not the target streaming architecture.
- On every failure path emit a terminal event, release ownership and clean temporary data. No permanently stuck speaking spinner.

### Interruption and acoustic quality

- Stop button cancels active output and flushes queued output immediately. It also cancels generation when requested; late worker results cannot resume speech.
- Spoken interruption suppresses self-triggering from the partner's speaker output through qualified echo cancellation. Headphone and speaker results are evaluated separately.
- Preserve the actually spoken portion in context and mark the rest interrupted. For native-audio providers, use their supported truncation/cancellation mechanism.
- Qualify noise suppression, gain handling, echo cancellation, and double-talk before advertising reliable hands-free speakers. Do not infer emotion from voice as a release requirement.

OpenAI documents the distinction between server-managed and client-managed interruption/truncation. The implementation must follow its chosen transport rather than only stopping the UI audio element. [Realtime conversations](https://developers.openai.com/api/docs/guides/realtime-conversations).

## 3. Hardware and model selection

Reference target: Windows 11 x64, 16 GB RAM, provisional 6 GB NVIDIA GPU, ordinary headset and laptop speakers. S0 records exact hardware, driver, power mode, model revision, quantization, prompt/context length, and competing editor/browser load.

Model selection procedure:

1. Benchmark compatible candidates with the same frozen test set.
2. Exclude candidates failing licensing/packaging, quality, or resource-headroom gates.
3. Among passing candidates, choose the smallest working set meeting latency targets; break ties by p95 latency, then voice preference.
4. Freeze artifact hashes and provider/model revision in a release catalog. Retain a rollback profile.
5. Publish a fast and quality profile only where each has distinct measured value.

Default local scheduling gives reasoning priority on GPU and uses CPU speech/embeddings when that avoids model eviction. Measure before selecting device placement; do not load several large models simultaneously just because each fits independently. Reduce context/model size or show a warming/degraded state rather than silently using cloud inference.

[whisper.cpp](https://github.com/ggml-org/whisper.cpp) documents acceleration options, and [Kokoro](https://huggingface.co/hexgrad/Kokoro-82M) is the initial lightweight speech candidate. Their existence does not establish performance on the user's PC.

## 4. Latency budget and instrumentation

Record monotonic timestamps for last speech sample, speech-end decision, final transcript, retrieval completion, first model token, first speakable phrase, first PCM, and first audible sample. Record cold/warm state, provider, hardware profile, and cancellation timing without transcript contents.

Illustrative warm-local median budget, used for diagnosis rather than independent promises:

| Stage | Budget |
|---|---|
| Endpointing after last speech | 400 ms |
| Final recognition tail | 250 ms |
| Bounded context retrieval | 100 ms |
| First useful phrase | 450 ms |
| First synthesis/playback audio | 250 ms |
| Scheduling margin | 50 ms |
| Total target | 1,500 ms |

Stages can overlap; calculate measured end-to-end latency directly rather than summing overlapping spans. Default 500 ms endpointing may consume extra budget and must be tuned using truncation/accuracy evidence. Do not optimize silence timeout so aggressively that users are cut off.

| Gate | Target |
|---|---|
| Warm local end-of-speech to meaningful first audio | p50 <=1.5 s; p95 <=3 s |
| Paid real-time under declared network conditions | p50 <=1 s; p95 <=2 s |
| Button to playback halt | p95 <=150 ms |
| Detected user speech to playback halt | p95 <=300 ms |

Cold startup, model warm-up, first token, acknowledgements, meaningful response audio, and full task completion are distinct measurements. Vendor synthesis-only claims exclude parts of the user journey; for example ElevenLabs explicitly distinguishes application/network latency. [ElevenLabs model documentation](https://elevenlabs.io/docs/overview/models).

## 5. Recognition and voice sweetness

Recognition corpus: consenting/licensed speakers, regional English accents, technical names, low-volume speech, background noise, Bluetooth/laptop/headset devices, and non-speech negatives. Split tuning and held-out evaluation by speaker. Report corpus size, WER normalization, exact entity/command match, and every subgroup; don't hide weak accent results inside an average.

Release targets: <=8% clean WER, <=15% accent/noise WER, >=95% exact technical-vocabulary recognition on the specified set. A narrowly perfect command corpus is not general conversation accuracy.

Voice audition uses identical short/long scripts and randomized labels. Rate warmth, naturalness, clarity, pronunciation, pacing, consistency, and fatigue after sustained listening. Target mean >=4/5 for naturalness and comfort, with distribution and sample size reported. Include neutral, joyful, thoughtful and frustrating work scenarios without exaggerating emotion.

Offer a pronunciation dictionary, pace control, concise response preference, and supported style presets. Do not claim that pitch or noise-scale settings alone create empathy. Voice cloning and copyrighted/identity-specific voices stay out of the initial release until consent, licensing, and quality gates are completed.

## 6. System accuracy and user outcomes

- Retrieval: curated project documents, expected source spans, missing/conflicting information, outdated revisions, same-name documents, and cross-project negatives. Target >=95% supported-answer correctness, not merely plausible wording.
- Automation: representative tasks with actual postconditions/artifacts, success >=95% for the declared supported suite. Record unsupported tasks separately and disclose exclusions.
- Companion pilot: 20–30 consenting adult PC workers over repeated sessions. Ask whether the partner felt warm, useful, attentive and appropriately quiet; measure perceived accompaniment without claiming a medical effect.
- Comparison: freeze competitor/product versions, tasks, hardware/network profiles, and scoring. Test original VoicePartner as a baseline. Publish limitations and confidence intervals where meaningful.
- Soak: eight-hour co-working, repeated device changes, suspend/resume, provider outage, queued work, indexing, and model unloading. Watch resident memory, handles, orphan processes, and task loss.

Do not publish superiority or loneliness-reduction claims from a code build or synthetic benchmark alone.
