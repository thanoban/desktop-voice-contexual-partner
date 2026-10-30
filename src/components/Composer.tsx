import { useCallback, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import { sendMessage, stopSpeaking } from "@/lib/tauri";
import { useChatStore } from "@/store/chatStore";
import { useSettingsStore } from "@/store/settingsStore";

interface Props {
  disabled?: boolean;
}

export function Composer({ disabled = false }: Props) {
  const projectId = useSettingsStore((state) => state.settings.selected_project_id);
  const [draft, setDraft] = useState("");
  const isProcessing = useChatStore((state) => state.isProcessing);
  const captureOrPlayback = useChatStore((state) => state.isListening || state.isSpeaking);
  const draftRevision = useRef(0);
  const addMessage = useChatStore((state) => state.addMessage);

  const pendingSubmit = useRef(false);
  const submit = useCallback(async () => {
    const content = draft.trim();
    const active = useChatStore.getState();
    if (!content || disabled || pendingSubmit.current || active.isProcessing || active.isListening || active.isSpeaking) return;
    pendingSubmit.current = true;

    const submittedRevision = draftRevision.current;
    try {
      await sendMessage(content, projectId);
      // Retain rejected submissions and any edits made while this request ran.
      if (draftRevision.current === submittedRevision) setDraft("");
    } catch (error) {
      if (useSettingsStore.getState().settings.selected_project_id !== projectId) return;
      addMessage({ role: "assistant", content: `[Error: ${String(error)}]` });
    } finally { pendingSubmit.current = false; }
  }, [addMessage, disabled, draft, isProcessing, projectId]);

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    void submit();
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      void submit();
    }
  };

  const cancel = () => {
    void stopSpeaking();
  };

  return (
    <form
      onSubmit={handleSubmit}
      style={{
        display: "flex",
        gap: "8px",
        padding: "10px 14px",
        borderTop: "1px solid var(--text-dim)",
        background: "var(--bg-surface)",
        alignItems: "flex-end",
      }}
    >
      <textarea
        value={draft}
        onChange={(event) => { draftRevision.current++; setDraft(event.target.value); }}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        rows={1}
        maxLength={12_000}
        aria-label="Message VoicePartner"
        placeholder={disabled ? "Start a text provider to chat" : "Type a message…"}
        style={{
          flex: 1,
          minHeight: "38px",
          maxHeight: "112px",
          resize: "vertical",
          borderRadius: "10px",
          border: "1px solid var(--text-dim)",
          background: "var(--bg-elevated)",
          color: "var(--text-primary)",
          padding: "9px 11px",
          font: "inherit",
          lineHeight: 1.35,
          outline: "none",
        }}
      />
      <button
        type={isProcessing ? "button" : "submit"}
        onClick={isProcessing ? cancel : undefined}
        disabled={!isProcessing && (disabled || captureOrPlayback || !draft.trim())}
        aria-label={isProcessing ? "Stop generating" : "Send message"}
        style={{
          height: "38px",
          minWidth: "58px",
          borderRadius: "10px",
          border: "none",
          background: "var(--accent)",
          color: "white",
          fontWeight: 600,
          cursor: !isProcessing && (disabled || captureOrPlayback || !draft.trim()) ? "not-allowed" : "pointer",
          opacity: !isProcessing && (disabled || captureOrPlayback || !draft.trim()) ? 0.45 : 1,
        }}
      >
        {isProcessing ? "Stop" : "Send"}
      </button>
    </form>
  );
}
