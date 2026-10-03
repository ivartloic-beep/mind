/**
 * Liste des rappels de tâches en cours + planification à une heure HH:MM.
 */

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  clearTaskReminder,
  listReminders,
  listTasks,
  setTaskReminder,
} from "../../services/api";
import { listenDataChanged } from "../../services/events";
import { isTaskOpen, type Reminder, type Task } from "../../types/models";
import {
  fireAtAtLocalTime,
  formatReminderLabel,
  localTimeFromIso,
} from "./presets";
import "../timer/timer.css";
import "./reminder.css";

type Row = {
  reminder: Reminder;
  task: Task | null;
  when: string;
};

function effectiveWhen(r: Reminder): string {
  return r.snoozedUntil || r.fireAt;
}

/** Prochaine heure pleine locale (ex. 14:00), pour le formulaire. */
function defaultLocalTime(): string {
  const d = new Date();
  d.setMinutes(0, 0, 0);
  d.setHours(d.getHours() + 1);
  return localTimeFromIso(d.toISOString());
}

export function ActiveReminders() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [newTaskId, setNewTaskId] = useState("");
  const [newTime, setNewTime] = useState(defaultLocalTime);

  const [editId, setEditId] = useState<string | null>(null);
  const [editTime, setEditTime] = useState("09:00");

  const refresh = useCallback(async () => {
    const [taskRows, reminderRows] = await Promise.all([
      listTasks(),
      listReminders(),
    ]);
    setTasks(taskRows);
    setReminders(reminderRows);
  }, []);

  useEffect(() => {
    void refresh().catch((err) => {
      setError(err instanceof Error ? err.message : "Chargement impossible");
    });
    let unlisten: (() => void) | undefined;
    void listenDataChanged((payload) => {
      if (
        payload.entity === "reminder" ||
        payload.entity === "task" ||
        payload.entity === "sync"
      ) {
        void refresh().catch(() => undefined);
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [refresh]);

  const taskById = useMemo(() => {
    const map = new Map<string, Task>();
    for (const t of tasks) map.set(t.id, t);
    return map;
  }, [tasks]);

  const rows: Row[] = useMemo(() => {
    const now = Date.now();
    return reminders
      .map((reminder) => {
        const when = effectiveWhen(reminder);
        return {
          reminder,
          task: taskById.get(reminder.targetId) ?? null,
          when,
        };
      })
      .filter((row) => {
        const t = new Date(row.when).getTime();
        return Number.isFinite(t) && t >= now - 60_000;
      })
      .sort(
        (a, b) => new Date(a.when).getTime() - new Date(b.when).getTime(),
      );
  }, [reminders, taskById]);

  const openTasks = useMemo(
    () =>
      tasks
        .filter(isTaskOpen)
        .slice()
        .sort((a, b) => a.title.localeCompare(b.title, "fr")),
    [tasks],
  );

  useEffect(() => {
    if (!newTaskId && openTasks[0]) setNewTaskId(openTasks[0].id);
  }, [newTaskId, openTasks]);

  async function scheduleNew() {
    if (!newTaskId || !newTime) return;
    setBusyId("new");
    setError(null);
    try {
      const fireAt = fireAtAtLocalTime(newTime);
      await setTaskReminder(newTaskId, fireAt);
      setEditId(null);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Planification impossible");
    } finally {
      setBusyId(null);
    }
  }

  async function saveEdit(taskId: string) {
    setBusyId(taskId);
    setError(null);
    try {
      const fireAt = fireAtAtLocalTime(editTime);
      await setTaskReminder(taskId, fireAt);
      setEditId(null);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Modification impossible");
    } finally {
      setBusyId(null);
    }
  }

  async function removeReminder(taskId: string) {
    setBusyId(taskId);
    setError(null);
    try {
      await clearTaskReminder(taskId);
      if (editId === taskId) setEditId(null);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Suppression impossible");
    } finally {
      setBusyId(null);
    }
  }

  return (
    <div className="active-reminders">
      <h3 className="active-reminders-title">Rappels de tâches</h3>

      {rows.length === 0 ? (
        <p className="active-reminders-empty">Aucun rappel planifié.</p>
      ) : (
        <ul className="active-reminders-list">
          {rows.map(({ reminder, task, when }) => {
            const taskId = reminder.targetId;
            const editing = editId === taskId;
            return (
              <li key={reminder.id} className="active-reminders-item">
                <div className="active-reminders-item-main">
                  <span className="active-reminders-when">
                    {formatReminderLabel(when)}
                  </span>
                  <span className="active-reminders-task" title={task?.title}>
                    {task?.title || "Tâche introuvable"}
                  </span>
                </div>
                {editing ? (
                  <div className="active-reminders-edit">
                    <input
                      type="time"
                      value={editTime}
                      disabled={busyId === taskId}
                      onChange={(e) => setEditTime(e.target.value)}
                      aria-label="Nouvelle heure"
                    />
                    <button
                      type="button"
                      className="timer-btn is-primary"
                      disabled={busyId === taskId || !editTime}
                      onClick={() => void saveEdit(taskId)}
                    >
                      OK
                    </button>
                    <button
                      type="button"
                      className="timer-btn"
                      disabled={busyId === taskId}
                      onClick={() => setEditId(null)}
                    >
                      Annuler
                    </button>
                  </div>
                ) : (
                  <div className="active-reminders-actions">
                    <button
                      type="button"
                      className="timer-btn"
                      disabled={busyId === taskId}
                      onClick={() => {
                        setEditId(taskId);
                        setEditTime(localTimeFromIso(when));
                      }}
                    >
                      Modifier
                    </button>
                    <button
                      type="button"
                      className="timer-btn"
                      disabled={busyId === taskId}
                      onClick={() => void removeReminder(taskId)}
                    >
                      Supprimer
                    </button>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}

      <div className="active-reminders-form">
        <p className="active-reminders-form-label">Nouveau rappel</p>
        <label className="active-reminders-field">
          <span>Tâche</span>
          <select
            value={newTaskId}
            disabled={busyId === "new" || openTasks.length === 0}
            onChange={(e) => setNewTaskId(e.target.value)}
          >
            {openTasks.length === 0 ? (
              <option value="">Aucune tâche ouverte</option>
            ) : (
              openTasks.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.title}
                </option>
              ))
            )}
          </select>
        </label>
        <label className="active-reminders-field">
          <span>Heure</span>
          <input
            type="time"
            value={newTime}
            disabled={busyId === "new"}
            onChange={(e) => setNewTime(e.target.value)}
          />
        </label>
        <button
          type="button"
          className="timer-btn is-primary"
          disabled={
            busyId === "new" || !newTaskId || !newTime || openTasks.length === 0
          }
          onClick={() => void scheduleNew()}
        >
          Planifier
        </button>
        <p className="active-reminders-hint">
          Si l’heure est déjà passée, le rappel est mis à demain.
        </p>
      </div>

      {error && <p className="active-reminders-error">{error}</p>}
    </div>
  );
}
