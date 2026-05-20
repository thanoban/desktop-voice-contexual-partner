interface Props {
  visible: boolean;
  onDismiss: () => void;
}

export function SafetyPanel({ visible, onDismiss }: Props) {
  if (!visible) return null;

  return (
    <div
      style={{
        position: "fixed",
        bottom: "60px",
        left: "50%",
        transform: "translateX(-50%)",
        zIndex: 200,
        width: "320px",
        background: "var(--bg-surface)",
        border: "1px solid var(--accent)",
        borderRadius: "12px",
        padding: "18px",
        boxShadow: "0 8px 32px rgba(0,0,0,0.6)",
        display: "flex",
        flexDirection: "column",
        gap: "12px",
      }}
      className="animate-fade-in"
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <p style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)" }}>
          You matter. Real support is available.
        </p>
        <button
          type="button"
          onClick={onDismiss}
          style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer", fontSize: "18px", flexShrink: 0, marginLeft: "8px" }}
        >
          ✕
        </button>
      </div>

      <p style={{ fontSize: "13px", color: "var(--text-muted)", lineHeight: 1.6 }}>
        I&apos;m an AI and I care — but a trained human can help more right now.
        Reach out to a crisis line in your country.
      </p>

      <a
        href="https://www.iasp.info/resources/Crisis_Centres"
        target="_blank"
        rel="noopener noreferrer"
        style={{
          display: "block",
          textAlign: "center",
          background: "var(--accent)",
          color: "#fff",
          fontWeight: 600,
          fontSize: "13px",
          padding: "10px",
          borderRadius: "8px",
          textDecoration: "none",
        }}
      >
        Find a crisis line near you →
      </a>
    </div>
  );
}
