import { create } from "zustand";

export type Role = "user" | "assistant";

export interface Message {
  id: string;
  role: Role;
  content: string;
  timestamp: number;
  citations?: string[];
}

interface ChatState {
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
  messages: [],
  streamingContent: "",
  isStreaming: false,
  isListening: false,
  isSpeaking: false,
  isProcessing: false,
  currentSessionId: null,

  addMessage: (msg) => {
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
  clearMessages: () => set({ messages: [], streamingContent: "", currentSessionId: null }),
}));
