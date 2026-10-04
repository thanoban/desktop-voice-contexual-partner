import { useEffect, useState, type ReactNode } from "react";
import { getSettings, onProjectSelected } from "@/lib/tauri";
import { useChatStore } from "@/store/chatStore";
import { useSettingsStore } from "@/store/settingsStore";

export function WorkspaceBoundary({ children }: { children: ReactNode }) {
  const [scope, setScope] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let disposed = false;
    let revision = 0;
    let unlisten: (() => void) | undefined;
    const refresh = async () => {
      const current = ++revision;
      try {
        const settings = await getSettings();
        if (disposed || current !== revision) return;
        const previous = useSettingsStore.getState().settings.selected_project_id;
        if (previous !== settings.selected_project_id) useChatStore.getState().clearMessages();
        useSettingsStore.setState({ settings, loaded: true });
        setError("");
        setScope(settings.selected_project_id);
      } catch (reason) {
        if (!disposed && current === revision) setError(String(reason));
      }
    };

    void onProjectSelected(() => {
      if (disposed) return;
      setScope(null);
      useChatStore.getState().clearMessages();
      void refresh();
    }).then((cleanup) => {
      if (disposed) { cleanup(); return; }
      unlisten = cleanup;
      void refresh();
    }).catch((reason) => { if (!disposed) setError(String(reason)); });

    return () => { disposed = true; revision++; unlisten?.(); };
  }, [attempt]);

  if (!scope || error) return (
    <div role={error ? "alert" : "status"} style={{ padding: "20px", color: "var(--text-primary)" }}>
      {error ? `Could not load workspace: ${error}` : "Loading workspace…"}
      {error && <button type="button" onClick={() => setAttempt((value) => value + 1)}>Retry</button>}
    </div>
  );
  return <div key={scope} style={{ display: "contents" }}>{children}</div>;
}
