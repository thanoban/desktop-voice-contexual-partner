import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ── Types mirroring Rust structs ────────────────────────────────────────────

export interface OllamaModel {
  name: string;
  size: number;
  modified_at: string;
}

export interface Settings {
  endpoint: string;
  model: string;
  companion_name: string;
  personality: "gentle" | "playful" | "calm" | "energetic" | "mentor" | "caring";
  partner_mode: "company" | "work" | "focus";
  selected_project_id: string;
  piper_binary: string;
  piper_voice: string;
  onboarding_done: string;
  whisper_binary: string;
  whisper_model: string;
  audio_input_device: string;
  window_context_auto: string;
  window_context_allowed: string;
  voice_speed: string;
  voice_expressiveness: string;
  embedding_model: string;
  custom_system_prompt: string;
  custom_voices: string;
  kokoro_model: string;
  kokoro_voices: string;
}

export interface OllamaStatus {
  connected: boolean;
  models: OllamaModel[];
  error?: string;
}

// ── System commands ──────────────────────────────────────────────────────────

export const getOllamaStatus = (): Promise<OllamaStatus> =>
  invoke("get_ollama_status");

export const listModels = (): Promise<OllamaModel[]> =>
  invoke("list_models");

// ── Settings commands ────────────────────────────────────────────────────────

export const getSettings = (): Promise<Settings> =>
  invoke("get_settings");

export const updateSetting = (key: string, value: string): Promise<void> =>
  invoke("update_setting", { key, value });

export interface ProjectInfo {
  id: string;
  name: string;
  kind: "personal" | "client" | "project";
}

export interface ProjectSelection {
  project_id: string;
  session_id: string;
}

export const listProjects = (): Promise<ProjectInfo[]> => invoke("list_projects");
export const createProject = (name: string, kind: "client" | "project"): Promise<ProjectInfo> =>
  invoke("create_project", { name, kind });
export const selectProject = (projectId: string): Promise<ProjectSelection> =>
  invoke("select_project", { projectId });
export const onProjectSelected = (cb: (selection: ProjectSelection) => void): Promise<UnlistenFn> =>
  listen<ProjectSelection>("project:selected", (event) => cb(event.payload));

// ── Chat commands ────────────────────────────────────────────────────────────

export const sendMessage = (content: string, projectId: string): Promise<void> =>
  invoke("send_message", { content, projectId });

export const startNewSession = (): Promise<string> =>
  invoke("start_new_session");

export const getGreeting = (projectId: string): Promise<void> =>
  invoke("get_greeting", { projectId });

// ── TTS commands ─────────────────────────────────────────────────────────────

export const speakText = (text: string, voice: string): Promise<void> =>
  invoke("speak_text", { text, voice });

export const getSapiVoices = (): Promise<string[]> =>
  invoke("get_sapi_voices");

export interface CancellationResponse {
  generation: "requested" | "no_active_turn";
  playback: "unsupported";
}

export interface ConversationSnapshot {
  active_turn_id?: string | null;
  cancellation_requested: boolean;
}

export const stopSpeaking = (): Promise<CancellationResponse> =>
  invoke("stop_speaking");

export const getConversationSnapshot = (): Promise<ConversationSnapshot> =>
  invoke("get_conversation_snapshot");

// ── Voice input commands (M1) ─────────────────────────────────────────────────

export interface AudioDevice {
  name: string;
  is_default: boolean;
}

export const getAudioDevices = (): Promise<AudioDevice[]> =>
  invoke("get_audio_devices");

export const startListening = (): Promise<void> =>
  invoke("start_listening");

/** Returns the transcribed text from the recorded audio. */
export const stopListening = (): Promise<string> =>
  invoke("stop_listening");

// ── Context sharing commands (M1) ─────────────────────────────────────────────

export interface ContextStatus {
  sharing: boolean;
  window_title?: string;
  custom_note?: string;
}

export const getWindowTitle = (): Promise<string | null> =>
  invoke("get_window_title");

export const startSharingContext = (): Promise<ContextStatus> =>
  invoke("start_sharing_context");

export const stopSharingContext = (): Promise<void> =>
  invoke("stop_sharing_context");

export const setContextNote = (note: string): Promise<ContextStatus> =>
  invoke("set_context_note", { note });

// ── Event listeners ──────────────────────────────────────────────────────────

interface ProjectEvent<T> { project_id: string; payload: T }
const listenProject = <T,>(name: string, projectId: string, cb: (payload: T) => void): Promise<UnlistenFn> =>
  listen<ProjectEvent<T>>(name, (event) => {
    if (event.payload.project_id === projectId) cb(event.payload.payload);
  });

export const onChatToken = (projectId: string, cb: (token: string) => void): Promise<UnlistenFn> =>
  listenProject<string>("chat:token", projectId, cb);

export const onChatDone = (projectId: string, cb: () => void): Promise<UnlistenFn> =>
  listenProject("chat:done", projectId, cb);

export const onChatError = (projectId: string, cb: (msg: string) => void): Promise<UnlistenFn> =>
  listenProject<string>("chat:error", projectId, cb);

export const onChatCancelled = (projectId: string, cb: (turnId: string) => void): Promise<UnlistenFn> =>
  listenProject<string>("chat:cancelled", projectId, cb);

export const onSpeakStart = (cb: () => void): Promise<UnlistenFn> =>
  listen("tts:start", () => cb());

export const onSpeakEnd = (cb: () => void): Promise<UnlistenFn> =>
  listen("tts:end", () => cb());

export const onVadSpeech = (cb: (active: boolean) => void): Promise<UnlistenFn> =>
  listen<boolean>("vad:speech", (e) => cb(e.payload));

export const onTranscription = (cb: (text: string) => void): Promise<UnlistenFn> =>
  listen<string>("stt:transcription", (e) => cb(e.payload));

export const onAudioListening = (cb: (active: boolean) => void): Promise<UnlistenFn> =>
  listen<boolean>("audio:listening", (e) => cb(e.payload));

export const onAudioProcessing = (cb: (active: boolean) => void): Promise<UnlistenFn> =>
  listen<boolean>("audio:processing", (e) => cb(e.payload));

export const onAudioError = (cb: (msg: string) => void): Promise<UnlistenFn> =>
  listen<string>("audio:error", (e) => cb(e.payload));

export const onContextUpdate = (cb: (status: ContextStatus) => void): Promise<UnlistenFn> =>
  listen<ContextStatus>("context:update", (e) => cb(e.payload));

export const onSafetyShow = (projectId: string, cb: () => void): Promise<UnlistenFn> =>
  listenProject("safety:show", projectId, cb);

// ── Memory commands (M2) ─────────────────────────────────────────────────────

export interface Memory {
  id: string;
  session_id: string | null;
  content: string;
  memory_type: string;
  created_at: number;
  project_id: string;
}

export const getMemories = (): Promise<Memory[]> =>
  invoke("get_memories");

export const getMemoryCount = (): Promise<number> =>
  invoke("get_memory_count");

export const deleteMemory = (id: string, projectId: string): Promise<void> =>
  invoke("delete_memory", { id, projectId });

export const forgetAll = (projectId: string): Promise<void> =>
  invoke("forget_all", { projectId });

// ── RAG — document ingestion (M3) ────────────────────────────────────────────

export interface DocumentInfo {
  source_file: string;
  chunk_count: number;
  ingested_at: number;
}

export interface IngestProgress {
  project_id: string;
  source: string;
  current: number;
  total: number;
}

export interface IngestResult {
  project_id: string;
  source: string;
  chunks: number;
}

export const pickDocument = (): Promise<string | null> =>
  invoke("pick_document");

export const ingestDocument = (path: string, projectId: string): Promise<string> =>
  invoke("ingest_document", { path, projectId });

export const listDocuments = (): Promise<DocumentInfo[]> =>
  invoke("list_documents");

export const deleteDocument = (sourceFile: string, projectId: string): Promise<void> =>
  invoke("delete_document", { sourceFile, projectId });

export const onRagProgress = (cb: (p: IngestProgress) => void): Promise<UnlistenFn> =>
  listen<IngestProgress>("rag:progress", (e) => cb(e.payload));

export const onRagDone = (cb: (r: IngestResult) => void): Promise<UnlistenFn> =>
  listen<IngestResult>("rag:done", (e) => cb(e.payload));

export interface IngestError {
  project_id: string;
  message: string;
}

export const onRagError = (cb: (error: IngestError) => void): Promise<UnlistenFn> =>
  listen<IngestError>("rag:error", (e) => cb(e.payload));

// ── Setup / download commands (M6) ───────────────────────────────────────────

export interface SetupStatus {
  piper_ok: boolean;
  piper_voice_ok: boolean;
  whisper_model_ok: boolean;
  piper_path: string;
  whisper_model_path: string;
  voice_paths: Record<string, string>;
  // Kokoro (M6)
  kokoro_model_ok: boolean;
  kokoro_voices_ok: boolean;
  python_ok: boolean;
  kokoro_lib_ok: boolean;
  kokoro_model_path: string;
  kokoro_voices_path: string;
}

export interface DownloadProgress {
  tool: string;
  downloaded: number;
  total: number;
}

export interface DownloadDone {
  tool: string;
  path: string;
}

export const checkSetup = (): Promise<SetupStatus> =>
  invoke("check_setup");

export const downloadTool = (tool: string): Promise<string> =>
  invoke("download_tool", { tool });

export const pickFile = (filters: string[]): Promise<string | null> =>
  invoke("pick_file", { filters });

export const onDownloadProgress = (
  cb: (p: DownloadProgress) => void
): Promise<UnlistenFn> =>
  listen<DownloadProgress>("download:progress", (e) => cb(e.payload));

export const onDownloadDone = (
  cb: (d: DownloadDone) => void
): Promise<UnlistenFn> =>
  listen<DownloadDone>("download:done", (e) => cb(e.payload));

export const onDownloadError = (
  cb: (d: { tool: string; error: string }) => void
): Promise<UnlistenFn> =>
  listen<{ tool: string; error: string }>("download:error", (e) => cb(e.payload));

// ── Widget commands (M5) ────────────────────────────────────────────────────

export const expandToMain = (): Promise<void> =>
  invoke("expand_to_main");

export const exportConversation = (content: string, filename: string): Promise<void> =>
  invoke("export_conversation", { content, filename });

// ── History commands ──────────────────────────────────────────────────────────

export interface SessionInfo {
  id: string;
  started_at: number;
  ended_at: number | null;
  turn_count: number;
  preview: string;
  project_id: string;
  partner_mode: "company" | "work" | "focus";
}

export interface TurnInfo {
  role: "user" | "assistant";
  content: string;
  created_at: number;
}

export const listSessions = (): Promise<SessionInfo[]> =>
  invoke("list_sessions");

export const getSessionTurns = (sessionId: string): Promise<TurnInfo[]> =>
  invoke("get_session_turns", { sessionId });

// ── Global PTT shortcut events (M5) ─────────────────────────────────────────

export const onPttToggle = (cb: () => void): Promise<UnlistenFn> =>
  listen("shortcut:ptt:toggle", () => cb());
