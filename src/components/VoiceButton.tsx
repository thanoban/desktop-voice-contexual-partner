import { useEffect, useCallback, useRef } from "react";
import { useChatStore } from "@/store/chatStore";
import { sendMessage, stopSpeaking } from "@/lib/tauri";

interface Props {
  disabled?: boolean;
}

export function VoiceButton({ disabled }: Props) {
  const isListening = useChatStore((s) => s.isListening);
  const isProcessing = useChatStore((s) => s.isProcessing);
  const isSpeaking = useChatStore((s) => s.isSpeaking);
  const setListening = useChatStore((s) => s.setListening);
  const setProcessing = useChatStore((s) => s.setProcessing);
  const addMessage = useChatStore((s) => s.addMessage);

  const holdStart = useRef<number>(0);

  const state: "idle" | "listening" | "processing" | "speaking" = isSpeaking
    ? "speaking"
    : isProcessing
    ? "processing"
    : isListening
    ? "listening"
    : "idle";

  const handleInterrupt = useCallback(async () => {
    if (isSpeaking) {
      await stopSpeaking();
    }
  }, [isSpeaking]);

  const handlePTTStart = useCallback(() => {
    if (disabled || isProcessing) return;
    if (isSpeaking) {
      handleInterrupt();
      return;
    }
    holdStart.current = Date.now();
    setListening(true);
  }, [disabled, isProcessing, isSpeaking, setListening, handleInterrupt]);

  const handlePTTEnd = useCallback(async () => {
    if (!isListening) return;
    setListening(false);

    const held = Date.now() - holdStart.current;
    if (held < 300) return; // too short, ignore

    // M0 dev affordance: native dialog to type text until STT is wired in M1
    // This block will be removed in M1 when whisper.cpp STT is integrated
    const { ask } = await import("@tauri-apps/plugin-dialog");
    const text = await ask("What did you want to say?", {
      title: "Voice Input (M0 placeholder)",
      okLabel: "Send",
      cancelLabel: "Cancel",
    });
    if (!text || typeof text !== "string") return;

    const userText = String(text).trim();
    if (!userText) return;

    addMessage({ role: "user", content: userText });
    setProcessing(true);
    try {
      await sendMessage(userText);
    } finally {
      setProcessing(false);
    }
  }, [isListening, setListening, addMessage, setProcessing]);

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.code === "Space" && !e.repeat && document.activeElement === document.body) {
        e.preventDefault();
        handlePTTStart();
      }
    };
    const up = (e: KeyboardEvent) => {
      if (e.code === "Space") {
        e.preventDefault();
        handlePTTEnd();
      }
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
    };
  }, [handlePTTStart, handlePTTEnd]);

  const colors: Record<typeof state, string> = {
    idle:       "var(--bg-elevated)",
    listening:  "var(--accent)",
    processing: "var(--accent-dim)",
    speaking:   "#3b82f6",
  };

  const labels: Record<typeof state, string> = {
    idle:       "Hold Space to Speak",
    listening:  "Listening…",
    processing: "Thinking…",
    speaking:   "Tap to Interrupt",
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: "10px" }}>
      <button
        onMouseDown={handlePTTStart}
        onMouseUp={handlePTTEnd}
        onMouseLeave={() => { if (isListening) handlePTTEnd(); }}
        disabled={disabled}
        style={{
          width: "72px",
          height: "72px",
          borderRadius: "50%",
          background: colors[state],
          border: `2px solid ${state === "idle" ? "var(--text-dim)" : colors[state]}`,
          cursor: disabled ? "not-allowed" : "pointer",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontSize: "28px",
          transition: "all 0.15s ease",
          transform: isListening ? "scale(1.08)" : "scale(1)",
          opacity: disabled ? 0.4 : 1,
        }}
        className={state === "idle" ? "animate-pulse-glow" : ""}
        aria-label={labels[state]}
      >
        {state === "processing" ? (
          <span style={{ fontSize: "20px" }} className="animate-spin">⟳</span>
        ) : state === "speaking" ? "✕" : "🎙"}
      </button>

      <span style={{ fontSize: "12px", color: "var(--text-muted)", letterSpacing: "0.02em" }}>
        {labels[state]}
      </span>
    </div>
  );
}
