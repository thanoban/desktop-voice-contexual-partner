import { useEffect, useCallback } from "react";
import { useChatStore } from "@/store/chatStore";
import { startListening, stopListening, stopSpeaking, sendMessage, onPttToggle } from "@/lib/tauri";

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

  const state: "idle" | "listening" | "processing" | "speaking" = isSpeaking
    ? "speaking"
    : isProcessing ? "processing"
    : isListening  ? "listening"
    : "idle";

  // ── Toggle: click once to start, click again to stop & send ──────────────
  const handleToggle = useCallback(async () => {
    if (disabled) return;

    // Interrupt TTS if speaking
    if (isSpeaking) {
      await stopSpeaking().catch(() => null);
      return;
    }

    if (isProcessing) return;

    if (!isListening) {
      // ── Start ──
      if (!voiceReady) { onNeedsSetup?.(); return; }
      setListening(true);
      try {
        await startListening();
      } catch (e) {
        setListening(false);
        addMessage({ role: "assistant", content: `[Voice setup needed: ${e}]` });
      }
    } else {
      // ── Stop & send ──
      setListening(false);
      setProcessing(true);
      try {
        const transcript = await stopListening();
        if (!transcript?.trim()) { setProcessing(false); return; }
        const text = transcript.trim();
        addMessage({ role: "user", content: text });
        await sendMessage(text);
      } catch (e) {
        addMessage({ role: "assistant", content: `[Error: ${e}]` });
        setProcessing(false);
      }
    }
  }, [disabled, isListening, isProcessing, isSpeaking, voiceReady, onNeedsSetup, setListening, setProcessing, addMessage]);

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
    onPttToggle(() => handleToggle()).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [handleToggle]);

  // ── Render ────────────────────────────────────────────────────────────────
  const colors: Record<typeof state, string> = {
    idle:       "var(--bg-elevated)",
    listening:  "var(--accent)",
    processing: "var(--accent-dim)",
    speaking:   "#3b82f6",
  };

  const labels: Record<typeof state, string> = {
    idle:       "Press Space to Speak",
    listening:  "Listening… press Space to send",
    processing: "Thinking…",
    speaking:   "Tap to Interrupt",
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: "10px" }}>
      <button
        onClick={handleToggle}
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
