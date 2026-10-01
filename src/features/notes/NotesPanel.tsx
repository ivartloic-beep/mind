import { useEffect, useRef, useState } from "react";
import { createNote, upsertNote } from "../../services/api";
import { postitOpenForNote } from "../../services/postit";
import type { Note, Project } from "../../types/models";
import { ProjectSelect } from "../projects/ProjectSelect";

type Props = {
  notes: Note[];
  projects: Project[];
  loadState: "loading" | "ready" | "error";
  error: string | null;
};

export function NotesPanel({ notes, projects, loadState, error }: Props) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draftTitle, setDraftTitle] = useState("");
  const [draftBody, setDraftBody] = useState("");
  const [draftProjectId, setDraftProjectId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [postitBusy, setPostitBusy] = useState(false);
  const bodyRef = useRef<HTMLTextAreaElement>(null);

  const selected = notes.find((n) => n.id === selectedId) ?? null;

  useEffect(() => {
    if (creating) return;
    if (selected) {
      setDraftTitle(selected.title ?? "");
      setDraftBody(selected.body);
      setDraftProjectId(selected.projectId);
      setSaveError(null);
    } else {
      setDraftTitle("");
      setDraftBody("");
      setDraftProjectId(null);
    }
  }, [selected, creating, selectedId]);

  useEffect(() => {
    if (selectedId && !notes.some((n) => n.id === selectedId) && !creating) {
      setSelectedId(null);
    }
  }, [notes, selectedId, creating]);

  async function startCreate() {
    setCreating(true);
    setSelectedId(null);
    setDraftTitle("");
    setDraftBody("");
    setDraftProjectId(null);
    setSaveError(null);
    window.setTimeout(() => bodyRef.current?.focus(), 40);
  }

  async function save() {
    const body = draftBody.trim();
    if (!body || saving) return;
    setSaving(true);
    setSaveError(null);
    try {
      const title = draftTitle.trim() || null;
      if (creating || !selected) {
        const created = await createNote(body, "note", draftProjectId, title);
        setCreating(false);
        setSelectedId(created.id);
      } else {
        const updated = await upsertNote({
          ...selected,
          title: title ?? undefined,
          body,
          projectId: draftProjectId,
        });
        setSelectedId(updated.id);
      }
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : "Enregistrement impossible");
    } finally {
      setSaving(false);
    }
  }

  function cancelEdit() {
    setCreating(false);
    setSaveError(null);
    if (selected) {
      setDraftTitle(selected.title ?? "");
      setDraftBody(selected.body);
      setDraftProjectId(selected.projectId);
    } else {
      setSelectedId(null);
      setDraftTitle("");
      setDraftBody("");
      setDraftProjectId(null);
    }
  }

  async function openAsPostit() {
    if (!selected || creating || postitBusy) return;
    setPostitBusy(true);
    setSaveError(null);
    try {
      // Enregistrer d'abord les brouillons pour que le post-it voie le contenu à jour.
      const title = draftTitle.trim() || null;
      const body = draftBody.trim();
      if (body) {
        await upsertNote({
          ...selected,
          title: title ?? undefined,
          body,
          projectId: draftProjectId,
        });
      }
      await postitOpenForNote(selected.id);
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : "Ouverture post-it impossible");
    } finally {
      setPostitBusy(false);
    }
  }

  const editing = creating || selected !== null;

  return (
    <div className="notes-board">
      <div className="notes-toolbar">
        <button type="button" className="notes-new-btn" onClick={() => void startCreate()}>
          + Note
        </button>
      </div>

      {loadState === "loading" && <p className="panel-muted">Chargement…</p>}
      {loadState === "error" && (
        <p className="panel-error">{error ?? "Erreur"}</p>
      )}

      {loadState === "ready" && !editing && notes.length === 0 && (
        <p className="panel-muted">Aucune note — capture une idée ou crée-en une.</p>
      )}

      {loadState === "ready" && !editing && notes.length > 0 && (
        <ul className="notes-list">
          {notes.map((note) => {
            const projectName = note.projectId
              ? projects.find((p) => p.id === note.projectId)?.name
              : null;
            return (
              <li key={note.id}>
                <button
                  type="button"
                  className="notes-item"
                  onClick={() => {
                    setCreating(false);
                    setSelectedId(note.id);
                  }}
                >
                  <span className="notes-item-kind">
                    {note.kind === "idea" ? "idée" : "note"}
                    {projectName ? ` · ${projectName}` : ""}
                  </span>
                  <span className="notes-item-title">
                    {note.title?.trim() || preview(note.body)}
                  </span>
                  {note.title?.trim() ? (
                    <span className="notes-item-preview">{preview(note.body)}</span>
                  ) : null}
                </button>
              </li>
            );
          })}
        </ul>
      )}

      {editing && (
        <div className="notes-editor">
          <input
            className="notes-title-input"
            type="text"
            value={draftTitle}
            placeholder="Titre (optionnel)"
            onChange={(e) => setDraftTitle(e.target.value)}
            disabled={saving}
          />
          <textarea
            ref={bodyRef}
            className="notes-body-input"
            value={draftBody}
            placeholder="Contenu…"
            rows={5}
            onChange={(e) => setDraftBody(e.target.value)}
            disabled={saving}
          />
          <label className="notes-project-row">
            <span>Projet</span>
            <ProjectSelect
              projects={projects}
              value={draftProjectId}
              disabled={saving}
              onChange={setDraftProjectId}
            />
          </label>
          <div className="notes-editor-actions">
            <button
              type="button"
              className="notes-save-btn"
              disabled={saving || !draftBody.trim()}
              onClick={() => void save()}
            >
              {saving ? "…" : "Enregistrer"}
            </button>
            {!creating && selected && (
              <button
                type="button"
                className="notes-postit-btn"
                disabled={saving || postitBusy || !draftBody.trim()}
                onClick={() => void openAsPostit()}
              >
                {postitBusy ? "…" : "Post-it"}
              </button>
            )}
            <button
              type="button"
              className="notes-cancel-btn"
              disabled={saving || postitBusy}
              onClick={cancelEdit}
            >
              {creating ? "Annuler" : "Fermer"}
            </button>
          </div>
          {saveError && <p className="panel-error">{saveError}</p>}
        </div>
      )}
    </div>
  );
}

function preview(body: string): string {
  const line = body.trim().split(/\r?\n/)[0] ?? "";
  return line.length > 72 ? `${line.slice(0, 72)}…` : line || "Sans contenu";
}
