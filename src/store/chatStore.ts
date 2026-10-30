import { create } from "zustand";
import type { ConversationSnapshot } from "@/lib/tauri";

export type Role = "user" | "assistant";

export interface Message {
  id: string;
  role: Role;
  content: string;
  timestamp: number;
  citations?: string[];
}

interface ChatState {
  snapshotRevision: number;
  applySnapshot: (snapshot: ConversationSnapshot) => void;
  messages: Message[];
  streamingContent: string;
  isStreaming: boolean;
  isListening: boolean;
  isSpeaking: boolean;
  isProcessing: boolean;
  currentSessionId: string | null;

  addMessage: (msg: Omit<Message, "id" | "timestamp">) => Message;
  appendToken: (token: string) => void;
  finalizeStream: (citations?: string[]) => void;
  setListening: (v: boolean) => void;
  setSpeaking: (v: boolean) => void;
  setProcessing: (v: boolean) => void;
  setSession: (id: string) => void;
  clearMessages: () => void;
}

let _idCounter = 0;
const uid = () => `msg_${Date.now()}_${_idCounter++}`;

export const useChatStore = create<ChatState>((set, get) => ({
  snapshotRevision: -1,
  applySnapshot: (snapshot) => set((state) => {
    if (snapshot.revision <= state.snapshotRevision) return state;
    return {
      snapshotRevision: snapshot.revision,
      messages: snapshot.error ? [...snapshot.messages, { id: `error:${snapshot.revision}`, role: "assistant" as const, content: `[Error: ${snapshot.error}]`, timestamp: Date.now() }] : snapshot.messages,
      streamingContent: snapshot.streaming_content,
      isStreaming: snapshot.generating && !!snapshot.streaming_content,
      isProcessing: snapshot.generating || snapshot.audio.transcribing,
      isListening: snapshot.audio.listening,
      isSpeaking: snapshot.audio.speaking,
      currentSessionId: snapshot.session_id,
    };
  }),
  messages: [],
  streamingContent: "",
  isStreaming: false,
  isListening: false,
  isSpeaking: false,
  isProcessing: false,
  currentSessionId: null,

  addMessage: (msg) => {
    const messages = get().messages;
    const previous = messages[messages.length - 1];
    if (previous?.role === msg.role && previous.content === msg.content && msg.content.startsWith("[Error:")) return previous;
    const full: Message = { ...msg, id: uid(), timestamp: Date.now() };
    set((s) => ({ messages: [...s.messages, full] }));
    return full;
  },

  appendToken: (token) => {
    set((s) => ({
      streamingContent: s.streamingContent + token,
      isStreaming: true,
    }));
  },

  finalizeStream: (citations) => {
    const { streamingContent } = get();
    if (!streamingContent) return;
    const msg: Message = {
      id: uid(),
      role: "assistant",
      content: streamingContent,
      timestamp: Date.now(),
      citations,
    };
    set((s) => ({
      messages: [...s.messages, msg],
      streamingContent: "",
      isStreaming: false,
    }));
  },

  setListening: (v) => set({ isListening: v }),
  setSpeaking: (v) => set({ isSpeaking: v }),
  setProcessing: (v) => set({ isProcessing: v }),
  setSession: (id) => set({ currentSessionId: id }),
  // Keep the process-wide watermark when changing projects: switching back
  // must not admit delayed snapshots from an earlier session in that project.
  clearMessages: () => set({ messages: [], streamingContent: "", currentSessionId: null, isStreaming: false, isProcessing: false, isListening: false }),
}));
