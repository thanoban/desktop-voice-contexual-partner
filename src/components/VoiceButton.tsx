import { createVoiceInteraction } from "@/lib/voiceInteraction";
import { useEffect, useCallback, useRef } from "react";
import { useChatStore } from "@/store/chatStore";
import { useSettingsStore } from "@/store/settingsStore";
import { startListening, stopListening, stopSpeaking, sendMessage, onPttToggle } from "@/lib/tauri";

interface Props {
  disabled?: boolean;
  voiceReady?: boolean;
  onNeedsSetup?: () => void;
}

export function VoiceButton({ disabled, voiceReady = true, onNeedsSetup }: Props) {
  const projectId = useSettingsStore((state) => state.settings.selected_project_id);
  const isListening  = useChatStore((s) => s.isListening);
  const isProcessing = useChatStore((s) => s.isProcessing);
  const isSpeaking   = useChatStore((s) => s.isSpeaking);
  const addMessage    = useChatStore((s) => s.addMessage);

  const state: "idle" | "listening" | "processing" | "speaking" = isSpeaking
    ? "speaking"
    : isProcessing ? "processing"
    : isListening  ? "listening"
    : "idle";

  // ── Toggle: click once to start, click again to stop & send ──────────────
  const interaction = useRef(createVoiceInteraction());
  const handleToggle = useCallback(async () => {
    try {
      await interaction.current({
        state: useChatStore.getState,
        disabled: !!disabled,
        start: async () => { if (!voiceReady) { onNeedsSetup?.(); return; } await startListening(); },
        stop: stopListening,
        cancel: stopSpeaking,
        send: (text) => sendMessage(text, projectId),
        isCurrentProject: () => useSettingsStore.getState().settings.selected_project_id === projectId,
      });
    } catch (error) { if (useSettingsStore.getState().settings.selected_project_id === projectId) addMessage({ role: "assistant", content: `[Error: ${error}]` }); }
  }, [disabled, voiceReady, onNeedsSetup, addMessage, projectId]);

  // ── Space bar toggle (window focused) ────────────────────────────────────
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.code === "Space" && !e.repeat && document.activeElement === document.body) {
        e.preventDefault();
        handleToggle();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [handleToggle]);

  // ── Global Alt+Space toggle (works from tray / background) ───────────────
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let disposed = false;
    onPttToggle(() => { if (!disposed) void handleToggle(); }).then((fn) => { if (disposed) fn(); else unlisten = fn; });
    return () => { disposed = true; unlisten?.(); };
  }, [handleToggle]);

  // ── Render ────────────────────────────────────────────────────────────────
  const colors: Record<typeof state, string> = {
    idle:       "var(--bg-elevated)",
    listening:  "var(--accent)",
    processing: "var(--accent-dim)",
    speaking:   "#3b82f6",
  };

  const labels: Record<typeof state, string> = {
    idle:       disabled ? "Ollama not running" : "Press Space to Speak",
    listening:  "Listening… press Space to send",
    processing: "Thinking… tap to stop",
    speaking:   "Speaking… press Space to stop",
  };
  const interactionDisabled = !!disabled && state === "idle";

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: "10px" }}>
      <button
        onClick={handleToggle}
        disabled={interactionDisabled}
        type="button"
        style={{
          width: "72px",
          height: "72px",
          borderRadius: "50%",
          background: colors[state],
          border: `2px solid ${state === "idle" ? "var(--text-dim)" : colors[state]}`,
          cursor: interactionDisabled ? "not-allowed" : "pointer",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontSize: "28px",
          transition: "all 0.15s ease",
          transform: isListening ? "scale(1.08)" : "scale(1)",
          opacity: interactionDisabled ? 0.4 : 1,
        }}
        className={state === "idle" ? "animate-pulse-glow" : ""}
        aria-label={labels[state]}
      >
        {state === "processing"
          ? <span style={{ fontSize: "20px" }} className="animate-spin">⟳</span>
          : state === "speaking" ? "✕" : "🎙"}
      </button>

      <span style={{ fontSize: "12px", color: "var(--text-muted)", letterSpacing: "0.02em" }}>
        {labels[state]}
      </span>
    </div>
  );
}
