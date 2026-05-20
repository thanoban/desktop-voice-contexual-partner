import { create } from "zustand";
import { getSettings, updateSetting, type Settings } from "@/lib/tauri";

interface SettingsState {
  settings: Settings;
  loaded: boolean;
  load: () => Promise<void>;
  update: (key: keyof Settings, value: string) => Promise<void>;
}

const defaults: Settings = {
  endpoint: "http://localhost:11434",
  model: "llama3.2:8b",
  companion_name: "Amy",
  personality: "gentle",
  piper_binary: "",
  piper_voice: "en_US-amy-medium",
  onboarding_done: "false",
};

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: defaults,
  loaded: false,

  load: async () => {
    try {
      const s = await getSettings();
      set({ settings: { ...defaults, ...s }, loaded: true });
    } catch {
      set({ loaded: true });
    }
  },

  update: async (key, value) => {
    await updateSetting(key as string, value);
    set((s) => ({ settings: { ...s.settings, [key]: value } }));
  },
}));
