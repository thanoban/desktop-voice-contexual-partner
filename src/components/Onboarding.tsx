import { useState } from "react";
import { useSettingsStore } from "@/store/settingsStore";

interface Props {
  onDone: () => void;
}

export function Onboarding({ onDone }: Props) {
  const [step, setStep] = useState<"age" | "disclosure">("age");
  const update = useSettingsStore((s) => s.update);

  const confirmAge = () => setStep("disclosure");

  const confirmDisclosure = async () => {
    await update("onboarding_done", "true");
    onDone();
  };

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "var(--bg-base)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 300,
        padding: "24px",
      }}
    >
      <div
        style={{
          maxWidth: "400px",
          width: "100%",
          display: "flex",
          flexDirection: "column",
          gap: "20px",
          textAlign: "center",
        }}
        className="animate-fade-in"
      >
        <div style={{ fontSize: "48px" }}>✦</div>

        {step === "age" && (
          <>
            <h1 style={{ fontSize: "22px", fontWeight: 700 }}>Welcome to VoicePartner</h1>
            <p style={{ color: "var(--text-muted)", fontSize: "14px", lineHeight: 1.7 }}>
              VoicePartner is designed for adults. Please confirm you are 18 or older to continue.
            </p>
            <button onClick={confirmAge} style={primaryBtn}>
              I am 18 or older — Continue
            </button>
            <p style={{ fontSize: "12px", color: "var(--text-muted)" }}>
              Not 18+? Please close this app.
            </p>
          </>
        )}

        {step === "disclosure" && (
          <>
            <h1 style={{ fontSize: "22px", fontWeight: 700 }}>A few things to know</h1>
            <div style={{ textAlign: "left", display: "flex", flexDirection: "column", gap: "12px", fontSize: "14px", color: "var(--text-muted)", lineHeight: 1.7 }}>
              <p>✦ VoicePartner is an <strong style={{ color: "var(--text-primary)" }}>AI companion</strong>, not a therapist or a replacement for human connection.</p>
              <p>✦ All your data — conversations, memories, documents — stays <strong style={{ color: "var(--text-primary)" }}>entirely on your machine</strong>. Nothing is sent to the cloud.</p>
              <p>✦ If you&apos;re ever in distress, VoicePartner will gently point you toward <strong style={{ color: "var(--text-primary)" }}>real human support</strong>.</p>
              <p>✦ The companion has limitations and can be wrong. Don&apos;t use it for medical, legal, or mental health advice.</p>
            </div>
            <button onClick={confirmDisclosure} style={primaryBtn}>
              Understood — Let&apos;s begin
            </button>
          </>
        )}
      </div>
    </div>
  );
}

const primaryBtn: React.CSSProperties = {
  background: "var(--accent)",
  border: "none",
  borderRadius: "10px",
  color: "#fff",
  fontSize: "15px",
  fontWeight: 600,
  padding: "13px 24px",
  cursor: "pointer",
  width: "100%",
};
