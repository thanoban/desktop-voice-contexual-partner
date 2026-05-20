import { useEffect, useState } from "react";
import {
  listDocuments,
  deleteDocument,
  pickDocument,
  ingestDocument,
  onRagProgress,
  onRagDone,
  onRagError,
  type DocumentInfo,
  type IngestProgress,
} from "@/lib/tauri";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function DocumentPanel({ open, onClose }: Props) {
  const [docs, setDocs] = useState<DocumentInfo[]>([]);
  const [progress, setProgress] = useState<IngestProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    listDocuments().then(setDocs).catch(() => null);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const listeners = Promise.all([
      onRagProgress((p) => setProgress(p)),
      onRagDone(() => {
        setProgress(null);
        listDocuments().then(setDocs).catch(() => null);
        setError(null);
      }),
      onRagError((msg) => {
        setProgress(null);
        setError(msg);
      }),
    ]);
    return () => {
      listeners.then((fns) => fns.forEach((fn) => fn()));
    };
  }, [open]);

  if (!open) return null;

  const handleAdd = async () => {
    setError(null);
    try {
      const path = await pickDocument();
      if (!path) return;
      await ingestDocument(path);
      // progress events take over from here
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDelete = async (src: string) => {
    await deleteDocument(src);
    setDocs((d) => d.filter((x) => x.source_file !== src));
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
        zIndex: 250,
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
            <h2 style={{ fontSize: "15px", fontWeight: 600 }}>Documents</h2>
            <p style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "2px" }}>
              {docs.length} {docs.length === 1 ? "document" : "documents"} · chunks are searched alongside memories
            </p>
          </div>
          <div style={{ display: "flex", gap: "8px" }}>
            <button
              type="button"
              onClick={handleAdd}
              disabled={progress !== null}
              style={{
                background: "var(--accent)",
                border: "none",
                borderRadius: "7px",
                color: "#fff",
                fontSize: "12px",
                fontWeight: 600,
                padding: "6px 12px",
                cursor: progress ? "not-allowed" : "pointer",
                opacity: progress ? 0.5 : 1,
              }}
            >
              + Add
            </button>
            <button type="button" onClick={onClose} style={iconBtn}>✕</button>
          </div>
        </div>

        {/* Progress bar */}
        {progress && (
          <div style={{ padding: "10px 18px", background: "var(--bg-elevated)", flexShrink: 0 }}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "5px" }}>
              <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                Embedding {progress.source}…
              </span>
              <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                {progress.current}/{progress.total}
              </span>
            </div>
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
                  width: `${progress.total > 0 ? (progress.current / progress.total) * 100 : 0}%`,
                  background: "var(--accent)",
                  borderRadius: "2px",
                  transition: "width 0.2s ease",
                }}
              />
            </div>
          </div>
        )}

        {/* Error */}
        {error && (
          <div
            style={{
              padding: "10px 18px",
              background: "rgba(239,68,68,0.1)",
              borderBottom: "1px solid rgba(239,68,68,0.25)",
              fontSize: "12px",
              color: "#f87171",
              flexShrink: 0,
            }}
          >
            {error}
          </div>
        )}

        {/* List */}
        <div style={{ overflowY: "auto", flex: 1, padding: "10px 16px" }}>
          {docs.length === 0 && !progress && (
            <p style={{ fontSize: "13px", color: "var(--text-muted)", textAlign: "center", padding: "24px 0" }}>
              No documents yet. Click + Add to upload a PDF, DOCX, TXT, or MD file.
            </p>
          )}
          {docs.map((doc) => (
            <div
              key={doc.source_file}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "10px",
                padding: "10px 0",
                borderBottom: "1px solid var(--bg-elevated)",
              }}
            >
              <span style={{ fontSize: "18px", flexShrink: 0 }}>
                {fileIcon(doc.source_file)}
              </span>
              <div style={{ flex: 1, overflow: "hidden" }}>
                <p
                  style={{
                    fontSize: "13px",
                    color: "var(--text-primary)",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {doc.source_file}
                </p>
                <p style={{ fontSize: "10px", color: "var(--text-muted)", marginTop: "2px" }}>
                  {doc.chunk_count} chunks · {formatAge(doc.ingested_at)}
                </p>
              </div>
              <button
                type="button"
                onClick={() => handleDelete(doc.source_file)}
                title="Remove document"
                style={iconBtn}
              >
                ✕
              </button>
            </div>
          ))}
        </div>

        <p
          style={{
            fontSize: "10px",
            color: "var(--text-muted)",
            padding: "10px 18px",
            borderTop: "1px solid var(--text-dim)",
            flexShrink: 0,
          }}
        >
          Large documents (~50+ pages) take a few minutes to embed. App stays responsive during ingestion.
        </p>
      </div>
    </div>
  );
}

function fileIcon(name: string): string {
  const ext = name.split(".").pop()?.toLowerCase();
  if (ext === "pdf") return "📄";
  if (ext === "docx") return "📝";
  return "📃";
}

function formatAge(ms: number): string {
  const diffSec = Math.floor((Date.now() - ms) / 1000);
  if (diffSec < 3600)  return `${Math.floor(diffSec / 60)}m ago`;
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
  return `${Math.floor(diffSec / 86400)}d ago`;
}

// suppress unused variable warning from onRagDone result
declare const _: unknown;

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
