/**
 * Libellés FR pour attributs tâche (panneau / bibliothèque).
 */

import type { Task, TaskPriority, TaskStatus } from "../../types/models";

export function formatTaskStatus(status: TaskStatus | string | undefined): string {
  switch (status) {
    case "todo":
      return "À faire";
    case "in_progress":
      return "En cours";
    case "done":
      return "Terminée";
    default:
      return status ? String(status) : "À faire";
  }
}

export function formatTaskPriority(
  priority: TaskPriority | string | undefined | null,
): string | null {
  if (!priority) return null;
  switch (priority) {
    case "low":
      return "Basse";
    case "medium":
    case "normal":
      return "Moyenne";
    case "high":
    case "urgent":
      return "Haute";
    default:
      return String(priority);
  }
}

export function formatTaskDue(iso: string | undefined | null): string | null {
  if (!iso) return null;
  const d = new Date(iso.length === 10 ? `${iso}T12:00:00` : iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString("fr-FR", {
    day: "numeric",
    month: "short",
  });
}

export function taskMetaParts(task: Pick<Task, "status" | "priority" | "dueDate" | "category">): string[] {
  const parts: string[] = [];
  parts.push(formatTaskStatus(task.status));
  const prio = formatTaskPriority(task.priority);
  if (prio) parts.push(prio);
  const due = formatTaskDue(task.dueDate);
  if (due) parts.push(due);
  if (task.category?.trim()) parts.push(task.category.trim());
  return parts;
}

export function priorityClass(priority: TaskPriority | string | undefined | null): string {
  switch (priority) {
    case "high":
    case "urgent":
      return "is-high";
    case "low":
      return "is-low";
    default:
      return "is-medium";
  }
}

export function statusClass(status: TaskStatus | string | undefined): string {
  switch (status) {
    case "done":
      return "is-done";
    case "in_progress":
      return "is-progress";
    default:
      return "is-todo";
  }
}
