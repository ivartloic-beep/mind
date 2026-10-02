/**
 * Panneau compact — actions + 5 dernières tâches / notes + minuteur.
 */

import { useCallback, useEffect, useRef, useState, useTransition } from "react";
import {
  autostartIsEnabled,
  autostartSetEnabled,
  clearTaskReminder,
  dismissReminder,
  listNotes,
  listProjects,
  listTasks,
  openTaskFromReminder,
  setTaskDone,
  setTaskReminder,
  snoozeReminder,
  type SnoozeKind,
} from "../../services/api";
import { captureShow } from "../../services/capture";
import {
  listenDataChanged,
  listenPanelFocusSettings,
  listenPanelStateChanged,
  listenReminderDue,
  listenReminderOpenTask,
  type ReminderDuePayload,
} from "../../services/events";
import { libraryShow } from "../../services/library";
import {
  panelGetState,
  panelSetAlwaysOnTop,
  panelSetOpen,
} from "../../services/panel";
import { createScratchPostit } from "../../services/postit";
import {
  setPanelAlwaysOnTop as storeSetAot,
  setPanelOpen as storeSetOpen,
} from "../../stores/ui-store";
import type { Note, Task } from "../../types/models";
import { ReminderDueBanner } from "../reminders/ReminderDueBanner";
import { TimerPanel } from "../timer/TimerPanel";
import "./panel.css";

type LoadState = "loading" | "ready" | "error";

function sortRecentTasks(rows: Task[]): Task[] {
  return [...rows]
    .filter((t) => t.status === "active")
    .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
    .slice(0, 5);
}

function sortRecentNotes(rows: Note[]): Note[] {
  return [...rows]
    .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
    .slice(0, 5);
}

function preview(text: string): string {
  const one = text.replace(/\s+/g, " ").trim();
  return one.length > 48 ? `${one.slice(0, 48)}…` : one;
}

export function PanelApp() {
  const [open, setOpen] = useState(true);
  const [alwaysOnTop, setAlwaysOnTop] = useState(false);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [notes, setNotes] = useState<Note[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [dueReminder, setDueReminder] = useState<ReminderDuePayload | null>(
    null,
  );
  const [reminderBusy, setReminderBusy] = useState(false);
  const [autostart, setAutostart] = useState(true);
  const [autostartBusy, setAutostartBusy] = useState(false);
  const [, startTransition] = useTransition();
  const settingsRef = useRef<HTMLElement | null>(null);

  const refresh = useCallback(async () => {
    const [taskRows, noteRows] = await Promise.all([
      listTasks(),
      listNotes(),
    ]);
    setTasks(sortRecentTasks(taskRows));
    setNotes(sortRecentNotes(noteRows));
    setLoadState("ready");
    setLoadError(null);
  }, []);

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
        /* hors Tauri */
      }

      try {
        const enabled = await autostartIsEnabled();
        if (!cancelled) setAutostart(enabled);
      } catch {
        /* ignore */
      }

      try {
        await listProjects();
        await refresh();
      } catch (err) {
        if (cancelled) return;
        setLoadState("error");
        setLoadError(err instanceof Error ? err.message : "Chargement impossible");
      }
    }

    void bootstrap();

    const unlistens: Array<() => void> = [];
    void listenDataChanged((payload) => {
      if (
        payload.entity === "task" ||
        payload.entity === "note" ||
        payload.entity === "reminder" ||
        payload.entity === "project"
      ) {
        startTransition(() => {
          void refresh().catch(() => undefined);
        });
      }
    }).then((fn) => unlistens.push(fn));

    void listenReminderDue((payload) => {
      setDueReminder(payload);
      setOpen(true);
      storeSetOpen(true);
      void panelSetOpen(true).catch(() => undefined);
    }).then((fn) => unlistens.push(fn));

    void listenReminderOpenTask((payload) => {
      setOpen(true);
      storeSetOpen(true);
      window.setTimeout(() => {
        document
          .querySelector(`[data-task-id="${payload.taskId}"]`)
          ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      }, 80);
    }).then((fn) => unlistens.push(fn));

    void listenPanelStateChanged((payload) => {
      setOpen(payload.open);
      storeSetOpen(payload.open);
      setAlwaysOnTop(payload.alwaysOnTop);
      storeSetAot(payload.alwaysOnTop);
    }).then((fn) => unlistens.push(fn));

    void listenPanelFocusSettings(() => {
      setOpen(true);
      storeSetOpen(true);
      window.setTimeout(() => {
        settingsRef.current?.scrollIntoView({
          block: "nearest",
          behavior: "smooth",
        });
      }, 80);
    }).then((fn) => unlistens.push(fn));

    return () => {
      cancelled = true;
      for (const fn of unlistens) fn();
    };
  }, [refresh]);

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

  async function toggleAutostart() {
    const next = !autostart;
    setAutostartBusy(true);
    setAutostart(next);
    try {
      setAutostart(await autostartSetEnabled(next));
    } catch {
      setAutostart(!next);
    } finally {
      setAutostartBusy(false);
    }
  }

  async function toggleTaskDone(task: Task) {
    setPendingId(task.id);
    try {
      await setTaskDone(task.id, task.status !== "done");
      await refresh();
    } finally {
      setPendingId(null);
    }
  }

  async function quickRemind(task: Task) {
    const fireAt = new Date(Date.now() + 10 * 60_000).toISOString();
    setPendingId(task.id);
    try {
      if (task.reminder) await clearTaskReminder(task.id);
      else await setTaskReminder(task.id, fireAt);
      await refresh();
    } finally {
      setPendingId(null);
    }
  }

  async function handleOpenDue() {
    if (!dueReminder) return;
    setReminderBusy(true);
    try {
      await openTaskFromReminder(dueReminder.taskId);
      await libraryShow();
    } finally {
      setReminderBusy(false);
    }
  }

  async function handleSnoozeDue(kind: SnoozeKind) {
    if (!dueReminder) return;
    setReminderBusy(true);
    try {
      await snoozeReminder(dueReminder.reminderId, kind);
      setDueReminder(null);
      await refresh();
    } catch {
      /* keep */
    } finally {
      setReminderBusy(false);
    }
  }

  async function handleDismissDue() {
    if (!dueReminder) return;
    setReminderBusy(true);
    try {
      await dismissReminder(dueReminder.reminderId);
      setDueReminder(null);
      await refresh();
    } catch {
      /* keep */
    } finally {
      setReminderBusy(false);
    }
  }

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
          <h1>MIND</h1>
          <p>Capturer d&apos;abord, organiser ensuite.</p>
        </header>

        {dueReminder && (
          <ReminderDueBanner
            due={dueReminder}
            busy={reminderBusy}
            onOpen={() => void handleOpenDue()}
            onSnooze={(kind) => void handleSnoozeDue(kind)}
            onDismiss={() => void handleDismissDue()}
          />
        )}

        <section className="panel-section">
          <h2>Actions</h2>
          <div className="panel-actions">
            <button
              type="button"
              className="panel-action-btn is-primary"
              onClick={() => void captureShow()}
            >
              Capturer
            </button>
            <button
              type="button"
              className="panel-action-btn"
              onClick={() => void createScratchPostit()}
            >
              Post-it
            </button>
            <button
              type="button"
              className="panel-action-btn"
              onClick={() => void libraryShow()}
            >
              Bibliothèque
            </button>
          </div>
        </section>

        <section className="panel-section panel-section-grow">
          <div className="panel-section-head">
            <h2>Récent</h2>
            <button
              type="button"
              className="panel-link-btn"
              onClick={() => void libraryShow()}
            >
              Voir tout
            </button>
          </div>

          {loadState === "loading" && (
            <p className="panel-muted">Chargement…</p>
          )}
          {loadState === "error" && (
            <p className="panel-error">{loadError ?? "Erreur"}</p>
          )}
          {loadState === "ready" && (
            <>
              <p className="panel-subhead">Tâches</p>
              {tasks.length === 0 ? (
                <p className="panel-muted">Aucune tâche active.</p>
              ) : (
                <ul className="panel-recent-list">
                  {tasks.map((task) => (
                    <li
                      key={task.id}
                      className="panel-recent-item"
                      data-task-id={task.id}
                    >
                      <button
                        type="button"
                        className="panel-recent-check"
                        disabled={pendingId === task.id}
                        aria-label="Terminer"
                        onClick={() => void toggleTaskDone(task)}
                      >
                        ○
                      </button>
                      <span className="panel-recent-title">{task.title}</span>
                      <button
                        type="button"
                        className={`panel-recent-bell ${task.reminder ? "has-reminder" : ""}`}
                        disabled={pendingId === task.id}
                        title={task.reminder ? "Retirer le rappel" : "Rappel +10 min"}
                        onClick={() => void quickRemind(task)}
                      >
                        🔔
                      </button>
                    </li>
                  ))}
                </ul>
              )}

              <p className="panel-subhead">Notes</p>
              {notes.length === 0 ? (
                <p className="panel-muted">Aucune note.</p>
              ) : (
                <ul className="panel-recent-list">
                  {notes.map((note) => (
                    <li key={note.id} className="panel-recent-item">
                      <span className="panel-recent-kind">
                        {note.kind === "idea" ? "idée" : "note"}
                      </span>
                      <span className="panel-recent-title">
                        {note.title?.trim() || preview(note.body)}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </>
          )}
        </section>

        <section className="panel-section">
          <h2>Minuteur</h2>
          <TimerPanel />
        </section>

        <section
          className="panel-section panel-settings"
          ref={settingsRef}
          id="panel-settings"
        >
          <h2>Réglages</h2>
          <label className="panel-toggle">
            <input
              type="checkbox"
              checked={alwaysOnTop}
              onChange={() => void toggleAlwaysOnTop()}
            />
            <span>Toujours au-dessus</span>
          </label>
          <label className="panel-toggle">
            <input
              type="checkbox"
              checked={autostart}
              disabled={autostartBusy}
              onChange={() => void toggleAutostart()}
            />
            <span>Démarrer avec Windows</span>
          </label>
        </section>
      </div>
    </div>
  );
}
