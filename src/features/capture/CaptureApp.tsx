/**
 * Capture rapide — création directe Task / Note / Idée.
 */

import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createNote, createTask, listProjects } from "../../services/api";
import { captureHide } from "../../services/capture";
import type { Project } from "../../types/models";
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
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const closingRef = useRef(false);
  const busyRef = useRef(false);

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
      // Délai : le <select> projet fait perdre le focus sous Windows.
      const unFocus = await win.onFocusChanged(({ payload: focused }) => {
        if (focused) {
          if (blurTimer !== null) {
            window.clearTimeout(blurTimer);
            blurTimer = null;
          }
          return;
        }
        if (busyRef.current) return;
        blurTimer = window.setTimeout(() => {
          blurTimer = null;
          if (!busyRef.current) void closeCapture();
        }, 280);
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
      if (e.key === "Escape") {
        e.preventDefault();
        void closeCapture();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  async function submit() {
    const value = text.trim();
    if (!value || busy) return;
    setBusy(true);
    busyRef.current = true;
    setError(null);
    try {
      if (kind === "task") {
        await createTask(value, projectId);
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
        disabled={busy}
        placeholder="Écris, puis Entrée — Échap pour fermer"
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
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
              onClick={() => setKind(value)}
              disabled={busy}
            >
              {label}
            </button>
          ))}
        </div>
        <button
          type="button"
          className="capture-submit"
          disabled={busy || !text.trim()}
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => void submit()}
        >
          {busy ? "…" : "Entrée"}
        </button>
      </div>
      {kind === "task" && (
        <div className="capture-project-row">
          <span className="capture-project-label">Projet</span>
          <ProjectSelect
            projects={projects}
            value={projectId}
            disabled={busy}
            onChange={setProjectId}
            ariaLabel="Projet de la tâche (optionnel)"
          />
          <span className="capture-project-hint">optionnel</span>
        </div>
      )}
      <p className="capture-hint">Échap ou ✕ pour fermer sans créer</p>
      {error && <p className="capture-error">{error}</p>}
    </main>
  );
}
