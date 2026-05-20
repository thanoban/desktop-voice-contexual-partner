import { useEffect, useRef } from "react";
import { useChatStore, type Message } from "@/store/chatStore";
import { useSettingsStore } from "@/store/settingsStore";
import { useOllamaStore } from "@/store/ollamaStore";

function Turn({ msg, companionName }: { msg: Message; companionName: string }) {
  const isUser = msg.role === "user";
  const time = new Date(msg.timestamp).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });

  return (
    <div
      className="animate-fade-in flex flex-col gap-1 px-4 py-2"
      style={{ alignItems: isUser ? "flex-end" : "flex-start" }}
    >
      <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>
        {isUser ? "You" : companionName} · {time}
      </span>
      <div
        style={{
          maxWidth: "78%",
          padding: "10px 14px",
          borderRadius: isUser ? "18px 18px 4px 18px" : "18px 18px 18px 4px",
          background: isUser ? "var(--user-bubble)" : "var(--comp-bubble)",
          color: "var(--text-primary)",
          fontSize: "14px",
          lineHeight: "1.65",
          userSelect: "text",
          border: `1px solid ${isUser ? "rgba(167,139,250,0.15)" : "rgba(255,255,255,0.04)"}`,
        }}
      >
        {msg.content}
        {msg.citations && msg.citations.length > 0 && (
          <div style={{ marginTop: "6px", fontSize: "11px", color: "var(--text-muted)" }}>
            📄 {msg.citations.join(", ")}
          </div>
        )}
      </div>
    </div>
  );
}

function StreamingTurn({ companionName }: { companionName: string }) {
  const content = useChatStore((s) => s.streamingContent);
  const isStreaming = useChatStore((s) => s.isStreaming);
  if (!isStreaming && !content) return null;

  return (
    <div className="flex flex-col gap-1 px-4 py-2" style={{ alignItems: "flex-start" }}>
      <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>{companionName}</span>
      <div
        style={{
          maxWidth: "78%",
          padding: "10px 14px",
          borderRadius: "18px 18px 18px 4px",
          background: "var(--comp-bubble)",
          color: "var(--text-primary)",
          fontSize: "14px",
          lineHeight: "1.65",
          userSelect: "text",
          border: "1px solid rgba(255,255,255,0.04)",
        }}
        className={content ? "cursor-blink" : ""}
      >
        {content || <span style={{ color: "var(--text-muted)" }}>thinking…</span>}
      </div>
    </div>
  );
}

export function Transcript() {
  const messages = useChatStore((s) => s.messages);
  const isStreaming = useChatStore((s) => s.isStreaming);
  const isProcessing = useChatStore((s) => s.isProcessing);
  const companionName = useSettingsStore((s) => s.settings.companion_name);
  const ollamaStatus = useOllamaStore((s) => s.status);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, isStreaming]);

  const showThinking = isProcessing && !isStreaming;

  return (
    <div
      style={{
        flex: 1,
        overflowY: "auto",
        display: "flex",
        flexDirection: "column",
        paddingTop: "8px",
        paddingBottom: "8px",
      }}
    >
      {messages.length === 0 && !showThinking && !isStreaming && (
        <div
          style={{
            flex: 1,
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            color: "var(--text-muted)",
            gap: "8px",
            padding: "24px",
            textAlign: "center",
          }}
        >
          <div style={{ fontSize: "40px", opacity: 0.4 }}>✦</div>
          {ollamaStatus === "connected" ? (
            <>
              <p style={{ fontSize: "14px" }}>Press Space to speak to {companionName}</p>
              <p style={{ fontSize: "12px", opacity: 0.6 }}>Or type a message below</p>
            </>
          ) : ollamaStatus === "connecting" ? (
            <p style={{ fontSize: "14px" }}>Connecting to Ollama…</p>
          ) : (
            <>
              <p style={{ fontSize: "14px" }}>Ollama is not running</p>
              <p style={{ fontSize: "12px", opacity: 0.6, maxWidth: "240px", lineHeight: 1.5 }}>
                Start Ollama on your machine, then it will connect automatically.
              </p>
              <a
                href="https://ollama.com"
                target="_blank"
                rel="noreferrer noopener"
                style={{ fontSize: "12px", color: "var(--accent)", textDecoration: "none", marginTop: "4px" }}
              >
                ollama.com ↗
              </a>
            </>
          )}
        </div>
      )}

      {messages.map((m) => (
        <Turn key={m.id} msg={m} companionName={companionName} />
      ))}

      {(isStreaming || showThinking) && (
        <StreamingTurn companionName={companionName} />
      )}

      <div ref={bottomRef} />
    </div>
  );
}
