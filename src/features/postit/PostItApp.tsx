/**
 * Fenêtre post-it native — contenu = Note liée (pas de duplication).
 * Tâches cochables optionnelles : tâches du même projet que la note.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  createTask,
  getNote,
  getPostit,
  listTasks,
  setTaskDone,
  upsertNote,
} from "../../services/api";
import { listenDataChanged } from "../../services/events";
import {
  postitClose,
  postitSetAlwaysOnTop,
  postitUpdateGeometry,
} from "../../services/postit";
import type { Note, PostIt, Task } from "../../types/models";
import "./postit.css";

function readPostitId(): string | null {
  const params = new URLSearchParams(window.location.search);
  return params.get("postitId");
}

export function PostItApp() {
  const postitId = useMemo(() => readPostitId(), []);
  const [postit, setPostit] = useState<PostIt | null>(null);
  const [note, setNote] = useState<Note | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [taskDraft, setTaskDraft] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const saveTimer = useRef<number | null>(null);
  const geoTimer = useRef<number | null>(null);
  const noteRef = useRef<Note | null>(null);
  noteRef.current = note;

  async function loadRelatedTasks(projectId: string | null) {
    if (!projectId) {
      setTasks([]);
      return;
    }
    const active = await listTasks({ projectId, done: false });
    const done = await listTasks({ projectId, done: true });
    setTasks([...active, ...done.slice(0, 8)]);
  }

  useEffect(() => {
    if (!postitId) {
      setError("Post-it sans identifiant.");
      return;
    }
    let cancelled = false;

    async function bootstrap() {
      try {
        const p = await getPostit(postitId!);
        if (cancelled) return;
        if (!p) {
          setError("Post-it introuvable.");
          return;
        }
        setPostit(p);
        const n = await getNote(p.noteId);
        if (cancelled) return;
        if (!n) {
          setError("Note liée introuvable.");
          return;
        }
        setNote(n);
        setTitle(n.title ?? "");
        setBody(n.body);
        await loadRelatedTasks(n.projectId);
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Chargement impossible");
        }
      }
    }

    void bootstrap();

    let unlisten: (() => void) | undefined;
    void listenDataChanged((payload) => {
      const current = noteRef.current;
      if (!current) return;
      if (payload.entity === "note" && payload.id === current.id) {
        void getNote(current.id).then((n) => {
          if (!n) return;
          setNote(n);
          setTitle(n.title ?? "");
          setBody(n.body);
          void loadRelatedTasks(n.projectId);
        });
      }
      if (payload.entity === "task" && current.projectId) {
        void loadRelatedTasks(current.projectId);
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [postitId]);

  useEffect(() => {
    if (!postitId) return;
    const unsubs: Array<() => void> = [];

    async function wireWindow() {
      try {
        const win = getCurrentWindow();
        const persistGeo = () => {
          if (geoTimer.current) window.clearTimeout(geoTimer.current);
          geoTimer.current = window.setTimeout(() => {
            void (async () => {
              try {
                const pos = await win.outerPosition();
                const size = await win.outerSize();
                const scale = await win.scaleFactor();
                await postitUpdateGeometry(
                  postitId!,
                  pos.x / scale,
                  pos.y / scale,
                  size.width / scale,
                  size.height / scale,
                );
              } catch {
                /* ignore */
              }
            })();
          }, 280);
        };

        unsubs.push(await win.onMoved(persistGeo));
        unsubs.push(await win.onResized(persistGeo));
        unsubs.push(
          await win.onCloseRequested(async (event) => {
            event.preventDefault();
            try {
              const pos = await win.outerPosition();
              const size = await win.outerSize();
              const scale = await win.scaleFactor();
              await postitUpdateGeometry(
                postitId!,
                pos.x / scale,
                pos.y / scale,
                size.width / scale,
                size.height / scale,
              );
            } catch {
              /* ignore */
            }
            await postitClose(postitId!);
          }),
        );
      } catch {
        /* hors Tauri */
      }
    }

    void wireWindow();
    return () => {
      for (const u of unsubs) u();
      if (geoTimer.current) window.clearTimeout(geoTimer.current);
    };
  }, [postitId]);

  function scheduleNoteSave(nextTitle: string, nextBody: string) {
    const current = noteRef.current;
    if (!current) return;
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      void (async () => {
        setSaving(true);
        try {
          const updated = await upsertNote({
            ...current,
            title: nextTitle.trim() || undefined,
            body: nextBody,
          });
          setNote(updated);
          setError(null);
        } catch (err) {
          setError(err instanceof Error ? err.message : "Enregistrement impossible");
        } finally {
          setSaving(false);
        }
      })();
    }, 400);
  }

  async function toggleAlwaysOnTop() {
    if (!postit) return;
    try {
      const updated = await postitSetAlwaysOnTop(postit.id, !postit.alwaysOnTop);
      setPostit(updated);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Always-on-top impossible");
    }
  }

  async function toggleTask(task: Task) {
    try {
      await setTaskDone(task.id, task.status !== "done");
      if (note) await loadRelatedTasks(note.projectId);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Mise à jour tâche impossible");
    }
  }

  async function addTask() {
    const value = taskDraft.trim();
    if (!value || !note) return;
    try {
      await createTask(value, note.projectId);
      setTaskDraft("");
      await loadRelatedTasks(note.projectId);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Création tâche impossible");
    }
  }

  if (error && !note) {
    return (
      <main className="postit-root">
        <p className="postit-error">{error}</p>
      </main>
    );
  }

  return (
    <main className="postit-root">
      <header className="postit-header">
        <input
          className="postit-title"
          value={title}
          placeholder="Sans titre"
          onChange={(e) => {
            const next = e.target.value;
            setTitle(next);
            scheduleNoteSave(next, body);
          }}
        />
        <label className="postit-aot">
          <input
            type="checkbox"
            checked={postit?.alwaysOnTop ?? true}
            onChange={() => void toggleAlwaysOnTop()}
          />
          <span>Top</span>
        </label>
      </header>

      <textarea
        className="postit-body"
        value={body}
        placeholder="Contenu de la note…"
        onChange={(e) => {
          const next = e.target.value;
          setBody(next);
          scheduleNoteSave(title, next);
        }}
      />

      <section className="postit-tasks">
        <div className="postit-tasks-head">
          <span>Tâches</span>
          {saving && <span className="postit-saving">…</span>}
        </div>
        {!note?.projectId && (
          <p className="postit-hint">
            Assigne un projet à la note pour y rattacher des tâches cochables.
          </p>
        )}
        {note?.projectId && (
          <>
            <ul className="postit-task-list">
              {tasks.map((task) => (
                <li key={task.id}>
                  <label className={`postit-task ${task.status === "done" ? "is-done" : ""}`}>
                    <input
                      type="checkbox"
                      checked={task.status === "done"}
                      onChange={() => void toggleTask(task)}
                    />
                    <span>{task.title}</span>
                  </label>
                </li>
              ))}
            </ul>
            <div className="postit-task-add">
              <input
                type="text"
                value={taskDraft}
                placeholder="Nouvelle tâche…"
                onChange={(e) => setTaskDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    void addTask();
                  }
                }}
              />
              <button type="button" onClick={() => void addTask()} disabled={!taskDraft.trim()}>
                +
              </button>
            </div>
          </>
        )}
      </section>
      {error && note && <p className="postit-error">{error}</p>}
    </main>
  );
}
