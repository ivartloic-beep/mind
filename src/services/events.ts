/**
 * Abonnements events Tauri — `data-changed` après mutations storage.
 */

import { listen } from "@tauri-apps/api/event";
import type { DataChangedPayload } from "../types/models";

export type { DataChangedPayload };

export async function listenDataChanged(
  handler: (payload: DataChangedPayload) => void,
): Promise<() => void> {
  const unlisten = await listen<DataChangedPayload>("data-changed", (event) => {
    handler(event.payload);
  });
  return unlisten;
}
