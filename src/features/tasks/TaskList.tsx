import type { Task } from "../../types/models";

type Props = {
  active: Task[];
  done: Task[];
  showDone: boolean;
  onToggleShowDone: () => void;
  onToggleDone: (task: Task) => void;
  pendingId: string | null;
};

export function TaskList({
  active,
  done,
  showDone,
  onToggleShowDone,
  onToggleDone,
  pendingId,
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
              pending={pendingId === task.id}
              onToggle={() => onToggleDone(task)}
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
                  pending={pendingId === task.id}
                  onToggle={() => onToggleDone(task)}
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
  pending,
  onToggle,
}: {
  task: Task;
  pending: boolean;
  onToggle: () => void;
}) {
  const done = task.status === "done";
  return (
    <li className={`task-item ${done ? "is-done" : ""} ${pending ? "is-pending" : ""}`}>
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
        <span className="task-title">{task.title}</span>
        {(task.dueDate || task.priority) && (
          <span className="task-meta">
            {task.priority ? task.priority : null}
            {task.priority && task.dueDate ? " · " : null}
            {task.dueDate ? formatDue(task.dueDate) : null}
          </span>
        )}
      </div>
    </li>
  );
}

function formatDue(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString("fr-FR", { day: "numeric", month: "short" });
}
