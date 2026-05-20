import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
  onContextUpdate,
  startSharingContext,
  stopSharingContext,
  getGreeting,
  type ContextStatus,
} from "@/lib/tauri";

export default function App() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [safetyVisible, setSafetyVisible] = useState(false);
  const [showVoiceSetup, setShowVoiceSetup] = useState(false);
  const [ctx, setCtx] = useState<ContextStatus>({ sharing: false });

  const { settings, load: loadSettings, loaded } = useSettingsStore();
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
      onContextUpdate((s) => setCtx(s)),
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

  if (!loaded) {
    return (
      <div style={{ height: "100vh", display: "flex", alignItems: "center", justifyContent: "center", background: "var(--bg-base)" }}>
        <span style={{ fontSize: "13px", color: "var(--text-muted)" }}>Loading…</span>
      </div>
    );
  }

  if (!onboardingDone) {
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
        <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
          <button
            type="button"
            onClick={() => ctx.sharing
              ? stopSharingContext().then(() => setCtx({ sharing: false }))
              : startSharingContext().then(setCtx)
            }
            style={{
              ...titleBtnStyle,
              color: ctx.sharing ? "#60a5fa" : "var(--text-muted)",
              fontSize: "13px",
            }}
            aria-label={ctx.sharing ? "Stop sharing window context" : "Share window context"}
            title={ctx.sharing ? `Sharing: ${ctx.window_title ?? "active window"}` : "Share what you're working on"}
          >
            ◉
          </button>
          <button
            type="button"
            onClick={() => setSettingsOpen(true)}
            style={titleBtnStyle}
            aria-label="Settings"
            title="Settings"
          >
            ⚙
          </button>
          <div style={{ width: "1px", height: "14px", background: "var(--text-dim)", margin: "0 2px" }} />
          <button
            type="button"
            onClick={() => getCurrentWindow().minimize()}
            style={titleBtnStyle}
            aria-label="Minimize"
            title="Minimize"
          >
            −
          </button>
          <button
            type="button"
            onClick={() => getCurrentWindow().close()}
            style={{ ...titleBtnStyle, fontSize: "14px" }}
            aria-label="Close"
            title="Close"
          >
            ✕
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
