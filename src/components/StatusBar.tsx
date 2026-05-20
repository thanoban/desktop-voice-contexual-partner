import { useEffect, useState } from "react";
import { useOllamaStore } from "@/store/ollamaStore";
import { useSettingsStore } from "@/store/settingsStore";
import { onContextUpdate, stopSharingContext, type ContextStatus } from "@/lib/tauri";

export function StatusBar() {
  const { status, error, startPolling } = useOllamaStore();
  const { model, window_context_auto } = useSettingsStore((s) => ({
    model: s.settings.model,
    window_context_auto: s.settings.window_context_auto,
  }));
  const [ctx, setCtx] = useState<ContextStatus>({ sharing: false });

  useEffect(() => {
    const stop = startPolling();
    return stop;
  }, [startPolling]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    onContextUpdate((s) => setCtx(s)).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, []);

  const dot: Record<string, string> = {
    connected:    "var(--success)",
    connecting:   "var(--warning)",
    disconnected: "var(--error)",
    error:        "var(--error)",
  };

  const label: Record<string, string> = {
    connected:    "Connected",
    connecting:   "Connecting…",
    disconnected: "Ollama not running",
    error:        error ?? "Error",
  };

  return (
    <div
      style={{
        height: "28px",
        background: "var(--bg-base)",
        borderTop: "1px solid var(--text-dim)",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "0 14px",
        fontSize: "11px",
        color: "var(--text-muted)",
        flexShrink: 0,
        gap: "12px",
      }}
    >
      {/* Left: Ollama status */}
      <div style={{ display: "flex", alignItems: "center", gap: "6px", flex: 1 }}>
        <span
          style={{
            width: "6px",
            height: "6px",
            borderRadius: "50%",
            background: dot[status],
            display: "inline-block",
            flexShrink: 0,
          }}
        />
        <span>{label[status]}</span>
        {status === "connected" && (
          <span style={{ opacity: 0.6, marginLeft: "4px" }}>{model}</span>
        )}
      </div>

      {/* Auto-context badge */}
      {window_context_auto === "true" && !ctx.sharing && (
        <span
          style={{
            background: "rgba(167,139,250,0.1)",
            border: "1px solid rgba(167,139,250,0.25)",
            borderRadius: "10px",
            padding: "1px 8px",
            color: "var(--accent)",
            fontSize: "10px",
            flexShrink: 0,
          }}
          title="Auto-context: active window is injected into every message"
        >
          ◎ Auto
        </span>
      )}

      {/* Right: Manual context sharing indicator */}
      {ctx.sharing && (
        <button
          type="button"
          onClick={() => stopSharingContext().then(() => setCtx({ sharing: false }))}
          title={`Sharing context: ${ctx.window_title ?? "active window"} — click to stop`}
          style={{
            display: "flex",
            alignItems: "center",
            gap: "4px",
            background: "rgba(59,130,246,0.15)",
            border: "1px solid rgba(59,130,246,0.35)",
            borderRadius: "10px",
            padding: "1px 8px",
            color: "#60a5fa",
            fontSize: "10px",
            cursor: "pointer",
            maxWidth: "200px",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            flexShrink: 0,
          }}
        >
          <span>◉</span>
          <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>
            {ctx.window_title
              ? ctx.window_title.slice(0, 30) + (ctx.window_title.length > 30 ? "…" : "")
              : "Sharing context"}
          </span>
        </button>
      )}
    </div>
  );
}
