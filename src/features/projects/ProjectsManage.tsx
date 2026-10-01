import { useState } from "react";
import {
  deleteProject,
  upsertProject,
} from "../../services/api";
import type { Project } from "../../types/models";

type Props = {
  projects: Project[];
  onChanged: () => void;
};

export function ProjectsManage({ projects, onChanged }: Props) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function create() {
    const trimmed = name.trim();
    if (!trimmed || busy) return;
    setBusy(true);
    setError(null);
    try {
      await upsertProject({
        id: "",
        name: trimmed,
        createdAt: "",
      });
      setName("");
      onChanged();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Création impossible");
    } finally {
      setBusy(false);
    }
  }

  async function rename(project: Project) {
    const trimmed = editName.trim();
    if (!trimmed || busy) return;
    setBusy(true);
    setError(null);
    try {
      await upsertProject({ ...project, name: trimmed });
      setEditingId(null);
      onChanged();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Renommage impossible");
    } finally {
      setBusy(false);
    }
  }

  async function remove(project: Project) {
    if (busy) return;
    const ok = window.confirm(`Supprimer le projet « ${project.name} » ?`);
    if (!ok) return;
    setBusy(true);
    setError(null);
    try {
      await deleteProject(project.id);
      if (editingId === project.id) setEditingId(null);
      onChanged();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Suppression impossible");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="projects-manage">
      <button
        type="button"
        className="projects-manage-toggle"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
      >
        Projets
        <span aria-hidden="true">{open ? "▾" : "▸"}</span>
      </button>

      {open && (
        <div className="projects-manage-body">
          <div className="projects-create-row">
            <input
              className="projects-input"
              type="text"
              value={name}
              placeholder="Nouveau projet"
              disabled={busy}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  void create();
                }
              }}
            />
            <button
              type="button"
              className="projects-action-btn"
              disabled={busy || !name.trim()}
              onClick={() => void create()}
            >
              Créer
            </button>
          </div>

          {projects.length === 0 ? (
            <p className="panel-muted">Aucun projet — optionnel.</p>
          ) : (
            <ul className="projects-list">
              {projects.map((project) => (
                <li key={project.id} className="projects-list-item">
                  {editingId === project.id ? (
                    <>
                      <input
                        className="projects-input"
                        type="text"
                        value={editName}
                        disabled={busy}
                        onChange={(e) => setEditName(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            e.preventDefault();
                            void rename(project);
                          }
                          if (e.key === "Escape") setEditingId(null);
                        }}
                      />
                      <button
                        type="button"
                        className="projects-action-btn"
                        disabled={busy || !editName.trim()}
                        onClick={() => void rename(project)}
                      >
                        OK
                      </button>
                      <button
                        type="button"
                        className="projects-link-btn"
                        disabled={busy}
                        onClick={() => setEditingId(null)}
                      >
                        Annuler
                      </button>
                    </>
                  ) : (
                    <>
                      <span className="projects-name">{project.name}</span>
                      <button
                        type="button"
                        className="projects-link-btn"
                        disabled={busy}
                        onClick={() => {
                          setEditingId(project.id);
                          setEditName(project.name);
                        }}
                      >
                        Renommer
                      </button>
                      <button
                        type="button"
                        className="projects-link-btn is-danger"
                        disabled={busy}
                        onClick={() => void remove(project)}
                      >
                        Suppr.
                      </button>
                    </>
                  )}
                </li>
              ))}
            </ul>
          )}
          {error && <p className="panel-error">{error}</p>}
        </div>
      )}
    </div>
  );
}
