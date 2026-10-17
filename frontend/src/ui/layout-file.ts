/**
 * The interface layout as a file: the panel's folds, side, size and pinned rail, the tablet layout switch, the floating
 * toolbars, the shortcut overrides, the second-press setting and the pen and gamepad bindings, so a person can move their arrangement to another
 * browser or share it. Only these keys are read or written; nothing about the scene is in it.
 */

import { parseToolbars, TOOLBARS_STORE } from './toolbars';

const FORMAT = 'formsculptor-layout';
export const TOOL_SWAP_STORE = 'formsculptor.toolswap';

/** The browser storage keys a layout file carries. */
export const LAYOUT_KEYS = ['formsculptor.layout', 'formsculptor.tablet', TOOLBARS_STORE, 'formsculptor.prefs', TOOL_SWAP_STORE, 'formsculptor.bindings', 'formsculptor.theme', 'formsculptor.overlays'] as const;

/** The layout as kept in this browser, as the text of a layout file. */
export function exportLayout(version: string): string {
  const entries: Record<string, string> = {};
  for (const key of LAYOUT_KEYS) {
    try {
      const v = localStorage.getItem(key);
      if (v !== null) entries[key] = v;
    } catch {
      // storage blocked; the file holds what could be read
    }
  }
  return JSON.stringify({ format: FORMAT, version: 1, app: version, saved: new Date().toISOString(), entries }, null, 2);
}

/** Checks a layout file and writes its keys into this browser's storage; returns the keys written or why it was
 * refused. Values are checked to be the JSON or the switch each key holds, so a damaged file changes nothing. */
export function importLayout(text: string): { written: string[] } | { error: string } {
  let doc: unknown;
  try {
    doc = JSON.parse(text);
  } catch {
    return { error: 'not a layout file (it is not JSON)' };
  }
  const d = doc as { format?: unknown; entries?: unknown };
  if (!d || typeof d !== 'object' || d.format !== FORMAT || !d.entries || typeof d.entries !== 'object') return { error: 'not a formsculptor layout file' };
  const entries = d.entries as Record<string, unknown>;
  const accepted: [string, string][] = [];
  for (const key of LAYOUT_KEYS) {
    const v = entries[key];
    if (typeof v !== 'string') continue;
    if (key === 'formsculptor.tablet' || key === TOOL_SWAP_STORE) {
      if (v !== 'on' && v !== 'off') return { error: `${key} must be on or off` };
    } else {
      try {
        const parsed: unknown = JSON.parse(v);
        if (key === TOOLBARS_STORE && parseToolbars(parsed).length !== (Array.isArray(parsed) ? parsed.length : -1)) return { error: 'the toolbars in the file are damaged' };
        if (!parsed || typeof parsed !== 'object') return { error: `${key} is damaged` };
      } catch {
        return { error: `${key} is damaged` };
      }
    }
    accepted.push([key, v]);
  }
  if (accepted.length === 0) return { error: 'the file holds no layout settings' };
  try {
    for (const [key, v] of accepted) localStorage.setItem(key, v);
  } catch {
    return { error: 'this browser does not allow saving settings' };
  }
  return { written: accepted.map(([k]) => k) };
}
