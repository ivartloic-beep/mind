/**
 * Bannière discrète à l’échéance d’un rappel — ouvrir / snooze / dismiss.
 */

import type { SnoozeKind } from "../../services/api";
import type { ReminderDuePayload } from "../../services/events";
import "./reminder.css";

type Props = {
  due: ReminderDuePayload;
  busy?: boolean;
  onOpen: () => void;
  onSnooze: (kind: SnoozeKind) => void;
  onDismiss: () => void;
};

export function ReminderDueBanner({
  due,
  busy,
  onOpen,
  onSnooze,
  onDismiss,
}: Props) {
  return (
    <div
      className={`reminder-due ${due.missed ? "is-missed" : ""}`}
      role="status"
      aria-live="polite"
    >
      <div className="reminder-due-text">
        <span className="reminder-due-bell" aria-hidden="true">
          🔔
        </span>
        <div>
          <p className="reminder-due-title">
            {due.missed ? "Rappel manqué" : "Rappel"}
          </p>
          <p className="reminder-due-task">{due.taskTitle}</p>
        </div>
      </div>
      <div className="reminder-due-actions">
        <button
          type="button"
          className="reminder-btn reminder-btn-primary"
          disabled={busy}
          onClick={onOpen}
        >
          Ouvrir
        </button>
        <button
          type="button"
          className="reminder-btn"
          disabled={busy}
          onClick={() => onSnooze("10m")}
        >
          +10 min
        </button>
        <button
          type="button"
          className="reminder-btn"
          disabled={busy}
          onClick={() => onSnooze("1h")}
        >
          +1 h
        </button>
        <button
          type="button"
          className="reminder-btn"
          disabled={busy}
          onClick={() => onSnooze("tomorrow")}
        >
          Demain
        </button>
        <button
          type="button"
          className="reminder-btn reminder-btn-ghost"
          disabled={busy}
          onClick={onDismiss}
          aria-label="Ignorer le rappel"
        >
          ✕
        </button>
      </div>
    </div>
  );
}
