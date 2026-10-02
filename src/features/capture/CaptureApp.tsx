/**
 * Capture rapide — création directe Task / Note / Idée.
 * Tâches : mêmes champs que Gestion (desc, priorité, échéance, statut, notes, projet).
 */

import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  createNote,
  createTask,
  listProjects,
  upsertProject,
} from "../../services/api";
import { captureHide } from "../../services/capture";
import type { Project, TaskPriority, TaskStatus } from "../../types/models";
import { ProjectSelect } from "../projects/ProjectSelect";
import "./capture.css";

type CaptureKind = "task" | "note" | "idea";

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function CaptureApp() {
  const [text, setText] = useState("");
  const [kind, setKind] = useState<CaptureKind>("task");
  const [projectId, setProjectId] = useState<string | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [description, setDescription] = useState("");
  const [priority, setPriority] = useState<TaskPriority>("medium");
  const [status, setStatus] = useState<TaskStatus>("todo");
  const [dueDate, setDueDate] = useState("");
  const [notes, setNotes] = useState("");
  const [creatingProject, setCreatingProject] = useState(false);
  const [newProjectName, setNewProjectName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const projectNameRef = useRef<HTMLInputElement>(null);
  const closingRef = useRef(false);
  const busyRef = useRef(false);
  const creatingProjectRef = useRef(false);
  const interactingRef = useRef(false);

  async function closeCapture() {
    if (closingRef.current) return;
    closingRef.current = true;
    try {
      await captureHide();
    } finally {
      closingRef.current = false;
    }
  }

  function resetDraft() {
    setText("");
    setKind("task");
    setProjectId(null);
    setDescription("");
    setPriority("medium");
    setStatus("todo");
    setDueDate("");
    setNotes("");
    setCreatingProject(false);
    creatingProjectRef.current = false;
    setNewProjectName("");
    setError(null);
    setBusy(false);
    window.setTimeout(() => inputRef.current?.focus(), 30);
  }

  async function refreshProjects() {
    try {
      setProjects(await listProjects());
    } catch {
      /* hors Tauri / ignore */
    }
  }

  function openCreateProject() {
    setCreatingProject(true);
    creatingProjectRef.current = true;
    setNewProjectName("");
    setError(null);
    window.setTimeout(() => projectNameRef.current?.focus(), 30);
  }

  function cancelCreateProject() {
    setCreatingProject(false);
    creatingProjectRef.current = false;
    setNewProjectName("");
    window.setTimeout(() => inputRef.current?.focus(), 30);
  }

  async function createProjectAndSelect() {
    const name = newProjectName.trim();
    if (!name || busy) return;
    setBusy(true);
    busyRef.current = true;
    setError(null);
    try {
      const created = await upsertProject({
        id: "",
        name,
        createdAt: "",
      });
      await refreshProjects();
      setProjectId(created.id);
      setCreatingProject(false);
      creatingProjectRef.current = false;
      setNewProjectName("");
      window.setTimeout(() => inputRef.current?.focus(), 30);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Création projet impossible");
      projectNameRef.current?.focus();
    } finally {
      setBusy(false);
      busyRef.current = false;
    }
  }

  useEffect(() => {
    let cancelled = false;
    const unsubs: Array<() => void> = [];
    let blurTimer: number | null = null;

    async function wire() {
      await refreshProjects();
      if (!inTauri()) {
        inputRef.current?.focus();
        return;
      }

      const unOpened = await listen<{ clear: boolean; kind?: CaptureKind }>(
        "capture-opened",
        (event) => {
          if (cancelled) return;
          void refreshProjects();
          if (event.payload.clear) {
            resetDraft();
            if (
              event.payload.kind === "task" ||
              event.payload.kind === "note" ||
              event.payload.kind === "idea"
            ) {
              setKind(event.payload.kind);
            }
          } else {
            inputRef.current?.focus();
          }
        },
      );
      unsubs.push(unOpened);

      const win = getCurrentWindow();
      // Délai : select / champ projet font perdre le focus sous Windows.
      const unFocus = await win.onFocusChanged(({ payload: focused }) => {
        if (focused) {
          if (blurTimer !== null) {
            window.clearTimeout(blurTimer);
            blurTimer = null;
          }
          return;
        }
        if (busyRef.current || creatingProjectRef.current || interactingRef.current) {
          return;
        }
        blurTimer = window.setTimeout(() => {
          blurTimer = null;
          if (
            !busyRef.current &&
            !creatingProjectRef.current &&
            !interactingRef.current
          ) {
            void closeCapture();
          }
        }, 320);
      });
      unsubs.push(unFocus);
    }

    void wire();
    return () => {
      cancelled = true;
      if (blurTimer !== null) window.clearTimeout(blurTimer);
      for (const u of unsubs) u();
    };
  }, []);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key !== "Escape") return;
      e.preventDefault();
      if (creatingProjectRef.current) {
        cancelCreateProject();
        return;
      }
      void closeCapture();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  async function submit() {
    const value = text.trim();
    if (!value || busy || creatingProject) return;
    setBusy(true);
    busyRef.current = true;
    setError(null);
    try {
      if (kind === "task") {
        const projectName = projectId
          ? projects.find((p) => p.id === projectId)?.name
          : undefined;
        await createTask({
          title: value,
          projectId,
          description: description.trim() || null,
          category: projectName || null,
          priority,
          dueDate: dueDate.trim() || null,
          status,
          notes: notes.trim() || null,
        });
      } else {
        await createNote(value, kind, projectId);
      }
      setText("");
      setProjectId(null);
      await closeCapture();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Échec de la capture");
      setBusy(false);
      busyRef.current = false;
      inputRef.current?.focus();
    }
  }

  function markInteracting(on: boolean) {
    interactingRef.current = on;
  }

  return (
    <main className="capture-root">
      <div className="capture-top">
        <label className="capture-prompt" htmlFor="capture-input">
          Qu&apos;est-ce que tu veux retenir ?
        </label>
        <button
          type="button"
          className="capture-close"
          aria-label="Fermer (Échap)"
          title="Fermer (Échap)"
          disabled={busy}
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => void closeCapture()}
        >
          ✕
        </button>
      </div>
      <input
        id="capture-input"
        ref={inputRef}
        className="capture-input"
        type="text"
        value={text}
        autoFocus
        disabled={busy || creatingProject}
        placeholder={
          kind === "task"
            ? "Titre de la tâche — Entrée pour créer"
            : "Écris, puis Entrée — Échap pour fermer"
        }
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && kind !== "task") {
            e.preventDefault();
            void submit();
          }
          if (e.key === "Enter" && kind === "task" && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            void submit();
          }
        }}
      />
      <div className="capture-row">
        <div className="capture-kinds" role="group" aria-label="Type">
          {(
            [
              ["task", "Tâche"],
              ["note", "Note"],
              ["idea", "Idée"],
            ] as const
          ).map(([value, label]) => (
            <button
              key={value}
              type="button"
              className={`capture-kind ${kind === value ? "is-active" : ""}`}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => {
                setKind(value);
                if (value !== "task") {
                  setCreatingProject(false);
                  creatingProjectRef.current = false;
                  setNewProjectName("");
                }
              }}
              disabled={busy}
            >
              {label}
            </button>
          ))}
        </div>
        <button
          type="button"
          className="capture-submit"
          disabled={busy || creatingProject || !text.trim()}
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => void submit()}
        >
          {busy && !creatingProject ? "…" : kind === "task" ? "Créer" : "Entrée"}
        </button>
      </div>
      {kind === "task" && (
        <>
          <textarea
            className="capture-textarea"
            value={description}
            disabled={busy || creatingProject}
            placeholder="Description (optionnel)"
            rows={2}
            onFocus={() => markInteracting(true)}
            onBlur={() => markInteracting(false)}
            onChange={(e) => setDescription(e.target.value)}
          />
          {!creatingProject && (
            <div className="capture-project-row">
              <span className="capture-project-label">Projet</span>
              <ProjectSelect
                projects={projects}
                value={projectId}
                disabled={busy}
                onChange={setProjectId}
                ariaLabel="Projet Gestion (Production ou espace de travail)"
              />
              <button
                type="button"
                className="capture-project-new"
                disabled={busy}
                onMouseDown={(e) => e.preventDefault()}
                onClick={openCreateProject}
              >
                + Nouveau
              </button>
            </div>
          )}
          {!creatingProject && projects.length === 0 && (
            <p className="capture-hint">
              Aucun projet Gestion — connecte-toi dans Paramètres, ou crée-en un.
            </p>
          )}
          {creatingProject && (
            <div className="capture-project-create">
              <input
                ref={projectNameRef}
                className="capture-project-input"
                type="text"
                value={newProjectName}
                disabled={busy}
                placeholder="Nom du projet"
                aria-label="Nom du nouveau projet"
                onChange={(e) => setNewProjectName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    e.stopPropagation();
                    void createProjectAndSelect();
                  }
                }}
              />
              <button
                type="button"
                className="capture-project-create-btn"
                disabled={busy || !newProjectName.trim()}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => void createProjectAndSelect()}
              >
                {busy ? "…" : "Créer"}
              </button>
              <button
                type="button"
                className="capture-project-cancel"
                disabled={busy}
                onMouseDown={(e) => e.preventDefault()}
                onClick={cancelCreateProject}
              >
                Annuler
              </button>
            </div>
          )}
          <div className="capture-fields">
            <label className="capture-field">
              <span>Priorité</span>
              <select
                value={priority}
                disabled={busy || creatingProject}
                onFocus={() => markInteracting(true)}
                onBlur={() => markInteracting(false)}
                onChange={(e) => setPriority(e.target.value as TaskPriority)}
              >
                <option value="low">Basse</option>
                <option value="medium">Moyenne</option>
                <option value="high">Haute</option>
              </select>
            </label>
            <label className="capture-field">
              <span>Statut</span>
              <select
                value={status}
                disabled={busy || creatingProject}
                onFocus={() => markInteracting(true)}
                onBlur={() => markInteracting(false)}
                onChange={(e) => setStatus(e.target.value as TaskStatus)}
              >
                <option value="todo">À faire</option>
                <option value="in_progress">En cours</option>
                <option value="done">Terminée</option>
              </select>
            </label>
            <label className="capture-field">
              <span>Échéance</span>
              <input
                type="date"
                value={dueDate}
                disabled={busy || creatingProject}
                onFocus={() => markInteracting(true)}
                onBlur={() => markInteracting(false)}
                onChange={(e) => setDueDate(e.target.value)}
              />
            </label>
          </div>
          <textarea
            className="capture-textarea capture-textarea-notes"
            value={notes}
            disabled={busy || creatingProject}
            placeholder="Notes (optionnel)"
            rows={2}
            onFocus={() => markInteracting(true)}
            onBlur={() => markInteracting(false)}
            onChange={(e) => setNotes(e.target.value)}
          />
        </>
      )}
      <p className="capture-hint">
        {kind === "task"
          ? "Créer ou Ctrl+Entrée — Échap pour fermer"
          : "Échap ou ✕ pour fermer sans créer"}
      </p>
      {error && <p className="capture-error">{error}</p>}
    </main>
  );
}
