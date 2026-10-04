import { useCallback, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useChatStore } from "@/store/chatStore";
import { useOllamaStore } from "@/store/ollamaStore";
import { useSettingsStore } from "@/store/settingsStore";
import {
  startListening,
  stopListening,
  sendMessage,
  expandToMain,
  onPttToggle,
  onChatToken,
  onChatDone,
  onChatError,
  onChatCancelled,
  onSpeakStart,
  onSpeakEnd,
} from "@/lib/tauri";

export function Widget() {
  const { settings, load } = useSettingsStore();
  const { status: ollamaStatus, startPolling } = useOllamaStore();
  const {
    messages,
    streamingContent,
    isListening,
    isProcessing,
    isSpeaking,
    setListening,
    setProcessing,
    setSpeaking,
    appendToken,
    finalizeStream,
    addMessage,
  } = useChatStore();

  useEffect(() => { load(); }, [load]);
  useEffect(() => { const stop = startPolling(); return stop; }, [startPolling]);

  // Wire chat events so status text stays live
  useEffect(() => {
    const uns: Array<() => void> = [];
    let disposed = false;
    Promise.all([
      onChatToken(settings.selected_project_id, (t) => { if (!disposed) appendToken(t); }),
      onChatDone(settings.selected_project_id, () => { if (!disposed) { finalizeStream(); setProcessing(false); } }),
      onChatError(settings.selected_project_id, () => { if (!disposed) setProcessing(false); }),
      onChatCancelled(settings.selected_project_id, () => { if (!disposed) { finalizeStream(); setProcessing(false); } }),
      onSpeakStart(() => { if (!disposed) setSpeaking(true); }),
      onSpeakEnd(() => { if (!disposed) setSpeaking(false); }),
    ]).then((fns) => { if (disposed) fns.forEach((fn) => fn()); else uns.push(...fns); });
    return () => { disposed = true; uns.forEach((f) => f()); };
  }, [appendToken, finalizeStream, setProcessing, setSpeaking, settings.selected_project_id]);

  // ── Toggle: click/Space once to start, again to stop & send ─────────────

  const handleToggle = useCallback(async () => {
    if (ollamaStatus !== "connected") return;
    if (isProcessing) return;
    if (!isListening) {
      setListening(true);
      try {
        await startListening();
      } catch {
        setListening(false);
      }
    } else {
      setListening(false);
      setProcessing(true);
      try {
        const text = (await stopListening()).trim();
        if (useSettingsStore.getState().settings.selected_project_id !== settings.selected_project_id) return;
        if (!text) { setProcessing(false); return; }
        addMessage({ role: "user", content: text });
        await sendMessage(text, settings.selected_project_id);
      } catch {
        if (useSettingsStore.getState().settings.selected_project_id !== settings.selected_project_id) return;
        setProcessing(false);
      }
    }
  }, [isListening, isProcessing, setListening, setProcessing, addMessage, ollamaStatus, settings.selected_project_id]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.code === "Space" && !e.repeat) { e.preventDefault(); handleToggle(); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [handleToggle]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let disposed = false;
    onPttToggle(() => { if (!disposed) void handleToggle(); }).then((fn) => { if (disposed) fn(); else unlisten = fn; });
    return () => { disposed = true; unlisten?.(); };
  }, [handleToggle]);

  // ── Status text ───────────────────────────────────────────────────────────

  const state = isSpeaking ? "speaking"
    : isProcessing ? "processing"
    : isListening  ? "listening"
    : "idle";

  const lastAssistant = [...messages].reverse().find((m) => m.role === "assistant");
  const preview = (streamingContent || lastAssistant?.content || "").slice(-80);

  const statusText = {
    idle:       ollamaStatus === "connected" ? "Press Space to speak" : "Ollama not running",
    listening:  "Listening…",
    processing: "Thinking…",
    speaking:   preview || "Speaking…",
  }[state];

  const dotColor = {
    idle:       ollamaStatus === "connected" ? "var(--text-dim)" : "var(--error)",
    listening:  "var(--accent)",
    processing: "var(--accent-dim)",
    speaking:   "#3b82f6",
  }[state];

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div
      data-tauri-drag-region
      style={{
        width: "100vw",
        height: "100vh",
        background: "rgba(18, 16, 28, 0.93)",
        backdropFilter: "blur(18px)",
        WebkitBackdropFilter: "blur(18px)",
        borderRadius: "14px",
        border: "1px solid rgba(167, 139, 250, 0.28)",
        boxShadow: "0 8px 40px rgba(0,0,0,0.55)",
        display: "flex",
        alignItems: "center",
        padding: "0 14px",
        gap: "10px",
      }}
    >
      {/* Companion name */}
      <span style={{ fontSize: "13px", fontWeight: 700, color: "var(--accent)", flexShrink: 0, letterSpacing: "0.01em" }}>
        ✦ {settings.companion_name || "Amy"}
      </span>

      <span style={{ width: "1px", height: "14px", background: "var(--text-dim)", flexShrink: 0 }} />

      {/* Status / last message */}
      <span
        style={{
          flex: 1,
          fontSize: "12px",
          color: state === "idle" ? "var(--text-muted)" : "var(--text-primary)",
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          minWidth: 0,
        }}
      >
        {statusText}
      </span>

      {/* Status dot */}
      <span style={{ width: "6px", height: "6px", borderRadius: "50%", background: dotColor, flexShrink: 0, transition: "background 0.25s" }} />

      {/* Mic button */}
      <button
        type="button"
        onClick={handleToggle}
        aria-label={isListening ? "Click to stop" : "Click to speak"}
        title={isListening ? "Click to stop & send (Space)" : "Click to speak (Space)"}
        style={{
          width: "28px",
          height: "28px",
          borderRadius: "50%",
          background: isListening ? "var(--accent)" : "var(--bg-elevated)",
          border: `1px solid ${isListening ? "var(--accent)" : "var(--text-dim)"}`,
          color: "var(--text-primary)",
          fontSize: "13px",
          cursor: "pointer",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          flexShrink: 0,
          transition: "all 0.15s",
        }}
      >
        {isProcessing
          ? <span className="animate-spin" style={{ fontSize: "11px" }}>⟳</span>
          : "🎙"}
      </button>

      {/* Expand to full window */}
      <button
        type="button"
        onClick={() => expandToMain()}
        aria-label="Open full app"
        title="Open full app"
        style={iconBtnStyle}
      >
        ⬡
      </button>

      {/* Hide widget */}
      <button
        type="button"
        onClick={() => getCurrentWindow().hide()}
        aria-label="Hide widget"
        title="Hide widget"
        style={iconBtnStyle}
      >
        ✕
      </button>
    </div>
  );
}

const iconBtnStyle: React.CSSProperties = {
  background: "transparent",
  border: "none",
  color: "var(--text-muted)",
  fontSize: "13px",
  cursor: "pointer",
  padding: "3px 6px",
  borderRadius: "4px",
  flexShrink: 0,
};
