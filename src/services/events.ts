/**
 * Abonnements events Tauri — `data-changed` après mutations storage.
 */

import { listen } from "@tauri-apps/api/event";
import type { DataChangedPayload } from "../types/models";

export type { DataChangedPayload };

export type ReminderDuePayload = {
  reminderId: string;
  taskId: string;
  taskTitle: string;
  fireAt: string;
  missed: boolean;
};

export type ReminderOpenTaskPayload = {
  taskId: string;
};

export async function listenDataChanged(
  handler: (payload: DataChangedPayload) => void,
): Promise<() => void> {
  const unlisten = await listen<DataChangedPayload>("data-changed", (event) => {
    handler(event.payload);
  });
  return unlisten;
}

export async function listenReminderDue(
  handler: (payload: ReminderDuePayload) => void,
): Promise<() => void> {
  const unlisten = await listen<ReminderDuePayload>("reminder-due", (event) => {
    handler(event.payload);
  });
  return unlisten;
}

export async function listenReminderOpenTask(
  handler: (payload: ReminderOpenTaskPayload) => void,
): Promise<() => void> {
  const unlisten = await listen<ReminderOpenTaskPayload>(
    "reminder-open-task",
    (event) => {
      handler(event.payload);
    },
  );
  return unlisten;
}
