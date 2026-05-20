import { useEffect, useState } from "react";
import { getMemories, deleteMemory, forgetAll, type Memory } from "@/lib/tauri";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function MemoryPanel({ open, onClose }: Props) {
  const [memories, setMemories] = useState<Memory[]>([]);
  const [loading, setLoading] = useState(false);
  const [confirmForget, setConfirmForget] = useState(false);

  useEffect(() => {
    if (!open) return;
    setLoading(true);
    getMemories()
      .then(setMemories)
      .finally(() => setLoading(false));
  }, [open]);

  if (!open) return null;

  const handleDelete = async (id: string) => {
    await deleteMemory(id);
    setMemories((m) => m.filter((x) => x.id !== id));
  };

  const handleForgetAll = async () => {
    await forgetAll();
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
                  {formatAge(m.created_at)} · {m.memory_type.replace("_", " ")}
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
