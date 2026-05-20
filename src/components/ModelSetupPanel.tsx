import { useState } from "react";
import { useSettingsStore } from "@/store/settingsStore";

interface Props {
  onDone: () => void;
}

export function ModelSetupPanel({ onDone }: Props) {
  const { update } = useSettingsStore();
  const [whisperBin, setWhisperBin] = useState("");
  const [whisperModel, setWhisperModel] = useState("");

  const save = async () => {
    if (whisperBin) await update("whisper_binary", whisperBin);
    if (whisperModel) await update("whisper_model", whisperModel);
    onDone();
  };

  return (
    <div
      className="animate-fade-in"
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.75)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 200,
        padding: "24px",
      }}
    >
      <div
        style={{
          width: "100%",
          maxWidth: "420px",
          background: "var(--bg-surface)",
          borderRadius: "16px",
          padding: "24px",
          border: "1px solid var(--text-dim)",
          display: "flex",
          flexDirection: "column",
          gap: "18px",
        }}
      >
        <h2 style={{ fontSize: "17px", fontWeight: 700 }}>Voice Input Setup</h2>

        <p style={{ fontSize: "13px", color: "var(--text-muted)", lineHeight: 1.7 }}>
          VoicePartner uses{" "}
          <strong style={{ color: "var(--text-primary)" }}>whisper.cpp</strong> for speech-to-text,
          running entirely on your machine. You need two things:
        </p>

        <ol style={{ fontSize: "13px", color: "var(--text-muted)", lineHeight: 1.9, paddingLeft: "18px" }}>
          <li>
            <strong style={{ color: "var(--text-primary)" }}>whisper.cpp binary</strong> —{" "}
            download from{" "}
            <code style={{ fontSize: "11px", background: "var(--bg-elevated)", padding: "1px 5px", borderRadius: "3px" }}>
              github.com/ggerganov/whisper.cpp/releases
            </code>
          </li>
          <li style={{ marginTop: "6px" }}>
            <strong style={{ color: "var(--text-primary)" }}>Model file</strong> (ggml-base.en.bin, ~141 MB) —
            included in the whisper.cpp release or download from HuggingFace.
          </li>
        </ol>

        <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
          <label style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            Path to whisper binary
          </label>
          <input
            value={whisperBin}
            onChange={(e) => setWhisperBin(e.target.value)}
            placeholder="C:\tools\whisper\main.exe"
            style={inputStyle}
          />

          <label style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            Path to model file (.bin)
          </label>
          <input
            value={whisperModel}
            onChange={(e) => setWhisperModel(e.target.value)}
            placeholder="C:\tools\whisper\models\ggml-base.en.bin"
            style={inputStyle}
          />
        </div>

        <div style={{ display: "flex", gap: "10px", marginTop: "4px" }}>
          <button type="button" onClick={onDone} style={secondaryBtn}>
            Skip for now
          </button>
          <button type="button" onClick={save} style={primaryBtn}>
            Save &amp; Enable Voice
          </button>
        </div>

        <p style={{ fontSize: "11px", color: "var(--text-muted)" }}>
          You can always change these in Settings → Voice.
        </p>
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
  fontSize: "12px",
  outline: "none",
  width: "100%",
};

const primaryBtn: React.CSSProperties = {
  flex: 1,
  background: "var(--accent)",
  border: "none",
  borderRadius: "8px",
  color: "#fff",
  fontSize: "13px",
  fontWeight: 600,
  padding: "10px",
  cursor: "pointer",
};

const secondaryBtn: React.CSSProperties = {
  flex: 1,
  background: "transparent",
  border: "1px solid var(--text-dim)",
  borderRadius: "8px",
  color: "var(--text-muted)",
  fontSize: "13px",
  padding: "10px",
  cursor: "pointer",
};
