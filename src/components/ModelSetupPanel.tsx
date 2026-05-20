import { useState, useEffect, useCallback } from "react";
import { useSettingsStore } from "@/store/settingsStore";
import {
  checkSetup,
  downloadTool,
  pickFile,
  onDownloadProgress,
  onDownloadDone,
  onDownloadError,
  type SetupStatus,
  type DownloadProgress,
} from "@/lib/tauri";

interface Props {
  onDone: () => void;
}

type ToolId = "piper_windows" | "piper_voice_amy" | "whisper_model_base_en";

interface ToolState {
  downloading: boolean;
  progress: DownloadProgress | null;
  error: string | null;
  done: boolean;
}

type SetupBoolKey = "piper_ok" | "piper_voice_ok" | "whisper_model_ok";

const TOOLS: Array<{
  id: ToolId;
  label: string;
  desc: string;
  size: string;
  okKey: SetupBoolKey;
}> = [
  {
    id: "piper_windows",
    label: "Piper TTS binary",
    desc: "Text-to-speech engine (Windows)",
    size: "~18 MB",
    okKey: "piper_ok",
  },
  {
    id: "piper_voice_amy",
    label: "Amy voice model",
    desc: "English US female voice",
    size: "~65 MB",
    okKey: "piper_voice_ok",
  },
  {
    id: "whisper_model_base_en",
    label: "Whisper base.en model",
    desc: "Speech-to-text model",
    size: "~141 MB",
    okKey: "whisper_model_ok",
  },
];

export function ModelSetupPanel({ onDone }: Props) {
  const { update } = useSettingsStore();
  const [status, setStatus] = useState<SetupStatus | null>(null);
  const [tools, setTools] = useState<Record<ToolId, ToolState>>({
    piper_windows:         { downloading: false, progress: null, error: null, done: false },
    piper_voice_amy:       { downloading: false, progress: null, error: null, done: false },
    whisper_model_base_en: { downloading: false, progress: null, error: null, done: false },
  });

  const refresh = useCallback(async () => {
    try {
      setStatus(await checkSetup());
    } catch {}
  }, []);

  useEffect(() => {
    refresh();

    const subs = Promise.all([
      onDownloadProgress((p) => {
        const id = p.tool as ToolId;
        setTools((prev) => ({
          ...prev,
          [id]: { ...prev[id], progress: p },
        }));
      }),
      onDownloadDone(async (d) => {
        const id = d.tool as ToolId;
        setTools((prev) => ({
          ...prev,
          [id]: { ...prev[id], downloading: false, done: true, progress: null },
        }));
        // Sync updated path into settings store
        if (id === "piper_windows") await update("piper_binary", d.path);
        if (id === "piper_voice_amy") await update("piper_voice", d.path);
        if (id === "whisper_model_base_en") await update("whisper_model", d.path);
        await refresh();
      }),
      onDownloadError((d) => {
        const id = d.tool as ToolId;
        setTools((prev) => ({
          ...prev,
          [id]: { ...prev[id], downloading: false, error: d.error, progress: null },
        }));
      }),
    ]);

    return () => {
      subs.then((fns) => fns.forEach((fn) => fn()));
    };
  }, [refresh, update]);

  const startDownload = async (id: ToolId) => {
    setTools((prev) => ({
      ...prev,
      [id]: { ...prev[id], downloading: true, error: null, progress: null, done: false },
    }));
    try {
      await downloadTool(id);
    } catch (e) {
      setTools((prev) => ({
        ...prev,
        [id]: { ...prev[id], downloading: false, error: String(e) },
      }));
    }
  };

  const browseBinary = async (key: "piper_binary" | "whisper_binary" | "whisper_model") => {
    const ext = key === "whisper_model" ? ["bin"] : ["exe"];
    const path = await pickFile(ext);
    if (path) await update(key, path);
    await refresh();
  };

  const allOk = status
    ? TOOLS.every((t) => status[t.okKey])
    : false;

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
          <h2 style={{ fontSize: "17px", fontWeight: 700, marginBottom: "6px" }}>
            Voice Setup
          </h2>
          <p style={{ fontSize: "12px", color: "var(--text-muted)", lineHeight: 1.6 }}>
            Download the required components for voice input and output. All files
            are stored locally on your machine.
          </p>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
          {TOOLS.map((tool) => {
            const ts = tools[tool.id];
            const ok = status?.[tool.okKey] ?? false;

            return (
              <ToolRow
                key={tool.id}
                tool={tool}
                ok={ok}
                state={ts}
                onDownload={() => startDownload(tool.id)}
                onBrowse={
                  tool.id === "piper_windows"
                    ? () => browseBinary("piper_binary")
                    : tool.id === "whisper_model_base_en"
                    ? () => browseBinary("whisper_model")
                    : undefined
                }
              />
            );
          })}
        </div>

        {/* Whisper binary (not auto-downloadable) */}
        <WhisperBinaryRow onBrowse={() => browseBinary("whisper_binary")} />

        <div style={{ display: "flex", gap: "10px" }}>
          <button type="button" onClick={onDone} style={secondaryBtn}>
            {allOk ? "Done" : "Skip for now"}
          </button>
          {allOk && (
            <button type="button" onClick={onDone} style={primaryBtn}>
              Start using voice
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

// ── Sub-components ────────────────────────────────────────────────────────────

interface ToolRowProps {
  tool: (typeof TOOLS)[number];
  ok: boolean;
  state: ToolState;
  onDownload: () => void;
  onBrowse?: () => void;
}

function ToolRow({ tool, ok, state, onDownload, onBrowse }: ToolRowProps) {
  const pct =
    state.progress && state.progress.total > 0
      ? Math.round((state.progress.downloaded / state.progress.total) * 100)
      : null;

  return (
    <div
      style={{
        background: "var(--bg-elevated)",
        borderRadius: "10px",
        padding: "12px 14px",
        border: `1px solid ${ok ? "rgba(167,139,250,0.3)" : "var(--text-dim)"}`,
        display: "flex",
        flexDirection: "column",
        gap: "8px",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
        <span style={{ fontSize: "14px" }}>{ok ? "✓" : "○"}</span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: "13px", fontWeight: 600, color: ok ? "var(--accent)" : "var(--text-primary)" }}>
            {tool.label}
          </div>
          <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
            {tool.desc} · {tool.size}
          </div>
        </div>
        {!ok && !state.downloading && (
          <div style={{ display: "flex", gap: "6px" }}>
            {onBrowse && (
              <button type="button" onClick={onBrowse} style={smallSecondaryBtn}>
                Browse
              </button>
            )}
            <button type="button" onClick={onDownload} style={smallPrimaryBtn}>
              Download
            </button>
          </div>
        )}
        {state.done && !ok && (
          <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>Saved</span>
        )}
      </div>

      {state.downloading && (
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
              ? `${pct}% — ${fmtBytes(state.progress!.downloaded)} / ${fmtBytes(state.progress!.total)}`
              : "Connecting…"}
          </div>
        </div>
      )}

      {state.error && (
        <div style={{ fontSize: "11px", color: "var(--error)", wordBreak: "break-all" }}>
          {state.error}
        </div>
      )}
    </div>
  );
}

function WhisperBinaryRow({ onBrowse }: { onBrowse: () => void }) {
  return (
    <div
      style={{
        background: "var(--bg-elevated)",
        borderRadius: "10px",
        padding: "12px 14px",
        border: "1px solid var(--text-dim)",
        display: "flex",
        alignItems: "center",
        gap: "10px",
      }}
    >
      <span style={{ fontSize: "14px" }}>○</span>
      <div style={{ flex: 1 }}>
        <div style={{ fontSize: "13px", fontWeight: 600, color: "var(--text-primary)" }}>
          whisper.cpp binary
        </div>
        <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
          Download from{" "}
          <code style={{ fontSize: "10px", background: "rgba(255,255,255,0.06)", padding: "1px 4px", borderRadius: "3px" }}>
            github.com/ggerganov/whisper.cpp/releases
          </code>
        </div>
      </div>
      <button type="button" onClick={onBrowse} style={smallSecondaryBtn}>
        Browse
      </button>
    </div>
  );
}

function fmtBytes(n: number): string {
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

// ── Styles ────────────────────────────────────────────────────────────────────

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

const smallPrimaryBtn: React.CSSProperties = {
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

const smallSecondaryBtn: React.CSSProperties = {
  background: "transparent",
  border: "1px solid var(--text-dim)",
  borderRadius: "6px",
  color: "var(--text-muted)",
  fontSize: "11px",
  padding: "5px 10px",
  cursor: "pointer",
  whiteSpace: "nowrap",
};
