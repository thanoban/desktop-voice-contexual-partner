import { create } from "zustand";
import { getOllamaStatus, type OllamaModel, type OllamaStatus } from "@/lib/tauri";

type ConnectionState = "connected" | "connecting" | "disconnected" | "error";

interface OllamaState {
  status: ConnectionState;
  models: OllamaModel[];
  error: string | null;
  poll: () => Promise<void>;
  startPolling: () => () => void;
}

export const useOllamaStore = create<OllamaState>((set) => ({
  status: "connecting",
  models: [],
  error: null,

  poll: async () => {
    if (useOllamaStore.getState().status !== "connected") {
      set({ status: "connecting" });
    }
    try {
      const s: OllamaStatus = await getOllamaStatus();
      set({
        status: s.connected ? "connected" : "disconnected",
        models: s.models,
        error: s.error ?? null,
      });
    } catch (e) {
      set({ status: "error", error: String(e) });
    }
  },

  startPolling: () => {
    const store = useOllamaStore.getState();
    store.poll();
    const id = setInterval(() => useOllamaStore.getState().poll(), 5000);
    return () => clearInterval(id);
  },
}));
