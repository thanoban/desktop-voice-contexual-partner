import { useState, useEffect } from "react";
import { useSettingsStore } from "@/store/settingsStore";
import { useOllamaStore } from "@/store/ollamaStore";
import { getAudioDevices, getMemoryCount, pickFile, type AudioDevice } from "@/lib/tauri";
import { MemoryPanel } from "@/components/MemoryPanel";
import { DocumentPanel } from "@/components/DocumentPanel";
import { ContextPermissionDialog } from "@/components/ContextPermissionDialog";
import { ModelSetupPanel } from "@/components/ModelSetupPanel";
import { VoiceGallery, VOICE_CATALOG } from "@/components/VoiceGallery";

const PERSONALITIES = [
  { id: "gentle",    label: "Gentle",    desc: "Warm, patient, softly encouraging" },
  { id: "playful",   label: "Playful",   desc: "Light-hearted, witty, upbeat" },
  { id: "calm",      label: "Calm",      desc: "Steady, measured, quietly supportive" },
  { id: "energetic", label: "Energetic", desc: "Enthusiastic, lively, motivating" },
  { id: "mentor",    label: "Mentor",    desc: "Wise, guiding, nurtures your growth" },
  { id: "caring",    label: "Caring",    desc: "Deeply empathetic, makes you feel heard" },
];

interface Props {
  open: boolean;
  onClose: () => void;
}

export function SettingsPanel({ open, onClose }: Props) {
  const { settings, update, load } = useSettingsStore();
  const models = useOllamaStore((s) => s.models);
  const [localEndpoint, setLocalEndpoint] = useState(settings.endpoint);
  const [audioDevices, setAudioDevices] = useState<AudioDevice[]>([]);
  const [memoryCount, setMemoryCount] = useState(0);
  const [memoryPanelOpen, setMemoryPanelOpen] = useState(false);
  const [documentPanelOpen, setDocumentPanelOpen] = useState(false);
  const [ctxPermOpen, setCtxPermOpen] = useState(false);
  const [setupPanelOpen, setSetupPanelOpen] = useState(false);
  const [voiceGalleryOpen, setVoiceGalleryOpen] = useState(false);

  useEffect(() => { load(); }, [load]);
  useEffect(() => { setLocalEndpoint(settings.endpoint); }, [settings.endpoint]);
  useEffect(() => {
    if (open) {
      getAudioDevices().then(setAudioDevices).catch(() => null);
      getMemoryCount().then(setMemoryCount).catch(() => null);
    }
  }, [open]);

  if (!open) return null;

  return (
    <>
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
          <button type="button" onClick={onClose} style={btnStyle}>✕</button>
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
                type="button"
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
          <Label>Custom system prompt</Label>
          <textarea
            aria-label="Custom system prompt"
            value={settings.custom_system_prompt}
            onChange={(e) => update("custom_system_prompt", e.target.value)}
            placeholder="Leave blank to use the personality preset above…"
            rows={4}
            style={{
              ...inputStyle,
              resize: "vertical",
              fontFamily: "inherit",
              lineHeight: "1.5",
            }}
          />
          <p style={{ fontSize: "11px", color: "var(--text-muted)" }}>
            Overrides the personality preset entirely when non-empty.
          </p>
        </Section>

        {/* Voice */}
        <Section title="Voice">
          <Label>Active voice</Label>
          <div style={{ display: "flex", alignItems: "center", gap: "8px", background: "var(--bg-elevated)", border: "1px solid var(--text-dim)", borderRadius: "6px", padding: "7px 10px" }}>
            <span style={{ flex: 1, fontSize: "13px", color: "var(--text-primary)" }}>
              {VOICE_CATALOG.find((v) => settings.piper_voice?.includes(v.id))?.label ?? "Custom"}
              <span style={{ marginLeft: "6px", fontSize: "11px", color: "var(--text-muted)" }}>
                {VOICE_CATALOG.find((v) => settings.piper_voice?.includes(v.id))
                  ? `· ${VOICE_CATALOG.find((v) => settings.piper_voice?.includes(v.id))!.accent}`
                  : ""}
              </span>
            </span>
            <button
              type="button"
              onClick={() => setVoiceGalleryOpen(true)}
              style={{ background: "transparent", border: "1px solid var(--text-dim)", borderRadius: "5px", color: "var(--text-muted)", fontSize: "11px", padding: "3px 8px", cursor: "pointer", whiteSpace: "nowrap" }}
            >
              Change…
            </button>
          </div>
          <Label>Piper binary path</Label>
          <PathInput
            value={settings.piper_binary}
            onChange={(v) => update("piper_binary", v)}
            placeholder="Leave empty if piper is on your PATH"
            filters={["exe"]}
          />
          <button
            type="button"
            onClick={() => setSetupPanelOpen(true)}
            style={{
              background: "transparent",
              border: "1px solid var(--text-dim)",
              borderRadius: "6px",
              color: "var(--text-muted)",
              fontSize: "12px",
              padding: "5px 10px",
              cursor: "pointer",
              alignSelf: "flex-start",
              marginTop: "2px",
            }}
          >
            Auto-download tools…
          </button>
        </Section>

        {/* Voice Tone */}
        <Section title="Voice Tone">
          <Label>Speaking speed</Label>
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <input
              type="range"
              aria-label="Speaking speed"
              min="0.5" max="2.0" step="0.05"
              value={settings.voice_speed}
              onChange={(e) => update("voice_speed", e.target.value)}
              style={{ flex: 1, accentColor: "var(--accent)" }}
            />
            <span style={{ fontSize: "12px", color: "var(--text-muted)", minWidth: "32px" }}>
              {parseFloat(settings.voice_speed).toFixed(2)}×
            </span>
          </div>
          <p style={{ fontSize: "11px", color: "var(--text-muted)" }}>
            0.5 = fast · 1.0 = normal · 2.0 = slow
          </p>
          <Label>Expressiveness</Label>
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <input
              type="range"
              aria-label="Expressiveness"
              min="0.0" max="1.0" step="0.05"
              value={settings.voice_expressiveness}
              onChange={(e) => update("voice_expressiveness", e.target.value)}
              style={{ flex: 1, accentColor: "var(--accent)" }}
            />
            <span style={{ fontSize: "12px", color: "var(--text-muted)", minWidth: "32px" }}>
              {parseFloat(settings.voice_expressiveness).toFixed(2)}
            </span>
          </div>
          <p style={{ fontSize: "11px", color: "var(--text-muted)" }}>
            0.0 = flat · 0.67 = default · 1.0 = very expressive
          </p>
        </Section>

        {/* Voice Input */}
        <Section title="Voice Input">
          <Label>Microphone</Label>
          <select
            aria-label="Microphone"
            value={settings.audio_input_device}
            onChange={(e) => update("audio_input_device", e.target.value)}
            style={selectStyle}
          >
            <option value="">System default</option>
            {audioDevices.map((d) => (
              <option key={d.name} value={d.name}>{d.name}{d.is_default ? " (default)" : ""}</option>
            ))}
          </select>
          <Label>whisper.cpp binary path</Label>
          <PathInput
            value={settings.whisper_binary}
            onChange={(v) => update("whisper_binary", v)}
            placeholder="C:\tools\whisper\main.exe"
            filters={["exe"]}
          />
          <Label>Whisper model file (.bin)</Label>
          <PathInput
            value={settings.whisper_model}
            onChange={(v) => update("whisper_model", v)}
            placeholder="C:\tools\whisper\models\ggml-base.en.bin"
            filters={["bin"]}
          />
          <label style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "13px", cursor: "pointer" }}>
            <input
              type="checkbox"
              checked={settings.window_context_auto === "true"}
              onChange={(e) => {
                if (e.target.checked) {
                  if (settings.window_context_allowed === "unset") {
                    setCtxPermOpen(true);
                  } else if (settings.window_context_allowed === "true") {
                    update("window_context_auto", "true");
                  }
                  // "false" means denied — do nothing (checkbox stays off)
                } else {
                  update("window_context_auto", "false");
                }
              }}
            />
            Auto-detect active window (inject into every message)
          </label>
          {settings.window_context_allowed === "false" && (
            <p style={{ fontSize: "11px", color: "var(--error)", marginTop: "2px" }}>
              Permission denied.{" "}
              <button
                type="button"
                onClick={() => setCtxPermOpen(true)}
                style={{ background: "none", border: "none", color: "var(--accent)", fontSize: "11px", cursor: "pointer", padding: 0 }}
              >
                Review
              </button>
            </p>
          )}
        </Section>

        {/* Model */}
        <Section title="Local LLM">
          <Label>Ollama endpoint</Label>
          <div style={{ display: "flex", gap: "6px" }}>
            <input
              aria-label="Ollama endpoint URL"
              value={localEndpoint}
              onChange={(e) => setLocalEndpoint(e.target.value)}
              onBlur={() => update("endpoint", localEndpoint)}
              style={{ ...inputStyle, flex: 1 }}
            />
          </div>
          <Label>Model</Label>
          {models.length > 0 ? (
            <select
              aria-label="LLM model"
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

        {/* Documents */}
        <Section title="Documents">
          <p style={{ fontSize: "11px", color: "var(--text-muted)" }}>
            Upload PDF, DOCX, TXT, or MD files. Chunks are embedded and searched alongside memories.
          </p>
          <button
            type="button"
            onClick={() => setDocumentPanelOpen(true)}
            style={{
              background: "transparent",
              border: "1px solid var(--text-dim)",
              borderRadius: "6px",
              color: "var(--text-muted)",
              fontSize: "12px",
              padding: "5px 10px",
              cursor: "pointer",
              alignSelf: "flex-start",
            }}
          >
            Manage documents
          </button>
        </Section>

        {/* Memory */}
        <Section title="Memory">
          <Label>Embedding model</Label>
          <Input
            value={settings.embedding_model}
            onChange={(v) => update("embedding_model", v)}
            placeholder="nomic-embed-text"
          />
          <p style={{ fontSize: "11px", color: "var(--text-muted)" }}>
            Run: <code style={{ fontSize: "10px", background: "var(--bg-elevated)", padding: "1px 4px", borderRadius: "3px" }}>ollama pull nomic-embed-text</code>
          </p>
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <span style={{ fontSize: "13px", color: "var(--text-muted)", flex: 1 }}>
              {memoryCount} {memoryCount === 1 ? "memory" : "memories"} stored
            </span>
            <button
              type="button"
              onClick={() => setMemoryPanelOpen(true)}
              style={{
                background: "transparent",
                border: "1px solid var(--text-dim)",
                borderRadius: "6px",
                color: "var(--text-muted)",
                fontSize: "12px",
                padding: "5px 10px",
                cursor: "pointer",
              }}
            >
              View memories
            </button>
          </div>
        </Section>

        <div style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "auto" }}>
          VoicePartner v0.1.0 · AGPL-3.0 · All data stays on your machine
        </div>
      </div>
    </div>
    <MemoryPanel open={memoryPanelOpen} onClose={() => setMemoryPanelOpen(false)} />
    <DocumentPanel open={documentPanelOpen} onClose={() => setDocumentPanelOpen(false)} />
    {setupPanelOpen && <ModelSetupPanel onDone={() => setSetupPanelOpen(false)} />}
    {voiceGalleryOpen && <VoiceGallery onDone={() => setVoiceGalleryOpen(false)} />}
    {ctxPermOpen && (
      <ContextPermissionDialog
        onAllow={async () => {
          await update("window_context_allowed", "true");
          await update("window_context_auto", "true");
          setCtxPermOpen(false);
        }}
        onDeny={async () => {
          await update("window_context_allowed", "false");
          setCtxPermOpen(false);
        }}
      />
    )}
    </>
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

function PathInput({ value, onChange, placeholder, filters }: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  filters: string[];
}) {
  const browse = async () => {
    const path = await pickFile(filters);
    if (path) onChange(path);
  };
  return (
    <div style={{ display: "flex", gap: "6px" }}>
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        style={{ ...inputStyle, flex: 1 }}
      />
      <button
        type="button"
        onClick={browse}
        style={{
          background: "var(--bg-elevated)",
          border: "1px solid var(--text-dim)",
          borderRadius: "6px",
          color: "var(--text-muted)",
          fontSize: "12px",
          padding: "0 10px",
          cursor: "pointer",
          whiteSpace: "nowrap",
        }}
      >
        Browse
      </button>
    </div>
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
