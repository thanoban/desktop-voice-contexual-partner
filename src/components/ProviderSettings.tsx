import { useEffect, useState } from "react";
import {
  clearProviderKey,
  configureProvider,
  getProviderSettings,
  setActiveProvider,
  setProviderKey,
  setRoutingProfile,
  type DataDestination,
  type ProviderKind,
  type ProviderSettingsView,
  type RoutingProfile,
} from "@/lib/tauri";
import { useSettingsStore } from "@/store/settingsStore";

const PROFILES: { id: RoutingProfile; label: string; desc: string }[] = [
  { id: "local", label: "Private local", desc: "Only Ollama on this PC. Remote providers are blocked." },
  { id: "hybrid", label: "Hybrid", desc: "Replies from the provider you choose; other stages stay local." },
  { id: "cloud", label: "Cloud", desc: "Replies from the remote provider you choose." },
];

const PROVIDER_LABELS: Record<ProviderKind, string> = {
  ollama: "Ollama (local)",
  openai_compatible: "OpenAI-compatible (OpenAI, OpenRouter, Groq, LM Studio…)",
  anthropic: "Anthropic",
};

/** Provider choice, credentials and routing profile with a visible data destination. */
export function ProviderSettings() {
  const [view, setView] = useState<ProviderSettingsView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pendingProfile, setPendingProfile] = useState<RoutingProfile | null>(null);
  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState("");
  const [keyDraft, setKeyDraft] = useState("");

  const active = view?.providers.find((p) => p.provider === view.active_provider) ?? null;

  // The Ollama endpoint is edited in the Local LLM section; keep its destination current.
  const ollamaEndpoint = useSettingsStore((s) => s.settings.endpoint);
  useEffect(() => {
    getProviderSettings().then(setView).catch((e) => setError(String(e)));
  }, [ollamaEndpoint]);
  useEffect(() => {
    setBaseUrl(active?.base_url ?? "");
    setModel(active?.model ?? "");
    setKeyDraft("");
  }, [active?.provider, active?.base_url, active?.model]);

  const run = async (action: () => Promise<ProviderSettingsView>) => {
    try {
      setView(await action());
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const chooseProfile = (profile: RoutingProfile) => {
    if (!view || profile === view.routing_profile) return;
    // Leaving private local mode is an explicit, disclosed choice.
    if (profile === "local") run(() => setRoutingProfile(profile));
    else setPendingProfile(profile);
  };

  const saveConfig = () => {
    if (!active || active.provider === "ollama") return;
    if (baseUrl === active.base_url && model === active.model) return;
    run(() => configureProvider(active.provider, baseUrl, model));
  };

  const saveKey = async () => {
    if (!active || !keyDraft.trim()) return;
    const key = keyDraft;
    setKeyDraft("");
    await run(() => setProviderKey(active.provider, key));
  };

  if (!view) {
    return (
      <Section title="AI provider">
        <p style={hintStyle}>{error ?? "Loading…"}</p>
      </Section>
    );
  }

  return (
    <Section title="AI provider">
      <label style={labelStyle}>Processing profile</label>
      <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
        {PROFILES.map((p) => (
          <button
            type="button"
            key={p.id}
            onClick={() => chooseProfile(p.id)}
            style={{
              ...choiceStyle,
              border: `1px solid ${view.routing_profile === p.id ? "var(--accent)" : "var(--text-dim)"}`,
              background: view.routing_profile === p.id ? "rgba(167,139,250,0.12)" : "transparent",
            }}
          >
            <span style={{ fontWeight: 500 }}>{p.label}</span>
            <span style={{ color: "var(--text-muted)", marginLeft: "6px" }}>{p.desc}</span>
          </button>
        ))}
      </div>
      {pendingProfile && (
        <div style={noticeStyle}>
          <p style={{ fontSize: "12px", marginBottom: "8px" }}>
            {pendingProfile === "hybrid" ? "Hybrid" : "Cloud"} lets the provider you select receive your messages,
            recent conversation, relevant memories and any shared window context. Remote providers may keep
            their own copies, and VoicePartner cannot delete them.
          </p>
          <div style={{ display: "flex", gap: "6px" }}>
            <button
              type="button"
              style={smallButtonStyle}
              onClick={() => {
                const profile = pendingProfile;
                setPendingProfile(null);
                run(() => setRoutingProfile(profile));
              }}
            >
              Allow {pendingProfile}
            </button>
            <button type="button" style={smallButtonStyle} onClick={() => setPendingProfile(null)}>
              Stay local
            </button>
          </div>
        </div>
      )}

      <label style={labelStyle}>Reply provider</label>
      <select
        aria-label="Reply provider"
        value={view.active_provider ?? ""}
        onChange={(e) => run(() => setActiveProvider(e.target.value as ProviderKind))}
        style={{ ...inputStyle, cursor: "pointer" }}
      >
        {view.active_provider === null && <option value="">Unknown — choose a provider</option>}
        {view.providers.map((p) => (
          <option key={p.provider} value={p.provider}>{PROVIDER_LABELS[p.provider]}</option>
        ))}
      </select>

      {active && active.provider === "ollama" && (
        <p style={hintStyle}>Set the Ollama endpoint and model in Local LLM above.</p>
      )}

      {active && active.provider !== "ollama" && (
        <>
          <label style={labelStyle}>Base URL</label>
          <input
            aria-label="Provider base URL"
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            onBlur={saveConfig}
            style={inputStyle}
          />
          <label style={labelStyle}>Model</label>
          <input
            aria-label="Provider model"
            value={model}
            onChange={(e) => setModel(e.target.value)}
            onBlur={saveConfig}
            placeholder={active.provider === "anthropic" ? "claude-sonnet-5-5" : "model id"}
            style={inputStyle}
          />
          <label style={labelStyle}>API key{active.requires_key ? "" : " (optional for local servers)"}</label>
          <div style={{ display: "flex", gap: "6px" }}>
            <input
              aria-label="Provider API key"
              type="password"
              autoComplete="off"
              spellCheck={false}
              value={keyDraft}
              onChange={(e) => setKeyDraft(e.target.value)}
              placeholder={active.has_key ? "•••••••• stored" : "Paste key"}
              style={{ ...inputStyle, flex: 1 }}
            />
            <button type="button" style={smallButtonStyle} onClick={saveKey} disabled={!keyDraft.trim()}>
              Save
            </button>
            <button
              type="button"
              style={smallButtonStyle}
              onClick={() => run(() => clearProviderKey(active.provider))}
              disabled={!active.has_key && !active.key_destination}
            >
              Clear
            </button>
          </div>
          <p style={hintStyle}>
            {active.has_key
              ? `Key stored in Windows Credential Manager for ${active.destination?.host}.`
              : active.key_destination
                ? `A key is stored for ${active.key_destination}, not for ${active.destination?.host ?? "this URL"}. It is never sent to another host.`
                : "No key stored. Keys are kept in Windows Credential Manager, never in settings."}
          </p>
          {active.destination_error && <p style={errorStyle}>{active.destination_error}</p>}
        </>
      )}

      <DestinationNotice destination={view.route.destination} />
      {!view.route.allowed && view.route.message && (
        <p style={errorStyle}>Chat is blocked: {view.route.message}</p>
      )}
      {error && <p style={errorStyle}>{error}</p>}
    </Section>
  );
}

function DestinationNotice({ destination }: { destination: DataDestination | null }) {
  if (!destination) return null;
  const detail =
    destination.locality === "local"
      ? "on this PC through Ollama."
      : destination.locality === "loopback_unverified"
        ? "by a server on this PC. VoicePartner cannot verify where that server sends data."
        : "by a remote service. Your messages, recent conversation, relevant memories and any shared window context are sent there.";
  return (
    <div style={noticeStyle} role="status">
      <p style={{ fontSize: "12px" }}>
        Data destination: <strong>{destination.host}</strong>
      </p>
      <p style={hintStyle}>Replies are generated {detail} Memory embeddings and summaries still use local Ollama.</p>
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

const labelStyle: React.CSSProperties = { fontSize: "13px", color: "var(--text-primary)", marginTop: "4px" };

const hintStyle: React.CSSProperties = { fontSize: "11px", color: "var(--text-muted)" };

const errorStyle: React.CSSProperties = { fontSize: "11px", color: "var(--error)" };

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

const choiceStyle: React.CSSProperties = {
  padding: "8px 12px",
  borderRadius: "8px",
  color: "var(--text-primary)",
  cursor: "pointer",
  textAlign: "left",
  fontSize: "13px",
};

const smallButtonStyle: React.CSSProperties = {
  background: "var(--bg-elevated)",
  border: "1px solid var(--text-dim)",
  borderRadius: "6px",
  color: "var(--text-muted)",
  fontSize: "12px",
  padding: "5px 10px",
  cursor: "pointer",
  whiteSpace: "nowrap",
};

const noticeStyle: React.CSSProperties = {
  border: "1px solid var(--text-dim)",
  borderRadius: "8px",
  padding: "8px 10px",
  background: "var(--bg-elevated)",
  display: "flex",
  flexDirection: "column",
  gap: "4px",
};
