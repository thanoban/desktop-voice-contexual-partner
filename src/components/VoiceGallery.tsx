import { useState, useEffect, useCallback } from "react";
import { useSettingsStore } from "@/store/settingsStore";
import {
  checkSetup,
  downloadTool,
  pickFile,
  getSapiVoices,
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
  { id: "en_US-amy-medium",        toolId: "piper_voice_amy",        label: "Amy",        gender: "F" as const, accent: "US Female",     desc: "Warm & friendly",      lang: "EN" },
  { id: "en_US-lessac-medium",     toolId: "piper_voice_lessac",     label: "Lessac",     gender: "F" as const, accent: "US Female",     desc: "Clear & natural",      lang: "EN" },
  { id: "en_US-ryan-medium",       toolId: "piper_voice_ryan",       label: "Ryan",       gender: "M" as const, accent: "US Male",       desc: "Friendly & smooth",    lang: "EN" },
  { id: "en_GB-alan-medium",       toolId: "piper_voice_alan",       label: "Alan",       gender: "M" as const, accent: "UK Male",       desc: "Calm & articulate",    lang: "EN" },
  { id: "hi_IN-priyamvada-medium", toolId: "piper_voice_priyamvada", label: "Priyamvada", gender: "F" as const, accent: "India · Hindi", desc: "Natural & expressive", lang: "HI" },
  { id: "hi_IN-pratham-medium",    toolId: "piper_voice_pratham",    label: "Pratham",    gender: "M" as const, accent: "India · Hindi", desc: "Clear & warm",         lang: "HI" },
];

interface CustomVoice {
  label: string;
  path: string;
}

function parseCustomVoices(raw: string): CustomVoice[] {
  try { return JSON.parse(raw || "[]"); } catch { return []; }
}

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
  const [addForm, setAddForm] = useState<{ path: string; label: string } | null>(null);
  const [sapiVoices, setSapiVoices] = useState<string[]>([]);

  const refresh = useCallback(async () => {
    try { setStatus(await checkSetup()); } catch {}
  }, []);

  useEffect(() => {
    getSapiVoices().then(setSapiVoices).catch(() => {});
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

  const useBuiltinVoice = async (voiceId: string) => {
    const path = status?.voice_paths[voiceId];
    if (path) { await update("piper_voice", path); onDone(); }
  };

  const useCustomVoice = async (path: string) => {
    await update("piper_voice", path);
    onDone();
  };

  // Custom voice management
  const startAdd = async () => {
    const path = await pickFile(["onnx"]);
    if (!path) return;
    const defaultLabel = path.split(/[\\/]/).pop()?.replace(/\.onnx$/i, "") ?? "Custom Voice";
    setAddForm({ path, label: defaultLabel });
  };

  const saveCustomVoice = async () => {
    if (!addForm?.path || !addForm.label.trim()) return;
    const existing = parseCustomVoices(settings.custom_voices);
    const updated = [...existing, { label: addForm.label.trim(), path: addForm.path }];
    await update("custom_voices", JSON.stringify(updated));
    setAddForm(null);
  };

  const removeCustomVoice = async (path: string) => {
    const existing = parseCustomVoices(settings.custom_voices);
    const updated = existing.filter((v) => v.path !== path);
    await update("custom_voices", JSON.stringify(updated));
    // If removed voice was active, clear piper_voice
    if (settings.piper_voice === path) await update("piper_voice", "");
  };

  const customVoices = parseCustomVoices(settings.custom_voices);

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
          maxHeight: "calc(100vh - 48px)",
          background: "var(--bg-surface)",
          borderRadius: "16px",
          padding: "24px",
          border: "1px solid var(--text-dim)",
          display: "flex",
          flexDirection: "column",
          gap: "20px",
          overflowY: "auto",
        }}
      >
        <div>
          <h2 style={{ fontSize: "17px", fontWeight: 700, marginBottom: "6px" }}>Choose a Voice</h2>
          <p style={{ fontSize: "12px", color: "var(--text-muted)", lineHeight: 1.6 }}>
            Download built-in voices or add your own .onnx models.
          </p>
        </div>

        {/* Built-in voices grouped by language */}
        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
          {(["EN", "HI"] as const).map((lang) => (
            <div key={lang} style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
              <div style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.06em" }}>
                {lang === "EN" ? "English Voices" : "Hindi Voices"}
              </div>
              {lang === "HI" && (
                <p style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: 1.5, margin: 0 }}>
                  These voices speak Hindi — best used when chatting in Hindi.
                </p>
              )}
              {VOICE_CATALOG.filter((v) => v.lang === lang).map((v) => {
                const vs = voiceState[v.toolId];
                const downloaded = !!status?.voice_paths[v.id];
                const isActive = settings.piper_voice?.includes(v.id) ?? false;
                const pct = vs.progress && vs.progress.total > 0
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
                      <span
                        style={{
                          width: "28px", height: "28px", borderRadius: "50%", flexShrink: 0,
                          background: v.gender === "F" ? "rgba(236,72,153,0.15)" : "rgba(59,130,246,0.15)",
                          border: `1px solid ${v.gender === "F" ? "rgba(236,72,153,0.35)" : "rgba(59,130,246,0.35)"}`,
                          display: "flex", alignItems: "center", justifyContent: "center",
                          fontSize: "13px",
                          color: v.gender === "F" ? "rgba(236,72,153,0.9)" : "rgba(59,130,246,0.9)",
                        }}
                      >
                        {v.gender === "F" ? "♀" : "♂"}
                      </span>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ fontSize: "13px", fontWeight: 600, color: isActive ? "var(--accent)" : "var(--text-primary)" }}>
                          {v.label}
                          {isActive && <span style={{ marginLeft: "6px", fontSize: "10px", opacity: 0.8 }}>active</span>}
                        </div>
                        <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>{v.accent} · {v.desc}</div>
                      </div>
                      {!vs.downloading && (
                        <div style={{ display: "flex", gap: "6px" }}>
                          {downloaded && !isActive && (
                            <button type="button" onClick={() => useBuiltinVoice(v.id)} style={primarySmallBtn}>Use</button>
                          )}
                          {!downloaded && (
                            <button type="button" onClick={() => download(v.toolId)} style={secondarySmallBtn}>Download</button>
                          )}
                          {isActive && <span style={{ fontSize: "14px", color: "var(--accent)" }}>✓</span>}
                        </div>
                      )}
                    </div>

                    {vs.downloading && (
                      <div>
                        <div style={{ height: "4px", borderRadius: "2px", background: "var(--text-dim)", overflow: "hidden" }}>
                          <div style={{
                            height: "100%",
                            width: pct != null ? `${pct}%` : "30%",
                            background: "var(--accent)",
                            borderRadius: "2px",
                            transition: pct != null ? "width 0.2s" : "none",
                            animation: pct == null ? "pulse 1.2s ease-in-out infinite" : "none",
                          }} />
                        </div>
                        <div style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "4px" }}>
                          {pct != null
                            ? `${pct}% — ${fmtMB(vs.progress!.downloaded)} / ${fmtMB(vs.progress!.total)}`
                            : "Connecting…"}
                        </div>
                      </div>
                    )}
                    {vs.error && <div style={{ fontSize: "11px", color: "var(--error)", wordBreak: "break-all" }}>{vs.error}</div>}
                  </div>
                );
              })}
            </div>
          ))}
        </div>

        {/* Windows SAPI voices — shown only when system voices are available */}
        {sapiVoices.length > 0 && (
          <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
            <div style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.06em" }}>
              Windows Voices
            </div>
            <p style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: 1.5, margin: 0 }}>
              System voices installed on your PC. Indian English voices (Heera, Ravi) appear here if
              you have the Windows India language pack installed.
            </p>
            {sapiVoices.map((name) => {
              const key = `sapi:${name}`;
              const isActive = settings.piper_voice === key;
              const isIndian = /india|heera|ravi|hindi/i.test(name);
              return (
                <div
                  key={name}
                  style={{
                    background: "var(--bg-elevated)",
                    borderRadius: "10px",
                    padding: "10px 14px",
                    border: `1px solid ${isActive ? "var(--accent)" : isIndian ? "rgba(255,153,0,0.3)" : "var(--text-dim)"}`,
                    display: "flex",
                    alignItems: "center",
                    gap: "10px",
                  }}
                >
                  <span style={{ fontSize: "16px", flexShrink: 0 }}>{isIndian ? "🇮🇳" : "🖥"}</span>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontSize: "13px", fontWeight: 600, color: isActive ? "var(--accent)" : "var(--text-primary)" }}>
                      {name}
                      {isActive && <span style={{ marginLeft: "6px", fontSize: "10px", opacity: 0.8 }}>active</span>}
                    </div>
                    <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                      {isIndian ? "Indian English · Windows SAPI" : "Windows SAPI"}
                    </div>
                  </div>
                  {!isActive && (
                    <button type="button" onClick={() => useCustomVoice(key)} style={primarySmallBtn}>
                      Use
                    </button>
                  )}
                  {isActive && <span style={{ fontSize: "14px", color: "var(--accent)" }}>✓</span>}
                </div>
              );
            })}
            <p style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: 1.5, margin: 0 }}>
              To add Indian English voices: Windows Settings → Time &amp; Language → Language &amp; Region → Add a language → English (India).
            </p>
          </div>
        )}

        {/* Custom voices section */}
        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          <div style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.06em" }}>
            Custom Voices
          </div>
          <p style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: 1.5, margin: 0 }}>
            Add any Piper-compatible .onnx voice model. Find more at{" "}
            <a href="https://huggingface.co/rhasspy/piper-voices" target="_blank" rel="noreferrer noopener" style={{ color: "var(--accent)" }}>
              huggingface.co/rhasspy/piper-voices
            </a>
          </p>

          {customVoices.map((cv) => {
            const isActive = settings.piper_voice === cv.path;
            return (
              <div
                key={cv.path}
                style={{
                  background: "var(--bg-elevated)",
                  borderRadius: "10px",
                  padding: "10px 14px",
                  border: `1px solid ${isActive ? "var(--accent)" : "rgba(167,139,250,0.2)"}`,
                  display: "flex",
                  alignItems: "center",
                  gap: "10px",
                }}
              >
                <span style={{ fontSize: "16px", flexShrink: 0 }}>🎙</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: "13px", fontWeight: 600, color: isActive ? "var(--accent)" : "var(--text-primary)" }}>
                    {cv.label}
                    {isActive && <span style={{ marginLeft: "6px", fontSize: "10px", opacity: 0.8 }}>active</span>}
                  </div>
                  <div style={{ fontSize: "10px", color: "var(--text-muted)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {cv.path}
                  </div>
                </div>
                <div style={{ display: "flex", gap: "6px", flexShrink: 0 }}>
                  {!isActive && (
                    <button type="button" onClick={() => useCustomVoice(cv.path)} style={primarySmallBtn}>Use</button>
                  )}
                  {isActive && <span style={{ fontSize: "14px", color: "var(--accent)" }}>✓</span>}
                  <button
                    type="button"
                    onClick={() => removeCustomVoice(cv.path)}
                    style={{ ...secondarySmallBtn, color: "var(--error)", borderColor: "var(--error)" }}
                    title="Remove"
                  >
                    ✕
                  </button>
                </div>
              </div>
            );
          })}

          {/* Add form — shown after file is picked */}
          {addForm && (
            <div style={{ background: "var(--bg-elevated)", borderRadius: "10px", padding: "12px 14px", border: "1px solid var(--accent)", display: "flex", flexDirection: "column", gap: "8px" }}>
              <div style={{ fontSize: "11px", color: "var(--text-muted)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                📄 {addForm.path.split(/[\\/]/).pop()}
              </div>
              <input
                value={addForm.label}
                onChange={(e) => setAddForm({ ...addForm, label: e.target.value })}
                placeholder="Display name for this voice"
                onKeyDown={(e) => { if (e.key === "Enter") saveCustomVoice(); if (e.key === "Escape") setAddForm(null); }}
                autoFocus
                style={{
                  background: "var(--bg-surface)",
                  border: "1px solid var(--text-dim)",
                  borderRadius: "6px",
                  padding: "6px 10px",
                  color: "var(--text-primary)",
                  fontSize: "13px",
                  outline: "none",
                }}
              />
              <div style={{ display: "flex", gap: "6px" }}>
                <button type="button" onClick={saveCustomVoice} style={{ ...primarySmallBtn, flex: 1 }}>Save voice</button>
                <button type="button" onClick={() => setAddForm(null)} style={{ ...secondarySmallBtn, flex: 1 }}>Cancel</button>
              </div>
            </div>
          )}

          {!addForm && (
            <button type="button" onClick={startAdd} style={addVoiceBtn}>
              + Add custom voice (.onnx)
            </button>
          )}
        </div>

        <button type="button" onClick={onDone} style={doneBtn}>Done</button>
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

const addVoiceBtn: React.CSSProperties = {
  background: "transparent",
  border: "1px dashed var(--text-dim)",
  borderRadius: "8px",
  color: "var(--text-muted)",
  fontSize: "12px",
  padding: "10px",
  cursor: "pointer",
  textAlign: "center",
  width: "100%",
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
