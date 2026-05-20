interface Props {
  onClose: () => void;
}

const SHORTCUTS = [
  { key: "Space",       desc: "Toggle voice input" },
  { key: "Alt + Space", desc: "Global PTT (works from system tray)" },
  { key: "Esc",         desc: "Close panels" },
  { key: "?",           desc: "Show this dialog" },
];

export function AboutDialog({ onClose }: Props) {
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.75)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 300,
        padding: "24px",
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: "100%",
          maxWidth: "380px",
          background: "var(--bg-surface)",
          borderRadius: "16px",
          padding: "28px 24px",
          border: "1px solid var(--text-dim)",
          display: "flex",
          flexDirection: "column",
          gap: "20px",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div style={{ textAlign: "center" }}>
          <div style={{ fontSize: "40px", marginBottom: "10px", lineHeight: 1 }}>✦</div>
          <div style={{ fontSize: "20px", fontWeight: 700, color: "var(--text-primary)" }}>
            VoicePartner
          </div>
          <div style={{ fontSize: "12px", color: "var(--accent)", marginTop: "4px", fontWeight: 500 }}>
            Version 1.0.0
          </div>
          <div style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: "6px", lineHeight: 1.6 }}>
            Local-first AI voice companion<br />
            No cloud · No subscriptions · All data stays on your machine
          </div>
        </div>

        {/* Keyboard shortcuts */}
        <div>
          <div style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.06em", marginBottom: "8px" }}>
            Keyboard shortcuts
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            {SHORTCUTS.map((s) => (
              <div
                key={s.key}
                style={{ display: "flex", alignItems: "center", gap: "10px", padding: "5px 0" }}
              >
                <kbd style={{
                  background: "var(--bg-elevated)",
                  border: "1px solid var(--text-dim)",
                  borderRadius: "5px",
                  padding: "2px 8px",
                  fontSize: "11px",
                  fontFamily: "monospace",
                  color: "var(--text-primary)",
                  whiteSpace: "nowrap",
                  minWidth: "90px",
                  textAlign: "center",
                }}>
                  {s.key}
                </kbd>
                <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>{s.desc}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Footer links */}
        <div style={{ fontSize: "11px", color: "var(--text-muted)", display: "flex", flexDirection: "column", gap: "4px" }}>
          <div>
            License:{" "}
            <span style={{ color: "var(--accent)" }}>AGPL-3.0</span>
          </div>
          <div>
            Source:{" "}
            <a
              href="https://github.com/thanoban/desktop-voice-contexual-partner"
              target="_blank"
              rel="noreferrer noopener"
              style={{ color: "var(--accent)", textDecoration: "none" }}
            >
              github.com/thanoban/desktop-voice-contexual-partner
            </a>
          </div>
          <div style={{ marginTop: "4px", fontSize: "10px", opacity: 0.6 }}>
            Built with Tauri · Rust · React · Ollama · Piper TTS
          </div>
        </div>

        <button
          type="button"
          onClick={onClose}
          style={{
            background: "transparent",
            border: "1px solid var(--text-dim)",
            borderRadius: "8px",
            color: "var(--text-muted)",
            fontSize: "13px",
            padding: "9px",
            cursor: "pointer",
          }}
        >
          Close
        </button>
      </div>
    </div>
  );
}
