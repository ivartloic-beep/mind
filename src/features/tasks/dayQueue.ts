/**
 * File du jour — score simple pour le panneau MIND (≤4 tâches).
 */

import type { Task } from "../../types/models";
import { isTaskOpen } from "../../types/models";

function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

function parseDue(iso: string | undefined | null): Date | null {
  if (!iso) return null;
  const d = new Date(iso.length === 10 ? `${iso}T12:00:00` : iso);
  return Number.isNaN(d.getTime()) ? null : d;
}

/** Score décroissant : retard > aujourd’hui > demain > priorité > en cours > récent. */
export function scoreDayTask(task: Task, now = new Date()): number {
  let score = 0;
  const today = startOfDay(now);
  const tomorrow = new Date(today);
  tomorrow.setDate(tomorrow.getDate() + 1);
  const due = parseDue(task.dueDate);

  if (due) {
    const dueDay = startOfDay(due);
    if (dueDay < today) {
      const daysLate = Math.floor(
        (today.getTime() - dueDay.getTime()) / 86_400_000,
      );
      score += 1_000 + Math.min(daysLate, 14) * 20;
    } else if (dueDay.getTime() === today.getTime()) {
      score += 800;
    } else if (dueDay.getTime() === tomorrow.getTime()) {
      score += 600;
    } else {
      const daysAhead = Math.floor(
        (dueDay.getTime() - today.getTime()) / 86_400_000,
      );
      score += Math.max(0, 200 - daysAhead * 15);
    }
  }

  switch (task.priority) {
    case "high":
      score += 300;
      break;
    case "medium":
      score += 100;
      break;
    case "low":
      score += 20;
      break;
    default:
      break;
  }

  if (task.status === "in_progress") score += 150;

  const updated = Date.parse(task.updatedAt);
  if (!Number.isNaN(updated)) {
    // Plus récent = léger bonus (tie-break).
    score += Math.min(40, Math.max(0, 40 - (now.getTime() - updated) / 3_600_000));
  }

  return score;
}

export function pickDayQueue(rows: Task[], limit = 4): Task[] {
  return [...rows]
    .filter((t) => isTaskOpen(t))
    .sort((a, b) => {
      const ds = scoreDayTask(b) - scoreDayTask(a);
      if (ds !== 0) return ds;
      return b.updatedAt.localeCompare(a.updatedAt);
    })
    .slice(0, limit);
}
