import { useEffect, useState } from "react";
import { useSettingsStore } from "@/store/settingsStore";
import { useOllamaStore } from "@/store/ollamaStore";

const PERSONALITIES = [
  { id: "gentle",    label: "Gentle",    desc: "Warm, patient, softly encouraging" },
  { id: "calm",      label: "Calm",      desc: "Steady, measured, quietly supportive" },
  { id: "playful",   label: "Playful",   desc: "Light-hearted, witty, upbeat" },
  { id: "energetic", label: "Energetic", desc: "Enthusiastic, lively, motivating" },
  { id: "mentor",    label: "Mentor",    desc: "Wise, guiding, nurtures your growth" },
  { id: "caring",    label: "Caring",    desc: "Deeply empathetic, makes you feel heard" },
];

const STEPS = ["age", "disclosure", "companion", "connect"] as const;
type Step = typeof STEPS[number];

interface Props {
  onDone: () => void;
}

export function Onboarding({ onDone }: Props) {
  const [step, setStep] = useState<Step>("age");
  const [name, setName] = useState("Amy");
  const [personality, setPersonality] = useState("gentle");
  const [selectedModel, setSelectedModel] = useState("");

  const { update } = useSettingsStore();
  const { status, models, poll } = useOllamaStore();

  useEffect(() => {
    if (step !== "connect") return;
    poll();
    const id = setInterval(() => poll(), 5000);
    return () => clearInterval(id);
  }, [step, poll]);

  useEffect(() => {
    if (models.length > 0 && !selectedModel) {
      setSelectedModel(models[0].name);
    }
  }, [models, selectedModel]);

  const saveCompanion = async () => {
    await update("companion_name", name.trim() || "Amy");
    await update("personality", personality);
    setStep("connect");
  };

  const finish = async () => {
    if (selectedModel) await update("model", selectedModel);
    await update("onboarding_done", "true");
    onDone();
  };

  const currentIdx = STEPS.indexOf(step);

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
          maxWidth: "420px",
          width: "100%",
          display: "flex",
          flexDirection: "column",
          gap: "20px",
          textAlign: "center",
        }}
        className="animate-fade-in"
      >
        {/* Step dots */}
        <div style={{ display: "flex", justifyContent: "center", gap: "7px" }}>
          {STEPS.map((s, i) => (
            <span
              key={s}
              style={{
                width: "7px",
                height: "7px",
                borderRadius: "50%",
                background: i === currentIdx
                  ? "var(--accent)"
                  : i < currentIdx
                    ? "var(--accent-dim)"
                    : "var(--text-dim)",
                transition: "background 0.3s",
              }}
            />
          ))}
        </div>

        <div style={{ fontSize: "40px" }}>✦</div>

        {step === "age" && (
          <>
            <h1 style={{ fontSize: "22px", fontWeight: 700 }}>Welcome to VoicePartner</h1>
            <p style={{ color: "var(--text-muted)", fontSize: "14px", lineHeight: 1.7 }}>
              VoicePartner is designed for adults. Please confirm you are 18 or older to continue.
            </p>
            <button type="button" onClick={() => setStep("disclosure")} style={primaryBtn}>
              I am 18 or older — Continue
            </button>
            <p style={{ fontSize: "12px", color: "var(--text-muted)" }}>Not 18+? Please close this app.</p>
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
            <button type="button" onClick={() => setStep("companion")} style={primaryBtn}>
              Understood — Continue
            </button>
          </>
        )}

        {step === "companion" && (
          <>
            <h1 style={{ fontSize: "22px", fontWeight: 700 }}>Meet your companion</h1>
            <p style={{ color: "var(--text-muted)", fontSize: "14px" }}>Give them a name and a personality.</p>

            <div style={{ textAlign: "left", display: "flex", flexDirection: "column", gap: "14px" }}>
              <div>
                <label style={{ fontSize: "12px", color: "var(--text-muted)", display: "block", marginBottom: "6px" }}>
                  Companion name
                </label>
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Amy"
                  maxLength={32}
                  style={inputStyle}
                />
              </div>

              <div>
                <label style={{ fontSize: "12px", color: "var(--text-muted)", display: "block", marginBottom: "6px" }}>
                  Personality
                </label>
                <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
                  {PERSONALITIES.map((p) => (
                    <button
                      type="button"
                      key={p.id}
                      onClick={() => setPersonality(p.id)}
                      style={{
                        padding: "8px 12px",
                        borderRadius: "8px",
                        border: `1px solid ${personality === p.id ? "var(--accent)" : "var(--text-dim)"}`,
                        background: personality === p.id ? "rgba(167,139,250,0.12)" : "transparent",
                        color: "var(--text-primary)",
                        cursor: "pointer",
                        textAlign: "left",
                        fontSize: "13px",
                      }}
                    >
                      <span style={{ fontWeight: 500 }}>{p.label}</span>
                      <span style={{ color: "var(--text-muted)", marginLeft: "6px" }}>{p.desc}</span>
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <button type="button" onClick={saveCompanion} style={primaryBtn}>
              Next
            </button>
          </>
        )}

        {step === "connect" && (
          <>
            <h1 style={{ fontSize: "22px", fontWeight: 700 }}>Connect to Ollama</h1>
            <p style={{ color: "var(--text-muted)", fontSize: "14px", lineHeight: 1.6 }}>
              VoicePartner uses <strong style={{ color: "var(--text-primary)" }}>Ollama</strong> to run your
              AI model locally. No cloud, no subscriptions.
            </p>

            {status === "connected" ? (
              <div style={{ display: "flex", flexDirection: "column", gap: "14px", textAlign: "left" }}>
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "8px",
                    background: "rgba(52,211,153,0.08)",
                    border: "1px solid rgba(52,211,153,0.25)",
                    borderRadius: "8px",
                    padding: "10px 14px",
                    fontSize: "13px",
                    color: "var(--success)",
                  }}
                >
                  <span>●</span>
                  <span>Ollama is running · {models.length} model{models.length !== 1 ? "s" : ""} available</span>
                </div>

                {models.length > 0 && (
                  <div>
                    <label style={{ fontSize: "12px", color: "var(--text-muted)", display: "block", marginBottom: "6px" }}>
                      Choose a model
                    </label>
                    <select
                      aria-label="LLM model"
                      value={selectedModel}
                      onChange={(e) => setSelectedModel(e.target.value)}
                      style={inputStyle}
                    >
                      {models.map((m) => (
                        <option key={m.name} value={m.name}>{m.name}</option>
                      ))}
                    </select>
                    <p style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "6px" }}>
                      Recommended: llama3.2 or mistral for a good balance of speed and quality.
                    </p>
                  </div>
                )}

                <button type="button" onClick={finish} style={primaryBtn}>
                  Let&apos;s begin
                </button>
              </div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: "14px", textAlign: "left" }}>
                <div
                  style={{
                    background: "rgba(251,191,36,0.08)",
                    border: "1px solid rgba(251,191,36,0.25)",
                    borderRadius: "8px",
                    padding: "10px 14px",
                    fontSize: "12px",
                    color: "var(--warning)",
                  }}
                >
                  {status === "connecting" ? "Checking for Ollama…" : "Ollama not detected. Follow these steps:"}
                </div>

                {status !== "connecting" && (
                  <ol style={{ fontSize: "13px", color: "var(--text-muted)", lineHeight: 1.9, paddingLeft: "18px" }}>
                    <li>
                      Download and install Ollama from{" "}
                      <strong style={{ color: "var(--text-primary)" }}>ollama.com</strong>
                    </li>
                    <li>
                      Open a terminal and run:{" "}
                      <code style={{ fontSize: "11px", background: "var(--bg-elevated)", padding: "1px 5px", borderRadius: "3px" }}>
                        ollama pull llama3.2
                      </code>
                    </li>
                    <li>Ollama starts automatically on login.</li>
                  </ol>
                )}

                <div style={{ display: "flex", gap: "10px" }}>
                  <button type="button" onClick={poll} style={secondaryBtn}>
                    Retry
                  </button>
                  <button type="button" onClick={finish} style={{ ...primaryBtn, flex: 1 }}>
                    Skip for now
                  </button>
                </div>
                <p style={{ fontSize: "11px", color: "var(--text-muted)", textAlign: "center" }}>
                  You can connect Ollama later — the app will work when it&apos;s running.
                </p>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  background: "var(--bg-elevated)",
  border: "1px solid var(--text-dim)",
  borderRadius: "6px",
  padding: "8px 10px",
  color: "var(--text-primary)",
  fontSize: "13px",
  outline: "none",
  width: "100%",
  cursor: "pointer",
};

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

const secondaryBtn: React.CSSProperties = {
  flex: 1,
  background: "transparent",
  border: "1px solid var(--text-dim)",
  borderRadius: "10px",
  color: "var(--text-muted)",
  fontSize: "14px",
  padding: "12px",
  cursor: "pointer",
};
