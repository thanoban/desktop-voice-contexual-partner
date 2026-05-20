import { useEffect } from "react";
import { useOllamaStore } from "@/store/ollamaStore";
import { useSettingsStore } from "@/store/settingsStore";

export function StatusBar() {
  const { status, error, startPolling } = useOllamaStore();
  const model = useSettingsStore((s) => s.settings.model);

  useEffect(() => {
    const stop = startPolling();
    return stop;
  }, [startPolling]);

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
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
        <span
          style={{
            width: "6px",
            height: "6px",
            borderRadius: "50%",
            background: dot[status],
            display: "inline-block",
          }}
        />
        <span>{label[status]}</span>
      </div>

      {status === "connected" && (
        <span style={{ opacity: 0.7 }}>{model}</span>
      )}
    </div>
  );
}
