import { useEffect, useState } from "react";
import { Transcript } from "@/components/Transcript";
import { VoiceButton } from "@/components/VoiceButton";
import { VoiceVisualizer } from "@/components/VoiceVisualizer";
import { StatusBar } from "@/components/StatusBar";
import { SettingsPanel } from "@/components/SettingsPanel";
import { SafetyPanel } from "@/components/SafetyPanel";
import { ModelSetupPanel } from "@/components/ModelSetupPanel";
import { Onboarding } from "@/components/Onboarding";
import { useChatStore } from "@/store/chatStore";
import { useSettingsStore } from "@/store/settingsStore";
import { useOllamaStore } from "@/store/ollamaStore";
import {
  onChatToken,
  onChatDone,
  onChatError,
  onSpeakStart,
  onSpeakEnd,
  onSafetyShow,
  getGreeting,
} from "@/lib/tauri";

export default function App() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [safetyVisible, setSafetyVisible] = useState(false);
  const [showVoiceSetup, setShowVoiceSetup] = useState(false);

  const { settings, load: loadSettings } = useSettingsStore();
  const { appendToken, finalizeStream, setProcessing, setSpeaking, addMessage } = useChatStore();
  const ollamaStatus = useOllamaStore((s) => s.status);

  const onboardingDone = settings.onboarding_done === "true";

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  // Wire up Tauri event listeners
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    Promise.all([
      onChatToken((token) => appendToken(token)),
      onChatDone(() => {
        finalizeStream();
        setProcessing(false);
      }),
      onChatError((msg) => {
        addMessage({ role: "assistant", content: `[Error: ${msg}]` });
        setProcessing(false);
      }),
      onSpeakStart(() => setSpeaking(true)),
      onSpeakEnd(() => setSpeaking(false)),
      onSafetyShow(() => setSafetyVisible(true)),
    ]).then((fns) => {
      unlisteners.push(...fns);
    });

    return () => unlisteners.forEach((fn) => fn());
  }, [appendToken, finalizeStream, setProcessing, setSpeaking, addMessage]);

  // Send greeting when Ollama connects (once per session)
  const [greeted, setGreeted] = useState(false);
  useEffect(() => {
    if (ollamaStatus === "connected" && onboardingDone && !greeted) {
      setGreeted(true);
      setProcessing(true);
      getGreeting().catch(() => setProcessing(false));
    }
  }, [ollamaStatus, onboardingDone, greeted, setProcessing]);

  if (!settings.onboarding_done || !onboardingDone) {
    return <Onboarding onDone={() => loadSettings()} />;
  }

  return (
    <div
      style={{
        height: "100vh",
        display: "flex",
        flexDirection: "column",
        background: "var(--bg-base)",
        overflow: "hidden",
      }}
    >
      {/* Title bar */}
      <div className="titlebar">
        <span style={{ fontSize: "13px", fontWeight: 600, color: "var(--text-muted)" }}>
          {settings.companion_name}
        </span>
        <div style={{ display: "flex", gap: "8px" }}>
          <button
            onClick={() => setSettingsOpen(true)}
            style={titleBtnStyle}
            aria-label="Settings"
            title="Settings"
          >
            ⚙
          </button>
        </div>
      </div>

      {/* Transcript */}
      <Transcript />

      {/* Voice visualizer */}
      <div style={{ padding: "8px 16px 4px" }}>
        <VoiceVisualizer />
      </div>

      {/* Voice button */}
      <div
        style={{
          padding: "12px 0 16px",
          display: "flex",
          justifyContent: "center",
          borderTop: "1px solid var(--text-dim)",
          background: "var(--bg-surface)",
        }}
      >
        <VoiceButton
          disabled={ollamaStatus !== "connected"}
          voiceReady={!!settings.whisper_binary}
          onNeedsSetup={() => setShowVoiceSetup(true)}
        />
      </div>

      {/* Status bar */}
      <StatusBar />

      {/* Overlays */}
      <SettingsPanel open={settingsOpen} onClose={() => setSettingsOpen(false)} />
      <SafetyPanel visible={safetyVisible} onDismiss={() => setSafetyVisible(false)} />
      {showVoiceSetup && (
        <ModelSetupPanel onDone={() => setShowVoiceSetup(false)} />
      )}
    </div>
  );
}

const titleBtnStyle: React.CSSProperties = {
  background: "transparent",
  border: "none",
  color: "var(--text-muted)",
  fontSize: "16px",
  cursor: "pointer",
  padding: "2px 6px",
  borderRadius: "4px",
};
