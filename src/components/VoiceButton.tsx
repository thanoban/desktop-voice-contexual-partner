import { useEffect, useCallback, useRef } from "react";
import { useChatStore } from "@/store/chatStore";
import { startListening, stopListening, stopSpeaking, sendMessage } from "@/lib/tauri";

interface Props {
  disabled?: boolean;
  voiceReady?: boolean;
  onNeedsSetup?: () => void;
}

export function VoiceButton({ disabled, voiceReady = true, onNeedsSetup }: Props) {
  const isListening  = useChatStore((s) => s.isListening);
  const isProcessing = useChatStore((s) => s.isProcessing);
  const isSpeaking   = useChatStore((s) => s.isSpeaking);
  const setListening  = useChatStore((s) => s.setListening);
  const setProcessing = useChatStore((s) => s.setProcessing);
  const addMessage    = useChatStore((s) => s.addMessage);

  const holdStart  = useRef<number>(0);
  const isHolding  = useRef<boolean>(false);

  const state: "idle" | "listening" | "processing" | "speaking" = isSpeaking
    ? "speaking"
    : isProcessing
    ? "processing"
    : isListening
    ? "listening"
    : "idle";

  // ── Interrupt companion speech ────────────────────────────────────────────
  const handleInterrupt = useCallback(async () => {
    if (isSpeaking) await stopSpeaking().catch(() => null);
  }, [isSpeaking]);

  // ── PTT begin ────────────────────────────────────────────────────────────
  const handlePTTStart = useCallback(async () => {
    if (disabled || isProcessing || isHolding.current) return;
    if (isSpeaking) { handleInterrupt(); return; }
    if (!voiceReady) { onNeedsSetup?.(); return; }

    isHolding.current = true;
    holdStart.current = Date.now();
    setListening(true);

    try {
      await startListening();
    } catch (e) {
      setListening(false);
      isHolding.current = false;
      // Surface config errors as a chat message so the user can act on them
      addMessage({ role: "assistant", content: `[Voice setup needed: ${e}]` });
    }
  }, [disabled, isProcessing, isSpeaking, voiceReady, onNeedsSetup, setListening, addMessage, handleInterrupt]);

  // ── PTT end ──────────────────────────────────────────────────────────────
  const handlePTTEnd = useCallback(async () => {
    if (!isHolding.current) return;
    isHolding.current = false;

    const held = Date.now() - holdStart.current;
    setListening(false);

    if (held < 300) {
      // Too short — silently cancel
      try { await stopListening(); } catch { /* ignore */ }
      return;
    }

    setProcessing(true);
    try {
      const transcript = await stopListening();
      if (!transcript || !transcript.trim()) {
        setProcessing(false);
        return;
      }
      const text = transcript.trim();
      addMessage({ role: "user", content: text });
      await sendMessage(text);
    } catch (e) {
      addMessage({ role: "assistant", content: `[Error: ${e}]` });
      setProcessing(false);
    }
  }, [setListening, setProcessing, addMessage]);

  // ── Keyboard PTT (Space bar) ─────────────────────────────────────────────
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

  // ── Render ─────────────────────────────────────────────────────────────
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
        onMouseLeave={() => { if (isHolding.current) handlePTTEnd(); }}
        disabled={disabled}
        type="button"
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
