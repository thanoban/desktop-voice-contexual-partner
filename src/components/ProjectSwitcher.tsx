import { useEffect, useState, type FormEvent } from "react";
import { createProject, listProjects, selectProject, type ProjectInfo } from "@/lib/tauri";
import { useSettingsStore } from "@/store/settingsStore";

export function ProjectSwitcher() {
  const selected = useSettingsStore((state) => state.settings.selected_project_id);
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<"client" | "project">("project");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let disposed = false;
    void listProjects().then((items) => { if (!disposed) setProjects(items); })
      .catch((reason) => { if (!disposed) setError(String(reason)); });
    return () => { disposed = true; };
  }, []);

  const choose = async (id: string) => {
    if (pending || id === selected) return;
    setError("");
    setPending(true);
    try {
      await selectProject(id);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setPending(false);
    }
  };

  const add = async (event: FormEvent) => {
    event.preventDefault();
    if (!name.trim() || pending) return;
    setPending(true);
    setError("");
    try {
      const project = await createProject(name, kind);
      setProjects((items) => [...items, project]);
      setName("");
      setCreating(false);
      await selectProject(project.id);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setPending(false);
    }
  };

  const controlStyle = {
    borderRadius: "7px", padding: "6px 8px", border: "1px solid var(--text-dim)",
    background: "var(--bg-elevated)", color: "var(--text-primary)", fontSize: "12px",
  };

  return (
    <div style={{ padding: "8px 14px", background: "var(--bg-surface)" }}>
      <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
        <label htmlFor="project-selection" style={{ fontSize: "11px", color: "var(--text-muted)" }}>Workspace</label>
        <select id="project-selection" value={selected} disabled={pending || projects.length === 0}
          onChange={(event) => void choose(event.target.value)} style={{ ...controlStyle, flex: 1, minWidth: 0 }}>
          {projects.map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}
        </select>
        <button type="button" disabled={pending} aria-label="Create workspace" aria-expanded={creating}
          onClick={() => setCreating((value) => !value)} style={controlStyle}>+</button>
      </div>
      {creating && (
        <form onSubmit={(event) => void add(event)} style={{ display: "flex", flexWrap: "wrap", gap: "6px", marginTop: "6px" }}>
          <input autoFocus value={name} disabled={pending} maxLength={100} onChange={(event) => setName(event.target.value)}
            aria-label="Workspace name" placeholder="Workspace name" style={{ ...controlStyle, flex: 1, minWidth: "110px" }} />
          <select value={kind} disabled={pending} aria-label="Workspace type" style={controlStyle}
            onChange={(event) => setKind(event.target.value as "client" | "project")}>
            <option value="project">Project</option><option value="client">Client</option>
          </select>
          <button type="submit" disabled={pending || !name.trim()} style={controlStyle}>Create</button>
        </form>
      )}
      {error && <p role="alert" style={{ fontSize: "12px", color: "var(--error)", marginTop: "5px" }}>{error}</p>}
    </div>
  );
}
