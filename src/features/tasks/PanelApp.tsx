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
  upsertTask,
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
import {
  panelGetState,
  panelSetAlwaysOnTop,
  panelSetOpen,
} from "../../services/panel";
import {
  setActiveFilter as storeSetFilter,
  setPanelAlwaysOnTop as storeSetAot,
  setPanelOpen as storeSetOpen,
} from "../../stores/ui-store";
import type { Note, Project, Task } from "../../types/models";
import { NotesPanel } from "../notes/NotesPanel";
import {
  type ProjectFilterValue,
  toNoteFilter,
  toTaskFilter,
} from "../projects/filter";
import { ProjectFilterBar } from "../projects/ProjectFilterBar";
import { ProjectsManage } from "../projects/ProjectsManage";
import { ReminderDueBanner } from "../reminders/ReminderDueBanner";
import { TimerPanel } from "../timer/TimerPanel";
import { TaskList } from "./TaskList";
import "./panel.css";

type LoadState = "loading" | "ready" | "error";

export function PanelApp() {
  const [open, setOpen] = useState(true);
  const [alwaysOnTop, setAlwaysOnTop] = useState(false);
  const [projects, setProjects] = useState<Project[]>([]);
  const [filter, setFilter] = useState<ProjectFilterValue>("all");
  const [tasks, setTasks] = useState<Task[]>([]);
  const [notes, setNotes] = useState<Note[]>([]);
  const [tasksState, setTasksState] = useState<LoadState>("loading");
  const [notesState, setNotesState] = useState<LoadState>("loading");
  const [tasksError, setTasksError] = useState<string | null>(null);
  const [notesError, setNotesError] = useState<string | null>(null);
  const [showDone, setShowDone] = useState(false);
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [dueReminder, setDueReminder] = useState<ReminderDuePayload | null>(
    null,
  );
  const [reminderBusy, setReminderBusy] = useState(false);
  const [focusedTaskId, setFocusedTaskId] = useState<string | null>(null);
  const [autostart, setAutostart] = useState(true);
  const [autostartBusy, setAutostartBusy] = useState(false);
  const [, startTransition] = useTransition();
  const filterRef = useRef<ProjectFilterValue>(filter);
  filterRef.current = filter;
  const settingsRef = useRef<HTMLElement | null>(null);

  const refreshProjects = useCallback(async () => {
    const rows = await listProjects();
    setProjects(rows);
    return rows;
  }, []);

  const refreshTasks = useCallback(async (nextFilter: ProjectFilterValue) => {
    const rows = await listTasks(toTaskFilter(nextFilter));
    setTasks(rows);
    setTasksState("ready");
    setTasksError(null);
  }, []);

  const refreshNotes = useCallback(async (nextFilter: ProjectFilterValue) => {
    const rows = await listNotes(toNoteFilter(nextFilter));
    setNotes(rows);
    setNotesState("ready");
    setNotesError(null);
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
        // Hors Tauri / prefs absentes — UI reste utilisable.
      }

      try {
        const enabled = await autostartIsEnabled();
        if (!cancelled) setAutostart(enabled);
      } catch {
        /* plugin indisponible hors desktop */
      }

      try {
        await refreshProjects();
      } catch {
        /* projets optionnels */
      }

      try {
        await refreshTasks(filterRef.current);
      } catch (err) {
        if (cancelled) return;
        setTasksState("error");
        setTasksError(err instanceof Error ? err.message : "Chargement impossible");
      }

      try {
        await refreshNotes(filterRef.current);
      } catch (err) {
        if (cancelled) return;
        setNotesState("error");
        setNotesError(err instanceof Error ? err.message : "Chargement impossible");
      }
    }

    void bootstrap();

    const unlistens: Array<() => void> = [];
    void listenDataChanged((payload) => {
      if (payload.entity === "project") {
        startTransition(() => {
          void refreshProjects()
            .then((rows) => {
              const current = filterRef.current;
              if (
                current !== "all" &&
                current !== "no-project" &&
                !rows.some((p) => p.id === current)
              ) {
                filterRef.current = "all";
                setFilter("all");
                storeSetFilter("all");
              }
            })
            .catch(() => {
              /* ignore */
            });
        });
      }
      if (payload.entity === "task" || payload.entity === "reminder") {
        startTransition(() => {
          void refreshTasks(filterRef.current).catch(() => {
            /* ignore */
          });
        });
      }
      if (payload.entity === "note") {
        startTransition(() => {
          void refreshNotes(filterRef.current).catch(() => {
            /* ignore */
          });
        });
      }
    }).then((fn) => {
      unlistens.push(fn);
    });

    void listenReminderDue((payload) => {
      setDueReminder(payload);
      setOpen(true);
      storeSetOpen(true);
      void panelSetOpen(true).catch(() => {
        /* ignore */
      });
    }).then((fn) => {
      unlistens.push(fn);
    });

    void listenReminderOpenTask((payload) => {
      setFocusedTaskId(payload.taskId);
      setOpen(true);
      storeSetOpen(true);
      window.setTimeout(() => {
        const el = document.querySelector(
          `[data-task-id="${payload.taskId}"]`,
        );
        el?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      }, 80);
    }).then((fn) => {
      unlistens.push(fn);
    });

    void listenPanelStateChanged((payload) => {
      setOpen(payload.open);
      storeSetOpen(payload.open);
      setAlwaysOnTop(payload.alwaysOnTop);
      storeSetAot(payload.alwaysOnTop);
    }).then((fn) => {
      unlistens.push(fn);
    });

    void listenPanelFocusSettings(() => {
      setOpen(true);
      storeSetOpen(true);
      window.setTimeout(() => {
        settingsRef.current?.scrollIntoView({
          block: "nearest",
          behavior: "smooth",
        });
      }, 80);
    }).then((fn) => {
      unlistens.push(fn);
    });

    return () => {
      cancelled = true;
      for (const fn of unlistens) fn();
    };
  }, [refreshNotes, refreshProjects, refreshTasks]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        await refreshTasks(filter);
      } catch (err) {
        if (cancelled) return;
        setTasksState("error");
        setTasksError(err instanceof Error ? err.message : "Chargement impossible");
      }
      try {
        await refreshNotes(filter);
      } catch (err) {
        if (cancelled) return;
        setNotesState("error");
        setNotesError(err instanceof Error ? err.message : "Chargement impossible");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [filter, refreshNotes, refreshTasks]);

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
      const enabled = await autostartSetEnabled(next);
      setAutostart(enabled);
    } catch {
      setAutostart(!next);
    } finally {
      setAutostartBusy(false);
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

  async function assignTaskProject(task: Task, projectId: string | null) {
    const previous = tasks;
    setPendingId(task.id);
    setTasks((rows) =>
      rows.map((row) => (row.id === task.id ? { ...row, projectId } : row)),
    );
    try {
      await upsertTask({ ...task, projectId });
      await refreshTasks(filterRef.current);
    } catch {
      setTasks(previous);
    } finally {
      setPendingId(null);
    }
  }

  async function assignTaskReminder(task: Task, fireAt: string) {
    const previous = tasks;
    setPendingId(task.id);
    setTasks((rows) =>
      rows.map((row) =>
        row.id === task.id ? { ...row, reminder: fireAt } : row,
      ),
    );
    try {
      await setTaskReminder(task.id, fireAt);
      await refreshTasks(filterRef.current);
    } catch {
      setTasks(previous);
    } finally {
      setPendingId(null);
    }
  }

  async function removeTaskReminder(task: Task) {
    const previous = tasks;
    setPendingId(task.id);
    setTasks((rows) =>
      rows.map((row) =>
        row.id === task.id ? { ...row, reminder: undefined } : row,
      ),
    );
    try {
      await clearTaskReminder(task.id);
      await refreshTasks(filterRef.current);
    } catch {
      setTasks(previous);
    } finally {
      setPendingId(null);
    }
  }

  async function handleOpenDue() {
    if (!dueReminder) return;
    setReminderBusy(true);
    try {
      await openTaskFromReminder(dueReminder.taskId);
      setFocusedTaskId(dueReminder.taskId);
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
      await refreshTasks(filterRef.current);
    } catch {
      /* keep banner */
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
      await refreshTasks(filterRef.current);
    } catch {
      /* keep banner */
    } finally {
      setReminderBusy(false);
    }
  }

  function changeFilter(next: ProjectFilterValue) {
    setFilter(next);
    storeSetFilter(next);
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

        <section className="panel-section">
          <h2>Vue</h2>
          <ProjectFilterBar
            projects={projects}
            value={filter}
            onChange={changeFilter}
          />
        </section>

        <section className="panel-section panel-section-grow">
          <h2>Tâches</h2>
          {dueReminder && (
            <ReminderDueBanner
              due={dueReminder}
              busy={reminderBusy}
              onOpen={() => void handleOpenDue()}
              onSnooze={(kind) => void handleSnoozeDue(kind)}
              onDismiss={() => void handleDismissDue()}
            />
          )}
          {tasksState === "loading" && (
            <p className="panel-muted">Chargement…</p>
          )}
          {tasksState === "error" && (
            <p className="panel-error">{tasksError ?? "Erreur"}</p>
          )}
          {tasksState === "ready" && (
            <TaskList
              active={activeTasks}
              done={doneTasks}
              projects={projects}
              showDone={showDone}
              onToggleShowDone={() => setShowDone((v) => !v)}
              onToggleDone={(task) => void toggleTaskDone(task)}
              onAssignProject={(task, projectId) =>
                void assignTaskProject(task, projectId)
              }
              onSetReminder={(task, fireAt) =>
                void assignTaskReminder(task, fireAt)
              }
              onClearReminder={(task) => void removeTaskReminder(task)}
              pendingId={pendingId}
              focusedTaskId={focusedTaskId}
            />
          )}
        </section>

        <section className="panel-section panel-section-notes">
          <h2>Notes</h2>
          <NotesPanel
            notes={notes}
            projects={projects}
            loadState={notesState}
            error={notesError}
          />
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
          <ProjectsManage
            projects={projects}
            onChanged={() => {
              void refreshProjects()
                .then((rows) => {
                  if (
                    filter !== "all" &&
                    filter !== "no-project" &&
                    !rows.some((p) => p.id === filter)
                  ) {
                    changeFilter("all");
                  } else {
                    void refreshTasks(filter);
                    void refreshNotes(filter);
                  }
                })
                .catch(() => {
                  /* ignore */
                });
            }}
          />
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
