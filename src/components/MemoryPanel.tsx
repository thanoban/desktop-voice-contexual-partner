import { useEffect, useState } from "react";
import {
  getMemoryRecords,
  deleteMemory,
  forgetAll,
  getRetentionSettings,
  setRetentionSettings,
  purgeExpiredData,
  type MemoryRecord,
  type RetentionSettings,
} from "@/lib/tauri";
import { useSettingsStore } from "@/store/settingsStore";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function MemoryPanel({ open, onClose }: Props) {
  const projectId = useSettingsStore((state) => state.settings.selected_project_id);
  const [error, setError] = useState("");
  const [memories, setMemories] = useState<MemoryRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [confirmForget, setConfirmForget] = useState(false);
  const [retention, setRetention] = useState<RetentionSettings | null>(null);
  const [draft, setDraft] = useState<RetentionSettings>({ memory_days: 0, history_days: 0 });
  const [confirmRetention, setConfirmRetention] = useState(false);
  const [retentionNote, setRetentionNote] = useState("");

  const reload = () => {
    setLoading(true);
    getMemoryRecords()
      .then(setMemories)
      .catch((reason) => setError(String(reason)))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    if (!open) return;
    reload();
    setRetentionNote("");
    setConfirmRetention(false);
    getRetentionSettings()
      .then((value) => { setRetention(value); setDraft(value); })
      .catch((reason) => setError(String(reason)));
  }, [open, projectId]);

  if (!open) return null;

  const handleDelete = async (id: string) => {
    try { await deleteMemory(id, projectId); }
    catch (reason) { setError(String(reason)); return; }
    setMemories((m) => m.filter((x) => x.id !== id));
  };

  const retentionChanged = retention !== null
    && (draft.memory_days !== retention.memory_days || draft.history_days !== retention.history_days);

  const handleApplyRetention = async () => {
    try {
      const saved = await setRetentionSettings(draft.memory_days, draft.history_days);
      setRetention(saved);
      setDraft(saved);
      const report = await purgeExpiredData();
      setRetentionNote(
        `Saved. Deleted ${report.memories_deleted} memories and ${report.sessions_deleted} conversations past the limit.`,
      );
      reload();
    } catch (reason) {
      setError(String(reason));
    }
    setConfirmRetention(false);
  };

  const handleForgetAll = async () => {
    try { await forgetAll(projectId); }
    catch (reason) { setError(String(reason)); return; }
    setMemories([]);
    setConfirmForget(false);
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
        padding: "20px",
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: "100%",
          maxWidth: "440px",
          maxHeight: "80vh",
          background: "var(--bg-surface)",
          borderRadius: "16px",
          border: "1px solid var(--text-dim)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "16px 18px",
            borderBottom: "1px solid var(--text-dim)",
            flexShrink: 0,
          }}
        >
          <div>
            <h2 style={{ fontSize: "15px", fontWeight: 600 }}>Memories</h2>
            <p style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "2px" }}>
              {memories.length} stored · built from conversation summaries
            </p>
          </div>
          <button type="button" onClick={onClose} style={iconBtn}>✕</button>
        </div>

        {/* List */}
        <div style={{ overflowY: "auto", flex: 1, padding: "12px 16px" }}>
          {error && <p role="alert" style={{ color: "var(--error)", fontSize: "12px" }}>{error}</p>}
          {loading && (
            <p style={{ fontSize: "13px", color: "var(--text-muted)", textAlign: "center", padding: "20px 0" }}>
              Loading…
            </p>
          )}
          {!loading && memories.length === 0 && (
            <p style={{ fontSize: "13px", color: "var(--text-muted)", textAlign: "center", padding: "20px 0" }}>
              No memories yet. After 4 conversation turns the companion will start building memories automatically.
            </p>
          )}
          {memories.map((m) => (
            <div
              key={m.id}
              style={{
                display: "flex",
                gap: "10px",
                alignItems: "flex-start",
                padding: "10px 0",
                borderBottom: "1px solid var(--bg-elevated)",
              }}
            >
              <div style={{ flex: 1 }}>
                <p style={{ fontSize: "12px", color: "var(--text-primary)", lineHeight: 1.6 }}>
                  {m.content}
                </p>
                <p style={{ fontSize: "10px", color: "var(--text-muted)", marginTop: "4px" }}>
                  {formatAge(m.created_at)} · {m.memory_type.replace("_", " ")} ·{" "}
                  {m.embedding_model
                    ? `${m.embedding_model} (${m.embedding_dim ?? "?"}d)`
                    : "keyword search only"}
                </p>
              </div>
              <button
                type="button"
                onClick={() => handleDelete(m.id)}
                title="Delete this memory"
                style={iconBtn}
              >
                ✕
              </button>
            </div>
          ))}
        </div>

        {/* Retention */}
        <div style={{ padding: "12px 16px", borderTop: "1px solid var(--text-dim)", flexShrink: 0 }}>
          <p style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-primary)", marginBottom: "6px" }}>
            Retention <span style={{ fontWeight: 400, color: "var(--text-muted)" }}>· all workspaces · 0 = keep forever</span>
          </p>
          <div style={{ display: "flex", gap: "10px", alignItems: "center", flexWrap: "wrap" }}>
            <RetentionInput
              label="Memories (days)"
              value={draft.memory_days}
              onChange={(memory_days) => setDraft((d) => ({ ...d, memory_days }))}
            />
            <RetentionInput
              label="Conversation history (days)"
              value={draft.history_days}
              onChange={(history_days) => setDraft((d) => ({ ...d, history_days }))}
            />
            {confirmRetention ? (
              <>
                <button type="button" onClick={() => setConfirmRetention(false)} style={secondaryBtn}>
                  Cancel
                </button>
                <button type="button" onClick={handleApplyRetention} style={dangerBtn}>
                  Save and delete expired
                </button>
              </>
            ) : (
              <button
                type="button"
                disabled={!retentionChanged}
                onClick={() => setConfirmRetention(true)}
                style={{ ...secondaryBtn, opacity: retentionChanged ? 1 : 0.5 }}
              >
                Save…
              </button>
            )}
          </div>
          <p style={{ fontSize: "10px", color: "var(--text-muted)", marginTop: "6px" }}>
            {confirmRetention
              ? "Memories and conversations older than these limits are deleted now and at every startup. Documents are kept until you remove them."
              : retentionNote || "Expired items are deleted at startup. Documents are kept until you remove them."}
          </p>
        </div>

        {/* Footer */}
        {memories.length > 0 && (
          <div
            style={{
              padding: "12px 16px",
              borderTop: "1px solid var(--text-dim)",
              flexShrink: 0,
            }}
          >
            {confirmForget ? (
              <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                <span style={{ fontSize: "12px", color: "var(--text-muted)", flex: 1 }}>
                  Forget all memories permanently?
                </span>
                <button type="button" onClick={() => setConfirmForget(false)} style={secondaryBtn}>
                  Cancel
                </button>
                <button type="button" onClick={handleForgetAll} style={dangerBtn}>
                  Forget all
                </button>
              </div>
            ) : (
              <button type="button" onClick={() => setConfirmForget(true)} style={secondaryBtn}>
                Forget all memories…
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function RetentionInput({ label, value, onChange }: {
  label: string;
  value: number;
  onChange: (days: number) => void;
}) {
  return (
    <label style={{ fontSize: "11px", color: "var(--text-muted)", display: "flex", gap: "6px", alignItems: "center" }}>
      {label}
      <input
        type="number"
        min={0}
        max={36500}
        value={value}
        onChange={(e) => {
          const days = Math.floor(Number(e.target.value));
          onChange(Number.isFinite(days) ? Math.min(36500, Math.max(0, days)) : 0);
        }}
        style={{
          width: "64px",
          background: "var(--bg-elevated)",
          border: "1px solid var(--text-dim)",
          borderRadius: "6px",
          color: "var(--text-primary)",
          fontSize: "12px",
          padding: "4px 6px",
        }}
      />
    </label>
  );
}

function formatAge(createdAtMs: number): string {
  const diffSec = Math.floor((Date.now() - createdAtMs) / 1000);
  if (diffSec < 3600)  return `${Math.floor(diffSec / 60)}m ago`;
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
  return `${Math.floor(diffSec / 86400)}d ago`;
}

const iconBtn: React.CSSProperties = {
  background: "transparent",
  border: "none",
  color: "var(--text-muted)",
  fontSize: "14px",
  cursor: "pointer",
  padding: "3px 6px",
  borderRadius: "4px",
  flexShrink: 0,
};

const secondaryBtn: React.CSSProperties = {
  background: "transparent",
  border: "1px solid var(--text-dim)",
  borderRadius: "7px",
  color: "var(--text-muted)",
  fontSize: "12px",
  padding: "6px 12px",
  cursor: "pointer",
};

const dangerBtn: React.CSSProperties = {
  background: "rgba(239,68,68,0.15)",
  border: "1px solid rgba(239,68,68,0.4)",
  borderRadius: "7px",
  color: "#f87171",
  fontSize: "12px",
  padding: "6px 12px",
  cursor: "pointer",
};
