import { useState, useEffect, useCallback } from "react";
import { useSettingsStore } from "@/store/settingsStore";
import {
  checkSetup,
  downloadTool,
  onDownloadProgress,
  onDownloadDone,
  onDownloadError,
  type SetupStatus,
  type DownloadProgress,
} from "@/lib/tauri";

interface Props {
  onDone: () => void;
}

export const VOICE_CATALOG = [
  { id: "en_US-amy-medium",        toolId: "piper_voice_amy",        label: "Amy",        gender: "F" as const, accent: "US Female",    desc: "Warm & friendly",   lang: "EN" },
  { id: "en_US-lessac-medium",     toolId: "piper_voice_lessac",     label: "Lessac",     gender: "F" as const, accent: "US Female",    desc: "Clear & natural",   lang: "EN" },
  { id: "en_US-ryan-medium",       toolId: "piper_voice_ryan",       label: "Ryan",       gender: "M" as const, accent: "US Male",      desc: "Friendly & smooth", lang: "EN" },
  { id: "en_GB-alan-medium",       toolId: "piper_voice_alan",       label: "Alan",       gender: "M" as const, accent: "UK Male",      desc: "Calm & articulate", lang: "EN" },
  { id: "hi_IN-priyamvada-medium", toolId: "piper_voice_priyamvada", label: "Priyamvada", gender: "F" as const, accent: "India · Hindi", desc: "Natural & expressive", lang: "HI" },
  { id: "hi_IN-pratham-medium",    toolId: "piper_voice_pratham",    label: "Pratham",    gender: "M" as const, accent: "India · Hindi", desc: "Clear & warm",      lang: "HI" },
];

interface VoiceState {
  downloading: boolean;
  progress: DownloadProgress | null;
  error: string | null;
}

export function VoiceGallery({ onDone }: Props) {
  const { settings, update } = useSettingsStore();
  const [status, setStatus] = useState<SetupStatus | null>(null);
  const [voiceState, setVoiceState] = useState<Record<string, VoiceState>>(
    () => Object.fromEntries(
      VOICE_CATALOG.map((v) => [v.toolId, { downloading: false, progress: null, error: null }])
    )
  );

  const refresh = useCallback(async () => {
    try { setStatus(await checkSetup()); } catch {}
  }, []);

  useEffect(() => {
    refresh();

    const subs = Promise.all([
      onDownloadProgress((p) => {
        if (!VOICE_CATALOG.find((v) => v.toolId === p.tool)) return;
        setVoiceState((prev) => ({ ...prev, [p.tool]: { ...prev[p.tool], progress: p } }));
      }),
      onDownloadDone(async (d) => {
        if (!VOICE_CATALOG.find((v) => v.toolId === d.tool)) return;
        setVoiceState((prev) => ({
          ...prev,
          [d.tool]: { downloading: false, progress: null, error: null },
        }));
        await refresh();
      }),
      onDownloadError((d) => {
        if (!VOICE_CATALOG.find((v) => v.toolId === d.tool)) return;
        setVoiceState((prev) => ({
          ...prev,
          [d.tool]: { ...prev[d.tool], downloading: false, error: d.error, progress: null },
        }));
      }),
    ]);

    return () => { subs.then((fns) => fns.forEach((f) => f())); };
  }, [refresh]);

  const download = async (toolId: string) => {
    setVoiceState((prev) => ({ ...prev, [toolId]: { downloading: true, progress: null, error: null } }));
    try {
      await downloadTool(toolId);
    } catch (e) {
      setVoiceState((prev) => ({ ...prev, [toolId]: { ...prev[toolId], downloading: false, error: String(e) } }));
    }
  };

  const useVoice = async (voiceId: string) => {
    const path = status?.voice_paths[voiceId];
    if (path) {
      await update("piper_voice", path);
      onDone();
    }
  };

  return (
    <div
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
          maxWidth: "440px",
          background: "var(--bg-surface)",
          borderRadius: "16px",
          padding: "24px",
          border: "1px solid var(--text-dim)",
          display: "flex",
          flexDirection: "column",
          gap: "20px",
        }}
      >
        <div>
          <h2 style={{ fontSize: "17px", fontWeight: 700, marginBottom: "6px" }}>Choose a Voice</h2>
          <p style={{ fontSize: "12px", color: "var(--text-muted)", lineHeight: 1.6 }}>
            Download and switch between different TTS voices. Each model is ~60–80 MB and stored locally.
          </p>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
          {(["EN", "HI"] as const).map((lang) => (
            <div key={lang} style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
              <div style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.06em" }}>
                {lang === "EN" ? "English Voices" : "Hindi Voices"}
              </div>
              {lang === "HI" && (
                <p style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: 1.5, margin: 0 }}>
                  These voices speak Hindi. Use them when chatting in Hindi.
                </p>
              )}
              {VOICE_CATALOG.filter((v) => v.lang === lang).map((v) => {
            const vs = voiceState[v.toolId];
            const downloaded = !!status?.voice_paths[v.id];
            const isActive = settings.piper_voice?.includes(v.id) ?? false;
            const pct =
              vs.progress && vs.progress.total > 0
                ? Math.round((vs.progress.downloaded / vs.progress.total) * 100)
                : null;

            return (
              <div
                key={v.id}
                style={{
                  background: "var(--bg-elevated)",
                  borderRadius: "10px",
                  padding: "12px 14px",
                  border: `1px solid ${isActive ? "var(--accent)" : downloaded ? "rgba(167,139,250,0.2)" : "var(--text-dim)"}`,
                  display: "flex",
                  flexDirection: "column",
                  gap: "8px",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                  {/* Gender badge */}
                  <span
                    style={{
                      width: "28px",
                      height: "28px",
                      borderRadius: "50%",
                      background: v.gender === "F" ? "rgba(236,72,153,0.15)" : "rgba(59,130,246,0.15)",
                      border: `1px solid ${v.gender === "F" ? "rgba(236,72,153,0.35)" : "rgba(59,130,246,0.35)"}`,
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      fontSize: "13px",
                      flexShrink: 0,
                      color: v.gender === "F" ? "rgba(236,72,153,0.9)" : "rgba(59,130,246,0.9)",
                    }}
                  >
                    {v.gender === "F" ? "♀" : "♂"}
                  </span>

                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div
                      style={{
                        fontSize: "13px",
                        fontWeight: 600,
                        color: isActive ? "var(--accent)" : "var(--text-primary)",
                      }}
                    >
                      {v.label}
                      {isActive && (
                        <span style={{ marginLeft: "6px", fontSize: "10px", color: "var(--accent)", opacity: 0.8 }}>
                          active
                        </span>
                      )}
                    </div>
                    <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                      {v.accent} · {v.desc}
                    </div>
                  </div>

                  {!vs.downloading && (
                    <div style={{ display: "flex", gap: "6px" }}>
                      {downloaded && !isActive && (
                        <button type="button" onClick={() => useVoice(v.id)} style={primarySmallBtn}>
                          Use
                        </button>
                      )}
                      {!downloaded && (
                        <button type="button" onClick={() => download(v.toolId)} style={secondarySmallBtn}>
                          Download
                        </button>
                      )}
                      {isActive && (
                        <span style={{ fontSize: "14px", color: "var(--accent)" }}>✓</span>
                      )}
                    </div>
                  )}
                </div>

                {vs.downloading && (
                  <div>
                    <div
                      style={{
                        height: "4px",
                        borderRadius: "2px",
                        background: "var(--text-dim)",
                        overflow: "hidden",
                      }}
                    >
                      <div
                        style={{
                          height: "100%",
                          width: pct != null ? `${pct}%` : "30%",
                          background: "var(--accent)",
                          borderRadius: "2px",
                          transition: pct != null ? "width 0.2s" : "none",
                          animation: pct == null ? "pulse 1.2s ease-in-out infinite" : "none",
                        }}
                      />
                    </div>
                    <div style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "4px" }}>
                      {pct != null
                        ? `${pct}% — ${fmtMB(vs.progress!.downloaded)} / ${fmtMB(vs.progress!.total)}`
                        : "Connecting…"}
                    </div>
                  </div>
                )}

                {vs.error && (
                  <div style={{ fontSize: "11px", color: "var(--error)", wordBreak: "break-all" }}>
                    {vs.error}
                  </div>
                )}
              </div>
            );
          })}
            </div>
          ))}
        </div>

        <button type="button" onClick={onDone} style={doneBtn}>
          Done
        </button>
      </div>
    </div>
  );
}

function fmtMB(n: number): string {
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

const primarySmallBtn: React.CSSProperties = {
  background: "var(--accent)",
  border: "none",
  borderRadius: "6px",
  color: "#fff",
  fontSize: "11px",
  fontWeight: 600,
  padding: "5px 10px",
  cursor: "pointer",
  whiteSpace: "nowrap",
};

const secondarySmallBtn: React.CSSProperties = {
  background: "transparent",
  border: "1px solid var(--text-dim)",
  borderRadius: "6px",
  color: "var(--text-muted)",
  fontSize: "11px",
  padding: "5px 10px",
  cursor: "pointer",
  whiteSpace: "nowrap",
};

const doneBtn: React.CSSProperties = {
  background: "transparent",
  border: "1px solid var(--text-dim)",
  borderRadius: "8px",
  color: "var(--text-muted)",
  fontSize: "13px",
  padding: "10px",
  cursor: "pointer",
};
