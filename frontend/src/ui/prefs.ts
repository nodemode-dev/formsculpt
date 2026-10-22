const STORE = 'formsculpt.prefs';
const LEGACY = 'formsculpt.keys';

export interface Prefs {
  shortcuts: Record<string, string[]>;
  recent: string[];
}

export function loadPrefs(): Prefs {
  const out: Prefs = { shortcuts: {}, recent: [] };
  try {
    const raw = localStorage.getItem(STORE);
    const p = raw ? (JSON.parse(raw) as Partial<Prefs>) : {};
    if (p.shortcuts && typeof p.shortcuts === 'object') {
      for (const [id, keys] of Object.entries(p.shortcuts)) if (Array.isArray(keys)) out.shortcuts[id] = keys.filter((k): k is string => typeof k === 'string');
    }
    if (Array.isArray(p.recent)) out.recent = p.recent.filter((k): k is string => typeof k === 'string');
  } catch {
    // storage blocked or unreadable; defaults stand
  }
  return out;
}

export function savePrefs(prefs: Prefs): void {
  try {
    localStorage.setItem(STORE, JSON.stringify(prefs));
  } catch {
    // storage blocked; the change lasts for this session
  }
}

export function updatePrefs(patch: Partial<Prefs>): void {
  savePrefs({ ...loadPrefs(), ...patch });
}

/** The old rebind form's overrides (combo to action) once, removing them from storage; empty when none or already moved. */
export function takeLegacyKeys(): Record<string, string> {
  try {
    const raw = localStorage.getItem(LEGACY);
    if (raw === null) return {};
    localStorage.removeItem(LEGACY);
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return {};
    const out: Record<string, string> = {};
    for (const [combo, action] of Object.entries(parsed as Record<string, unknown>)) if (typeof action === 'string') out[combo] = action;
    return out;
  } catch {
    return {};
  }
}

interface KeyedCommands {
  all(): Array<{ id: string }>;
  get(id: string): unknown;
  overrides(): Record<string, string[]>;
  setKeys(id: string, keys: string[]): void;
}

export function persistShortcuts(registry: KeyedCommands): void {
  const kept = { ...loadPrefs().shortcuts };
  for (const id of Object.keys(kept)) if (registry.get(id)) delete kept[id];
  updatePrefs({ shortcuts: { ...kept, ...registry.overrides() } });
}

export function applyShortcuts(registry: KeyedCommands): void {
  for (const [id, keys] of Object.entries(loadPrefs().shortcuts)) if (registry.get(id)) registry.setKeys(id, keys);
}
