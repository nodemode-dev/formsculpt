/**
 * Custom hotkeys: every key the window answers comes from one map of shortcut to action. The defaults are the keys
 * the command registry declares plus the window's own (views, framing, file actions); the person's changes are kept
 * as overrides in this browser and laid over the defaults, so a reset brings the registry's keys back.
 */

export interface CatalogueCommand {
  name: string;
  label: string;
  keys: string[];
  annotations?: { readOnlyHint?: boolean };
  inputSchema?: { properties?: Record<string, unknown>; required?: string[] };
}

export interface KeyAction {
  id: string;
  label: string;
}

const STORE = 'formsculptor.keys';

/** Commands whose declared keys each carry their own arguments, by key position. */
const KEY_ARGS: Record<string, string[]> = { multires_set_level: ['up', 'down'] };

export const WINDOW_ACTIONS: KeyAction[] = [
  { id: 'view:front', label: 'View front' },
  { id: 'view:back', label: 'View back' },
  { id: 'view:right', label: 'View right' },
  { id: 'view:left', label: 'View left' },
  { id: 'view:top', label: 'View top' },
  { id: 'view:bottom', label: 'View bottom' },
  { id: 'view:ortho', label: 'Toggle orthographic' },
  { id: 'view:frame', label: 'Frame active object' },
  { id: 'file:save', label: 'Save project' },
  { id: 'file:open', label: 'Open file' },
  { id: 'file:export', label: 'Export' },
];

const WINDOW_KEYS: Record<string, string> = {
  '1': 'view:front',
  'Mod+1': 'view:back',
  '3': 'view:right',
  'Mod+3': 'view:left',
  '7': 'view:top',
  'Mod+7': 'view:bottom',
  '5': 'view:ortho',
  F: 'view:frame',
  'Mod+S': 'file:save',
  'Mod+O': 'file:open',
  'Mod+E': 'file:export',
  Delete: 'cmd:delete_object',
  Backspace: 'cmd:delete_object',
  'Mod+Z': 'cmd:undo',
  'Mod+Shift+Z': 'cmd:redo',
  'Mod+Y': 'cmd:redo',
};

/** The shortcut a key press makes, as Mod+Alt+Shift+Key (Mod is Ctrl, or Cmd on a Mac); null for a lone modifier. */
export function comboOf(e: KeyboardEvent): string | null {
  if (['Control', 'Meta', 'Alt', 'Shift'].includes(e.key)) return null;
  let key = e.key;
  if (/^Digit\d$/.test(e.code)) key = e.code.slice(5);
  else if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
  else if (key.length === 1) key = key.toUpperCase();
  else if (key === ' ') key = 'Space';
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push('Mod');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  parts.push(key);
  return parts.join('+');
}

/** Commands a key can run: ones that change something and need no argument beyond the active object's id. */
export function commandActions(commands: CatalogueCommand[]): KeyAction[] {
  const out: KeyAction[] = [];
  for (const c of commands) {
    if (c.annotations?.readOnlyHint) continue;
    if ((c.inputSchema?.required ?? []).some((r) => r !== 'id')) continue;
    const steps = KEY_ARGS[c.name];
    if (steps) for (const step of steps) out.push({ id: `cmd:${c.name}:${step}`, label: `${c.label} (${step})` });
    else out.push({ id: `cmd:${c.name}`, label: c.label });
  }
  return out;
}

export class Hotkeys {
  private defaults = new Map<string, string>();
  private overrides: Record<string, string> = {};

  constructor() {
    for (const [combo, action] of Object.entries(WINDOW_KEYS)) this.defaults.set(combo, action);
    try {
      const raw = localStorage.getItem(STORE);
      const parsed: unknown = raw ? JSON.parse(raw) : {};
      if (parsed && typeof parsed === 'object') this.overrides = parsed as Record<string, string>;
    } catch {
      // storage blocked or unreadable; the defaults stand for this session
    }
  }

  /** Adds the keys the registry declares; the window's own defaults keep the shortcuts they already hold. */
  addCatalogue(commands: CatalogueCommand[]): void {
    for (const c of commands) {
      c.keys.forEach((combo, i) => {
        if (this.defaults.has(combo)) return;
        const steps = KEY_ARGS[c.name];
        this.defaults.set(combo, steps ? `cmd:${c.name}:${steps[i] ?? steps[0]}` : `cmd:${c.name}`);
      });
    }
  }

  /** The action a shortcut runs, after the person's changes. */
  action(combo: string): string | null {
    if (combo in this.overrides) return this.overrides[combo] || null;
    return this.defaults.get(combo) ?? null;
  }

  /** Every shortcut that runs an action. */
  combosFor(action: string): string[] {
    const combos = new Set<string>([...this.defaults.keys(), ...Object.keys(this.overrides)]);
    return [...combos].filter((c) => this.action(c) === action).sort();
  }

  bind(combo: string, action: string): void {
    this.overrides[combo] = action;
    this.save();
  }

  clear(action: string): void {
    for (const combo of this.combosFor(action)) this.overrides[combo] = '';
    this.save();
  }

  reset(): void {
    this.overrides = {};
    this.save();
  }

  private save(): void {
    try {
      localStorage.setItem(STORE, JSON.stringify(this.overrides));
    } catch {
      // storage blocked; the change lasts for this session
    }
  }
}
