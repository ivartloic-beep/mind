/**
 * Wrappers invoke Tauri — stubs étape 1 (pas de commands métier).
 */

export async function ping(): Promise<string> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("ping");
}
