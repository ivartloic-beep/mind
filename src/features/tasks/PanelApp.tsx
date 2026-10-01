import { useEffect, useState, useTransition } from "react";
import { listTasks, setTaskDone } from "../../services/api";
import { captureShow } from "../../services/capture";
import { listenDataChanged } from "../../services/events";
import {
  panelGetState,
  panelSetAlwaysOnTop,
  panelSetOpen,
} from "../../services/panel";
import {
  setPanelAlwaysOnTop as storeSetAot,
  setPanelOpen as storeSetOpen,
} from "../../stores/ui-store";
import type { Task } from "../../types/models";
import { TaskList } from "./TaskList";
import "./panel.css";

type LoadState = "loading" | "ready" | "error";

export function PanelApp() {
  const [open, setOpen] = useState(true);
  const [alwaysOnTop, setAlwaysOnTop] = useState(false);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [error, setError] = useState<string | null>(null);
  const [showDone, setShowDone] = useState(false);
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [, startTransition] = useTransition();

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      try {
        const state = await panelGetState();
        if (cancelled) return;
        setOpen(state.open);
        setAlwaysOnTop(state.alwaysOnTop);
        storeSetOpen(state.open);
        storeSetAot(state.alwaysOnTop);
      } catch {
        // Hors Tauri / prefs absentes — UI reste utilisable.
      }

      try {
        const rows = await listTasks();
        if (cancelled) return;
        setTasks(rows);
        setLoadState("ready");
        setError(null);
      } catch (err) {
        if (cancelled) return;
        setLoadState("error");
        setError(err instanceof Error ? err.message : "Chargement impossible");
      }
    }

    void bootstrap();

    let unlisten: (() => void) | undefined;
    void listenDataChanged((payload) => {
      if (payload.entity !== "task" && payload.entity !== "note") return;
      startTransition(() => {
        void listTasks()
          .then((rows) => {
            setTasks(rows);
            setLoadState("ready");
            setError(null);
          })
          .catch(() => {
            /* ignore refresh errors */
          });
      });
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  async function toggleOpen() {
    const next = !open;
    try {
      if (open) {
        setOpen(false);
        storeSetOpen(false);
        await new Promise((r) => window.setTimeout(r, 170));
        const state = await panelSetOpen(false);
        setOpen(state.open);
        storeSetOpen(state.open);
      } else {
        const state = await panelSetOpen(true);
        setOpen(state.open);
        storeSetOpen(state.open);
      }
    } catch {
      setOpen(next);
      storeSetOpen(next);
    }
  }

  async function toggleAlwaysOnTop() {
    const next = !alwaysOnTop;
    setAlwaysOnTop(next);
    storeSetAot(next);
    try {
      const state = await panelSetAlwaysOnTop(next);
      setAlwaysOnTop(state.alwaysOnTop);
      storeSetAot(state.alwaysOnTop);
    } catch {
      setAlwaysOnTop(!next);
      storeSetAot(!next);
    }
  }

  async function toggleTaskDone(task: Task) {
    const nextDone = task.status !== "done";
    const previous = tasks;
    setPendingId(task.id);
    setTasks((rows) =>
      rows.map((row) =>
        row.id === task.id
          ? { ...row, status: nextDone ? "done" : "active" }
          : row,
      ),
    );
    try {
      const updated = await setTaskDone(task.id, nextDone);
      setTasks((rows) => rows.map((row) => (row.id === updated.id ? updated : row)));
      if (nextDone) setShowDone(true);
    } catch {
      setTasks(previous);
    } finally {
      setPendingId(null);
    }
  }

  const activeTasks = tasks.filter((t) => t.status === "active");
  const doneTasks = tasks.filter((t) => t.status === "done");

  return (
    <div className={`panel-root ${open ? "is-open" : "is-closed"}`}>
      <button
        type="button"
        className="panel-handle"
        onClick={() => void toggleOpen()}
        aria-label={open ? "Fermer le panneau" : "Ouvrir le panneau"}
        title={open ? "Fermer" : "Ouvrir"}
      >
        <span className="panel-handle-arrow" aria-hidden="true">
          {open ? "›" : "‹"}
        </span>
      </button>

      <div className="panel-body">
        <header className="panel-header">
          <h1>Ma Tête</h1>
          <p>Capturer d&apos;abord, organiser ensuite.</p>
        </header>

        <section className="panel-section">
          <h2>Capturer</h2>
          <button
            type="button"
            className="panel-capture-btn"
            onClick={() => void captureShow()}
          >
            Qu&apos;est-ce que tu veux retenir ?
          </button>
        </section>

        <section className="panel-section panel-section-grow">
          <h2>Tâches</h2>
          {loadState === "loading" && (
            <p className="panel-muted">Chargement…</p>
          )}
          {loadState === "error" && (
            <p className="panel-error">{error ?? "Erreur"}</p>
          )}
          {loadState === "ready" && (
            <TaskList
              active={activeTasks}
              done={doneTasks}
              showDone={showDone}
              onToggleShowDone={() => setShowDone((v) => !v)}
              onToggleDone={(task) => void toggleTaskDone(task)}
              pendingId={pendingId}
            />
          )}
        </section>

        <section className="panel-section">
          <h2>Notes</h2>
          <p className="panel-muted">Bientôt — étapes suivantes.</p>
        </section>

        <section className="panel-section">
          <h2>Minuteur</h2>
          <p className="panel-muted">25 / 5 — bientôt.</p>
        </section>

        <section className="panel-section panel-settings">
          <h2>Réglages</h2>
          <label className="panel-toggle">
            <input
              type="checkbox"
              checked={alwaysOnTop}
              onChange={() => void toggleAlwaysOnTop()}
            />
            <span>Toujours au-dessus</span>
          </label>
        </section>
      </div>
    </div>
  );
}
