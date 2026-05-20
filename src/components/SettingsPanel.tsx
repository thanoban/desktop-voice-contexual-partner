import { useState, useEffect } from "react";
import { useSettingsStore } from "@/store/settingsStore";
import { useOllamaStore } from "@/store/ollamaStore";

const PERSONALITIES = [
  { id: "gentle",    label: "Gentle",    desc: "Warm, patient, softly encouraging" },
  { id: "playful",   label: "Playful",   desc: "Light-hearted, witty, upbeat" },
  { id: "calm",      label: "Calm",      desc: "Steady, measured, quietly supportive" },
  { id: "energetic", label: "Energetic", desc: "Enthusiastic, lively, motivating" },
];

const PIPER_VOICES = [
  { id: "en_US-amy-medium",    label: "Amy (US Female, warm)" },
  { id: "en_US-lessac-medium", label: "Lessac (US Female, clear)" },
  { id: "en_GB-alan-medium",   label: "Alan (UK Male, calm)" },
  { id: "en_US-ryan-medium",   label: "Ryan (US Male, friendly)" },
];

interface Props {
  open: boolean;
  onClose: () => void;
}

export function SettingsPanel({ open, onClose }: Props) {
  const { settings, update, load } = useSettingsStore();
  const models = useOllamaStore((s) => s.models);
  const [localEndpoint, setLocalEndpoint] = useState(settings.endpoint);

  useEffect(() => { load(); }, [load]);
  useEffect(() => { setLocalEndpoint(settings.endpoint); }, [settings.endpoint]);

  if (!open) return null;

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 100,
        display: "flex",
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: "320px",
          height: "100%",
          background: "var(--bg-surface)",
          borderRight: "1px solid var(--text-dim)",
          overflowY: "auto",
          padding: "20px 16px",
          display: "flex",
          flexDirection: "column",
          gap: "24px",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <h2 style={{ fontSize: "16px", fontWeight: 600 }}>Settings</h2>
          <button onClick={onClose} style={btnStyle}>✕</button>
        </div>

        {/* Companion */}
        <Section title="Companion">
          <Label>Name</Label>
          <Input
            value={settings.companion_name}
            onChange={(v) => update("companion_name", v)}
          />
          <Label>Personality</Label>
          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            {PERSONALITIES.map((p) => (
              <button
                key={p.id}
                onClick={() => update("personality", p.id)}
                style={{
                  padding: "8px 12px",
                  borderRadius: "8px",
                  border: `1px solid ${settings.personality === p.id ? "var(--accent)" : "var(--text-dim)"}`,
                  background: settings.personality === p.id ? "rgba(167,139,250,0.12)" : "transparent",
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
        </Section>

        {/* Voice */}
        <Section title="Voice">
          <Label>Voice preset</Label>
          <select
            value={settings.piper_voice}
            onChange={(e) => update("piper_voice", e.target.value)}
            style={selectStyle}
          >
            {PIPER_VOICES.map((v) => (
              <option key={v.id} value={v.id}>{v.label}</option>
            ))}
          </select>
          <Label>Piper binary path</Label>
          <Input
            value={settings.piper_binary}
            onChange={(v) => update("piper_binary", v)}
            placeholder="/usr/local/bin/piper"
          />
          <p style={{ fontSize: "11px", color: "var(--text-muted)" }}>
            Leave empty if piper is on your PATH. Voice cloning (XTTS-v2) arrives in M6.
          </p>
        </Section>

        {/* Model */}
        <Section title="Local LLM">
          <Label>Ollama endpoint</Label>
          <div style={{ display: "flex", gap: "6px" }}>
            <input
              value={localEndpoint}
              onChange={(e) => setLocalEndpoint(e.target.value)}
              onBlur={() => update("endpoint", localEndpoint)}
              style={{ ...inputStyle, flex: 1 }}
            />
          </div>
          <Label>Model</Label>
          {models.length > 0 ? (
            <select
              value={settings.model}
              onChange={(e) => update("model", e.target.value)}
              style={selectStyle}
            >
              {models.map((m) => (
                <option key={m.name} value={m.name}>{m.name}</option>
              ))}
            </select>
          ) : (
            <Input
              value={settings.model}
              onChange={(v) => update("model", v)}
              placeholder="llama3.2:8b"
            />
          )}
        </Section>

        <div style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "auto" }}>
          VoicePartner v0.1.0 · AGPL-3.0 · All data stays on your machine
        </div>
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
      <h3 style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.06em" }}>
        {title}
      </h3>
      {children}
    </div>
  );
}

function Label({ children }: { children: React.ReactNode }) {
  return <label style={{ fontSize: "13px", color: "var(--text-primary)", marginTop: "4px" }}>{children}</label>;
}

function Input({ value, onChange, placeholder }: { value: string; onChange: (v: string) => void; placeholder?: string }) {
  return (
    <input
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      style={inputStyle}
    />
  );
}

const inputStyle: React.CSSProperties = {
  background: "var(--bg-elevated)",
  border: "1px solid var(--text-dim)",
  borderRadius: "6px",
  padding: "7px 10px",
  color: "var(--text-primary)",
  fontSize: "13px",
  outline: "none",
  width: "100%",
};

const selectStyle: React.CSSProperties = {
  ...inputStyle,
  cursor: "pointer",
};

const btnStyle: React.CSSProperties = {
  background: "transparent",
  border: "none",
  color: "var(--text-muted)",
  fontSize: "16px",
  cursor: "pointer",
  padding: "4px 6px",
  borderRadius: "4px",
};
