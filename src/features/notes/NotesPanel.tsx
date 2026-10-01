import { useEffect, useRef, useState } from "react";
import { createNote, upsertNote } from "../../services/api";
import type { Note } from "../../types/models";

type Props = {
  notes: Note[];
  loadState: "loading" | "ready" | "error";
  error: string | null;
};

export function NotesPanel({ notes, loadState, error }: Props) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draftTitle, setDraftTitle] = useState("");
  const [draftBody, setDraftBody] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const bodyRef = useRef<HTMLTextAreaElement>(null);

  const selected = notes.find((n) => n.id === selectedId) ?? null;

  useEffect(() => {
    if (creating) return;
    if (selected) {
      setDraftTitle(selected.title ?? "");
      setDraftBody(selected.body);
      setSaveError(null);
    } else {
      setDraftTitle("");
      setDraftBody("");
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
        const created = await createNote(body, "note", null, title);
        setCreating(false);
        setSelectedId(created.id);
      } else {
        const updated = await upsertNote({
          ...selected,
          title: title ?? undefined,
          body,
          projectId: selected.projectId,
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
    } else {
      setSelectedId(null);
      setDraftTitle("");
      setDraftBody("");
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
          {notes.map((note) => (
            <li key={note.id}>
              <button
                type="button"
                className="notes-item"
                onClick={() => {
                  setCreating(false);
                  setSelectedId(note.id);
                }}
              >
                <span className="notes-item-kind">{note.kind === "idea" ? "idée" : "note"}</span>
                <span className="notes-item-title">
                  {note.title?.trim() || preview(note.body)}
                </span>
                {note.title?.trim() ? (
                  <span className="notes-item-preview">{preview(note.body)}</span>
                ) : null}
              </button>
            </li>
          ))}
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
          <div className="notes-editor-actions">
            <button
              type="button"
              className="notes-save-btn"
              disabled={saving || !draftBody.trim()}
              onClick={() => void save()}
            >
              {saving ? "…" : "Enregistrer"}
            </button>
            <button
              type="button"
              className="notes-cancel-btn"
              disabled={saving}
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
