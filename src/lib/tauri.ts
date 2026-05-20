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
  personality: "gentle" | "playful" | "calm" | "energetic";
  piper_binary: string;
  piper_voice: string;
  onboarding_done: string;
  whisper_binary: string;
  whisper_model: string;
  audio_input_device: string;
  window_context_auto: string;
  voice_speed: string;
  voice_expressiveness: string;
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

// ── Chat commands ────────────────────────────────────────────────────────────

export const sendMessage = (content: string): Promise<void> =>
  invoke("send_message", { content });

export const startNewSession = (): Promise<string> =>
  invoke("start_new_session");

export const getGreeting = (): Promise<void> =>
  invoke("get_greeting");

// ── TTS commands ─────────────────────────────────────────────────────────────

export const speakText = (text: string, voice: string): Promise<void> =>
  invoke("speak_text", { text, voice });

export const stopSpeaking = (): Promise<void> =>
  invoke("stop_speaking");

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

export const onChatToken = (cb: (token: string) => void): Promise<UnlistenFn> =>
  listen<string>("chat:token", (e) => cb(e.payload));

export const onChatDone = (cb: () => void): Promise<UnlistenFn> =>
  listen("chat:done", () => cb());

export const onChatError = (cb: (msg: string) => void): Promise<UnlistenFn> =>
  listen<string>("chat:error", (e) => cb(e.payload));

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

export const onSafetyShow = (cb: () => void): Promise<UnlistenFn> =>
  listen("safety:show", () => cb());
