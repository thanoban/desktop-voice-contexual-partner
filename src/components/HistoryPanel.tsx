import { useEffect, useState } from "react";
import { listSessions, getSessionTurns, type SessionInfo, type TurnInfo } from "@/lib/tauri";

interface Props {
  open: boolean;
  onClose: () => void;
}

function formatDate(ms: number): string {
  const d = new Date(ms);
  const now = new Date();
  const diffDays = Math.floor((now.getTime() - d.getTime()) / 86_400_000);
  if (diffDays === 0) return "Today";
  if (diffDays === 1) return "Yesterday";
  if (diffDays < 7) return `${diffDays} days ago`;
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

function formatTime(ms: number): string {
  return new Date(ms).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

export function HistoryPanel({ open, onClose }: Props) {
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [turns, setTurns] = useState<TurnInfo[]>([]);
  const [loadingTurns, setLoadingTurns] = useState(false);

  useEffect(() => {
    if (open) {
      listSessions().then(setSessions).catch(() => setSessions([]));
      setSelectedId(null);
      setTurns([]);
    }
  }, [open]);

  const selectSession = async (id: string) => {
    setSelectedId(id);
    setLoadingTurns(true);
    try {
      const t = await getSessionTurns(id);
      setTurns(t);
    } catch {
      setTurns([]);
    } finally {
      setLoadingTurns(false);
    }
  };

  if (!open) return null;

  // Group sessions by date label
  const groups: { label: string; items: SessionInfo[] }[] = [];
  for (const s of sessions) {
    const label = formatDate(s.started_at);
    const last = groups[groups.length - 1];
    if (last && last.label === label) {
      last.items.push(s);
    } else {
      groups.push({ label, items: [s] });
    }
  }

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 200,
        display: "flex",
      }}
      onClick={onClose}
    >
      {/* Backdrop */}
      <div style={{ flex: 1 }} />

      {/* Panel */}
      <div
        style={{
          width: "420px",
          height: "100%",
          background: "var(--bg-surface)",
          borderLeft: "1px solid var(--text-dim)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          style={{
            padding: "14px 16px",
            borderBottom: "1px solid var(--text-dim)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            {selectedId && (
              <button
                type="button"
                onClick={() => { setSelectedId(null); setTurns([]); }}
                style={iconBtn}
                aria-label="Back to sessions"
                title="Back"
              >
                ←
              </button>
            )}
            <span style={{ fontWeight: 600, fontSize: "14px", color: "var(--text-primary)" }}>
              {selectedId ? "Conversation" : "History"}
            </span>
          </div>
          <button type="button" onClick={onClose} style={iconBtn} aria-label="Close history">
            ✕
          </button>
        </div>

        {/* Content */}
        <div style={{ flex: 1, overflowY: "auto", padding: "8px 0" }}>

          {/* Session list */}
          {!selectedId && (
            sessions.length === 0 ? (
              <div style={{ padding: "40px 20px", textAlign: "center", color: "var(--text-muted)", fontSize: "13px" }}>
                No conversations yet.
              </div>
            ) : (
              groups.map((g) => (
                <div key={g.label}>
                  <div style={{
                    padding: "8px 16px 4px",
                    fontSize: "10px",
                    fontWeight: 600,
                    color: "var(--text-muted)",
                    textTransform: "uppercase",
                    letterSpacing: "0.07em",
                  }}>
                    {g.label}
                  </div>
                  {g.items.map((s) => (
                    <button
                      key={s.id}
                      type="button"
                      onClick={() => selectSession(s.id)}
                      style={{
                        width: "100%",
                        textAlign: "left",
                        padding: "10px 16px",
                        background: "transparent",
                        border: "none",
                        borderBottom: "1px solid var(--text-dim)",
                        cursor: "pointer",
                        display: "flex",
                        flexDirection: "column",
                        gap: "3px",
                      }}
                    >
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                        <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                          {formatTime(s.started_at)}
                        </span>
                        <span style={{
                          fontSize: "10px",
                          color: "var(--text-muted)",
                          background: "var(--bg-elevated)",
                          borderRadius: "4px",
                          padding: "1px 6px",
                        }}>
                          {s.turn_count} turns
                        </span>
                      </div>
                      <div style={{
                        fontSize: "13px",
                        color: "var(--text-primary)",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}>
                        {s.preview || <span style={{ color: "var(--text-muted)", fontStyle: "italic" }}>No messages</span>}
                      </div>
                    </button>
                  ))}
                </div>
              ))
            )
          )}

          {/* Turn view */}
          {selectedId && (
            loadingTurns ? (
              <div style={{ padding: "40px 20px", textAlign: "center", color: "var(--text-muted)", fontSize: "13px" }}>
                Loading…
              </div>
            ) : turns.length === 0 ? (
              <div style={{ padding: "40px 20px", textAlign: "center", color: "var(--text-muted)", fontSize: "13px" }}>
                No messages in this conversation.
              </div>
            ) : (
              <div style={{ padding: "12px 16px", display: "flex", flexDirection: "column", gap: "12px" }}>
                {turns.map((t, i) => (
                  <div
                    key={i}
                    style={{
                      display: "flex",
                      flexDirection: "column",
                      gap: "4px",
                      alignItems: t.role === "user" ? "flex-end" : "flex-start",
                    }}
                  >
                    <div style={{
                      maxWidth: "88%",
                      background: t.role === "user"
                        ? "rgba(167,139,250,0.15)"
                        : "var(--bg-elevated)",
                      border: `1px solid ${t.role === "user" ? "rgba(167,139,250,0.3)" : "var(--text-dim)"}`,
                      borderRadius: t.role === "user" ? "14px 14px 4px 14px" : "14px 14px 14px 4px",
                      padding: "9px 13px",
                      fontSize: "13px",
                      color: "var(--text-primary)",
                      lineHeight: 1.55,
                      whiteSpace: "pre-wrap",
                      wordBreak: "break-word",
                    }}>
                      {t.content}
                    </div>
                    <div style={{ fontSize: "10px", color: "var(--text-muted)", padding: "0 4px" }}>
                      {formatTime(t.created_at)}
                    </div>
                  </div>
                ))}
              </div>
            )
          )}
        </div>
      </div>
    </div>
  );
}

const iconBtn: React.CSSProperties = {
  background: "transparent",
  border: "none",
  color: "var(--text-muted)",
  fontSize: "16px",
  cursor: "pointer",
  padding: "4px 8px",
  borderRadius: "4px",
};
