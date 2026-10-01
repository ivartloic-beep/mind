/**
 * Abonnements events Tauri — stubs étape 1.
 * Event métier `data-changed` arrive à l'étape 2+.
 */

export type DataChangedPayload = {
  entity: string;
  id: string;
};

export async function listenDataChanged(
  _handler: (payload: DataChangedPayload) => void,
): Promise<() => void> {
  // Stub : aucun listener tant que le backend n'émet pas.
  return () => {};
}
