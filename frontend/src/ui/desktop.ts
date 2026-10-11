import { invoke } from '@tauri-apps/api/core';

/** True inside the Tauri window, which injects its IPC into the page the engine serves. */
export const inDesktop = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * Saves bytes through the native save dialog. The bytes travel as the raw IPC body (no JSON or base64), the name as a
 * header. Resolves to the chosen path, or null when the person cancels.
 */
export async function saveNative(blob: Blob, name: string): Promise<string | null> {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  return invoke<string | null>('save_file', bytes, { headers: { 'x-file-name': encodeURIComponent(name) } });
}
