interface Props {
  onAllow: () => void;
  onDeny: () => void;
}

export function ContextPermissionDialog({ onAllow, onDeny }: Props) {
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.7)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 400,
        padding: "24px",
      }}
    >
      <div
        style={{
          width: "100%",
          maxWidth: "380px",
          background: "var(--bg-surface)",
          borderRadius: "16px",
          border: "1px solid var(--text-dim)",
          padding: "24px",
          display: "flex",
          flexDirection: "column",
          gap: "16px",
        }}
        className="animate-fade-in"
      >
        <div style={{ fontSize: "32px", textAlign: "center" }}>🪟</div>

        <h2 style={{ fontSize: "16px", fontWeight: 700, textAlign: "center" }}>
          Allow window context access?
        </h2>

        <p style={{ fontSize: "13px", color: "var(--text-muted)", lineHeight: 1.7, textAlign: "center" }}>
          VoicePartner wants to read the <strong style={{ color: "var(--text-primary)" }}>title of your active window</strong>{" "}
          and inject it into every message so the companion understands what you're working on.
        </p>

        <ul style={{ fontSize: "12px", color: "var(--text-muted)", lineHeight: 1.8, paddingLeft: "18px" }}>
          <li>Only the window <em>title</em> is read — not content or keystrokes</li>
          <li>Data stays on your machine and is never sent anywhere</li>
          <li>You can turn this off anytime in Settings → Voice Input</li>
        </ul>

        <div style={{ display: "flex", gap: "10px", marginTop: "4px" }}>
          <button
            type="button"
            onClick={onDeny}
            style={{
              flex: 1,
              background: "transparent",
              border: "1px solid var(--text-dim)",
              borderRadius: "10px",
              color: "var(--text-muted)",
              fontSize: "14px",
              padding: "11px",
              cursor: "pointer",
            }}
          >
            Don&apos;t allow
          </button>
          <button
            type="button"
            onClick={onAllow}
            style={{
              flex: 1,
              background: "var(--accent)",
              border: "none",
              borderRadius: "10px",
              color: "#fff",
              fontSize: "14px",
              fontWeight: 600,
              padding: "11px",
              cursor: "pointer",
            }}
          >
            Allow
          </button>
        </div>
      </div>
    </div>
  );
}
