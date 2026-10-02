/**
 * Bibliothèque — vue complète tâches / notes / projets.
 */

import { useCallback, useEffect, useRef, useState, useTransition } from "react";
import {
  clearTaskReminder,
  listNotes,
  listProjects,
  listTasks,
  setTaskDone,
  setTaskReminder,
  upsertTask,
} from "../../services/api";
import { listenDataChanged } from "../../services/events";
import type { Note, Project, Task } from "../../types/models";
import { NotesPanel } from "../notes/NotesPanel";
import {
  type ProjectFilterValue,
  toNoteFilter,
  toTaskFilter,
} from "../projects/filter";
import { ProjectFilterBar } from "../projects/ProjectFilterBar";
import { ProjectsManage } from "../projects/ProjectsManage";
import { TaskList } from "../tasks/TaskList";
import "../tasks/panel.css";
import "./library.css";

type LoadState = "loading" | "ready" | "error";

export function LibraryApp() {
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
  const [, startTransition] = useTransition();
  const filterRef = useRef<ProjectFilterValue>(filter);
  filterRef.current = filter;

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
    void (async () => {
      try {
        await refreshProjects();
      } catch {
        /* optional */
      }
      try {
        await refreshTasks(filterRef.current);
      } catch (err) {
        if (!cancelled) {
          setTasksState("error");
          setTasksError(err instanceof Error ? err.message : "Erreur");
        }
      }
      try {
        await refreshNotes(filterRef.current);
      } catch (err) {
        if (!cancelled) {
          setNotesState("error");
          setNotesError(err instanceof Error ? err.message : "Erreur");
        }
      }
    })();

    let unlisten: (() => void) | undefined;
    void listenDataChanged((payload) => {
      if (payload.entity === "project") {
        startTransition(() => {
          void refreshProjects().catch(() => undefined);
        });
      }
      if (payload.entity === "task" || payload.entity === "reminder") {
        startTransition(() => {
          void refreshTasks(filterRef.current).catch(() => undefined);
        });
      }
      if (payload.entity === "note") {
        startTransition(() => {
          void refreshNotes(filterRef.current).catch(() => undefined);
        });
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [refreshNotes, refreshProjects, refreshTasks]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        await refreshTasks(filter);
      } catch (err) {
        if (!cancelled) {
          setTasksState("error");
          setTasksError(err instanceof Error ? err.message : "Erreur");
        }
      }
      try {
        await refreshNotes(filter);
      } catch (err) {
        if (!cancelled) {
          setNotesState("error");
          setNotesError(err instanceof Error ? err.message : "Erreur");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [filter, refreshNotes, refreshTasks]);

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

  const activeTasks = tasks.filter((t) => t.status === "active");
  const doneTasks = tasks.filter((t) => t.status === "done");

  return (
    <div className="library-root">
      <header className="library-header">
        <h1>Bibliothèque</h1>
        <p>Toutes les tâches et notes — organise ici.</p>
      </header>

      <section className="library-section">
        <h2>Vue</h2>
        <ProjectFilterBar
          projects={projects}
          value={filter}
          onChange={setFilter}
        />
      </section>

      <div className="library-columns">
        <section className="library-section library-col">
          <h2>Tâches</h2>
          {tasksState === "loading" && <p className="panel-muted">Chargement…</p>}
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
            />
          )}
        </section>

        <section className="library-section library-col">
          <h2>Notes</h2>
          <NotesPanel
            notes={notes}
            projects={projects}
            loadState={notesState}
            error={notesError}
          />
        </section>
      </div>

      <section className="library-section">
        <h2>Projets</h2>
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
                  setFilter("all");
                } else {
                  void refreshTasks(filter);
                  void refreshNotes(filter);
                }
              })
              .catch(() => undefined);
          }}
        />
      </section>
    </div>
  );
}
