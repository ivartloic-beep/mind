import { useState } from "react";
import type { Project, Task } from "../../types/models";
import { isTaskDone } from "../../types/models";
import {
  fireAtInMinutes,
  fireAtTomorrowMorning,
  formatReminderLabel,
} from "../reminders/presets";
import "../reminders/reminder.css";
import { ProjectSelect } from "../projects/ProjectSelect";
import {
  formatTaskDue,
  formatTaskPriority,
  formatTaskStatus,
  priorityClass,
  statusClass,
} from "./taskLabels";

type Props = {
  active: Task[];
  done: Task[];
  projects: Project[];
  showDone: boolean;
  onToggleShowDone: () => void;
  onToggleDone: (task: Task) => void;
  onDelete: (task: Task) => void;
  onAssignProject: (task: Task, projectId: string | null) => void;
  onSetReminder: (task: Task, fireAt: string) => void;
  onClearReminder: (task: Task) => void;
  onOpenTask?: (task: Task) => void;
  pendingId: string | null;
  focusedTaskId?: string | null;
};

export function TaskList({
  active,
  done,
  projects,
  showDone,
  onToggleShowDone,
  onToggleDone,
  onDelete,
  onAssignProject,
  onSetReminder,
  onClearReminder,
  onOpenTask,
  pendingId,
  focusedTaskId,
}: Props) {
  return (
    <div className="task-board">
      {active.length === 0 ? (
        <p className="panel-muted">Rien en cours — capture une tâche.</p>
      ) : (
        <ul className="task-list">
          {active.map((task) => (
            <TaskRow
              key={task.id}
              task={task}
              projects={projects}
              pending={pendingId === task.id}
              focused={focusedTaskId === task.id}
              onToggle={() => onToggleDone(task)}
              onDelete={() => onDelete(task)}
              onAssignProject={(projectId) => onAssignProject(task, projectId)}
              onSetReminder={(fireAt) => onSetReminder(task, fireAt)}
              onClearReminder={() => onClearReminder(task)}
              onOpen={onOpenTask ? () => onOpenTask(task) : undefined}
            />
          ))}
        </ul>
      )}

      {done.length > 0 && (
        <div className="task-done-block">
          <button
            type="button"
            className="task-done-toggle"
            onClick={onToggleShowDone}
            aria-expanded={showDone}
          >
            <span>Terminées</span>
            <span className="task-done-count">{done.length}</span>
            <span className="task-done-chevron" aria-hidden="true">
              {showDone ? "▾" : "▸"}
            </span>
          </button>
          {showDone && (
            <ul className="task-list task-list-done">
              {done.map((task) => (
                <TaskRow
                  key={task.id}
                  task={task}
                  projects={projects}
                  pending={pendingId === task.id}
                  focused={focusedTaskId === task.id}
                  onToggle={() => onToggleDone(task)}
                  onDelete={() => onDelete(task)}
                  onAssignProject={(projectId) => onAssignProject(task, projectId)}
                  onSetReminder={(fireAt) => onSetReminder(task, fireAt)}
                  onClearReminder={() => onClearReminder(task)}
                  onOpen={onOpenTask ? () => onOpenTask(task) : undefined}
                />
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}

function TaskRow({
  task,
  projects,
  pending,
  focused,
  onToggle,
  onDelete,
  onAssignProject,
  onSetReminder,
  onClearReminder,
  onOpen,
}: {
  task: Task;
  projects: Project[];
  pending: boolean;
  focused: boolean;
  onToggle: () => void;
  onDelete: () => void;
  onAssignProject: (projectId: string | null) => void;
  onSetReminder: (fireAt: string) => void;
  onClearReminder: () => void;
  onOpen?: () => void;
}) {
  const [menuOpen, setMenuOpen] = useState(false);
  const done = isTaskDone(task);
  const hasReminder = Boolean(task.reminder);
  const prio = formatTaskPriority(task.priority);
  const due = formatTaskDue(task.dueDate);

  return (
    <li
      className={`task-item ${done ? "is-done" : ""} ${pending ? "is-pending" : ""} ${focused ? "is-focused" : ""}`}
      data-task-id={task.id}
    >
      <button
        type="button"
        className={`task-check ${done ? "is-checked" : ""}`}
        onClick={onToggle}
        disabled={pending}
        aria-label={done ? "Remettre active" : "Marquer terminée"}
        title={done ? "Remettre active" : "Terminer"}
      >
        {done ? "✓" : ""}
      </button>
      <div className="task-body">
        <div className="task-title-row">
          {onOpen ? (
            <button
              type="button"
              className="task-title task-title-btn"
              disabled={pending}
              title="Ouvrir la fiche Gestion"
              onClick={onOpen}
            >
              {task.title}
            </button>
          ) : (
            <span className="task-title">{task.title}</span>
          )}
          <button
            type="button"
            className={`task-bell ${hasReminder ? "has-reminder" : ""} ${menuOpen ? "is-open" : ""}`}
            disabled={pending}
            aria-label={
              hasReminder
                ? `Rappel ${formatReminderLabel(task.reminder!)} — modifier`
                : "Ajouter un rappel"
            }
            title={
              hasReminder
                ? `Rappel : ${formatReminderLabel(task.reminder!)}`
                : "Rappel"
            }
            onClick={() => setMenuOpen((v) => !v)}
          >
            🔔
          </button>
          <button
            type="button"
            className="item-delete-btn"
            disabled={pending}
            aria-label="Supprimer la tâche"
            title="Supprimer"
            onClick={onDelete}
          >
            ×
          </button>
        </div>
        <div className="task-attrs" aria-label="Attributs">
          <span className={`task-chip status ${statusClass(task.status)}`}>
            {formatTaskStatus(task.status)}
          </span>
          {prio && (
            <span className={`task-chip priority ${priorityClass(task.priority)}`}>
              {prio}
            </span>
          )}
          {due && <span className="task-chip due">{due}</span>}
          {task.category?.trim() ? (
            <span className="task-chip category">{task.category.trim()}</span>
          ) : null}
          {(task.documents?.length ?? 0) > 0 && (
            <span className="task-chip docs">
              {task.documents.length} doc{task.documents.length > 1 ? "s" : ""}
            </span>
          )}
        </div>
        <div className="task-meta-row">
          <ProjectSelect
            projects={projects}
            value={task.projectId}
            disabled={pending}
            onChange={onAssignProject}
            ariaLabel="Projet de la tâche"
          />
          {hasReminder && (
            <span className="task-reminder-when">
              🔔 {formatReminderLabel(task.reminder!)}
            </span>
          )}
        </div>
        {menuOpen && (
          <div className="task-reminder-menu">
            <button
              type="button"
              className="task-reminder-chip"
              disabled={pending}
              onClick={() => {
                onSetReminder(fireAtInMinutes(10));
                setMenuOpen(false);
              }}
            >
              +10 min
            </button>
            <button
              type="button"
              className="task-reminder-chip"
              disabled={pending}
              onClick={() => {
                onSetReminder(fireAtInMinutes(60));
                setMenuOpen(false);
              }}
            >
              +1 h
            </button>
            <button
              type="button"
              className="task-reminder-chip"
              disabled={pending}
              onClick={() => {
                onSetReminder(fireAtTomorrowMorning());
                setMenuOpen(false);
              }}
            >
              Demain 9 h
            </button>
            {hasReminder && (
              <button
                type="button"
                className="task-reminder-chip is-danger"
                disabled={pending}
                onClick={() => {
                  onClearReminder();
                  setMenuOpen(false);
                }}
              >
                Retirer
              </button>
            )}
          </div>
        )}
      </div>
    </li>
  );
}
