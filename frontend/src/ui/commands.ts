import { isMac } from './dom';

export interface CommandParam {
  type: 'string' | 'number' | 'integer' | 'boolean' | 'array' | 'object';
  required?: boolean;
  enum?: ReadonlyArray<string | number>;
  min?: number;
  max?: number;
  description?: string;
}

export interface Command {
  id: string;
  label: string | (() => string);
  keys?: string[];
  icon?: string;
  run: (params?: Record<string, unknown>) => unknown;
  enabled?: () => boolean;
  checked?: () => boolean;
  group?: string;
  description?: string;
  when?: string;
  params?: Record<string, CommandParam>;
  mutates?: boolean;
  risky?: boolean;
}

const CODE_KEYS: Record<string, string> = {
  BracketLeft: '[', BracketRight: ']', Equal: '=', Minus: '-', NumpadAdd: '=', NumpadSubtract: '-',
  Backspace: 'Backspace', Delete: 'Delete', Enter: 'Enter', Escape: 'Escape', Space: 'Space',
  ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right', Slash: '/', Tab: 'Tab',
};

const KEY_ALIASES: Record<string, string> = { ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right', Del: 'Delete', Esc: 'Escape', ' ': 'Space' };

const ORDER = ['Alt', 'Shift', 'Mod'];

/** The canonical form of a combo: modifiers in the order Alt, Shift, Mod, then the key; Ctrl and Cmd read as Mod. */
export function normalizeCombo(combo: string): string {
  const plus = combo.endsWith('++');
  const parts = (plus ? combo.slice(0, -2) : combo).split('+').filter(Boolean);
  const mods = new Set<string>();
  let key = plus ? '+' : '';
  for (const p of parts) {
    const m = p === 'Ctrl' || p === 'Cmd' || p === 'Control' || p === 'Meta' ? 'Mod' : p;
    if (ORDER.includes(m)) mods.add(m);
    else key = KEY_ALIASES[p] ?? (p.length === 1 ? p.toUpperCase() : p);
  }
  return [...ORDER.filter((m) => mods.has(m)), key].join('+');
}

/** The combo text a key event stands for, e.g. 'Alt+Shift+Mod+S'; empty for a lone modifier. */
export function comboOf(e: KeyboardEvent): string {
  if (['Control', 'Meta', 'Alt', 'Shift'].includes(e.key)) return '';
  let key: string;
  if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
  else if (/^Digit\d$/.test(e.code)) key = e.code.slice(5);
  else if (/^Numpad\d$/.test(e.code)) key = e.code.slice(6);
  else if (e.code in CODE_KEYS) key = CODE_KEYS[e.code];
  else key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  const mod = isMac ? e.metaKey : e.ctrlKey;
  return (e.altKey ? 'Alt+' : '') + (e.shiftKey ? 'Shift+' : '') + (mod ? 'Mod+' : '') + key;
}

const GLYPHS: Record<string, string> = {
  Mod: '⌘', Shift: '⇧', Alt: '⌥', Backspace: '⌫', Delete: '⌦', Enter: '↩',
  Up: '↑', Down: '↓', Left: '←', Right: '→', Escape: 'Esc',
};

export function formatKeys(combo: string): string {
  const parts = combo.endsWith('++') ? [...combo.slice(0, -2).split('+'), '+'] : combo.split('+');
  if (isMac) return parts.map((p) => GLYPHS[p] ?? p).join('');
  return parts.map((p) => (p === 'Mod' ? 'Ctrl' : p)).join('+');
}

export class Commands {
  onRun: ((cmd: Command, label: string) => void) | null = null;
  gate: ((cmd: Command) => boolean) | null = null;
  blocked: ((cmd: Command) => string) | null = null;
  private readonly map = new Map<string, Command>();
  private readonly byKey = new Map<string, Command>();
  private readonly defaults = new Map<string, string[]>();
  private readonly listeners = new Set<() => void>();

  add(...cmds: Command[]): void {
    for (const cmd of cmds) {
      if (this.map.has(cmd.id)) throw new Error(`duplicate command: ${cmd.id}`);
      cmd.keys = (cmd.keys ?? []).map(normalizeCombo);
      this.map.set(cmd.id, cmd);
      this.defaults.set(cmd.id, [...cmd.keys]);
      for (const k of cmd.keys) if (!this.byKey.has(k)) this.byKey.set(k, cmd);
    }
    this.notify();
  }

  remove(id: string): void {
    const c = this.map.get(id);
    if (!c) return;
    this.map.delete(id);
    for (const k of c.keys ?? []) if (this.byKey.get(k) === c) this.byKey.delete(k);
    this.notify();
  }

  /** Calls `fn` after commands are added or removed; returns the function that stops it. */
  onChange(fn: () => void): () => void {
    this.listeners.add(fn);
    return () => {
      this.listeners.delete(fn);
    };
  }

  notify(): void {
    for (const fn of this.listeners) fn();
  }

  setKeys(id: string, keys: string[]): void {
    const cmd = this.map.get(id);
    if (!cmd) return;
    const next = keys.map(normalizeCombo);
    for (const k of cmd.keys ?? []) if (this.byKey.get(k) === cmd) this.byKey.delete(k);
    for (const k of next) {
      const had = this.byKey.get(k);
      if (had && had !== cmd) had.keys = (had.keys ?? []).filter((x) => x !== k);
      this.byKey.set(k, cmd);
    }
    cmd.keys = next;
  }

  forCombo(combo: string): Command | undefined {
    return this.byKey.get(normalizeCombo(combo));
  }

  defaultKeys(id: string): string[] {
    return [...(this.defaults.get(id) ?? [])];
  }

  overrides(): Record<string, string[]> {
    const out: Record<string, string[]> = {};
    for (const [id, cmd] of this.map) {
      const d = this.defaults.get(id) ?? [];
      const k = cmd.keys ?? [];
      if (d.length !== k.length || d.some((x, i) => x !== k[i])) out[id] = [...k];
    }
    return out;
  }

  resetKeys(): void {
    this.byKey.clear();
    for (const [id, cmd] of this.map) {
      cmd.keys = [...(this.defaults.get(id) ?? [])];
      for (const k of cmd.keys) if (!this.byKey.has(k)) this.byKey.set(k, cmd);
    }
  }

  get(id: string): Command | undefined {
    return this.map.get(id);
  }

  all(): Command[] {
    return [...this.map.values()];
  }

  label(cmd: Command): string {
    return typeof cmd.label === 'function' ? cmd.label() : cmd.label;
  }

  enabled(cmd: Command): boolean {
    return !this.why(cmd) && (!this.gate || this.gate(cmd)) && (cmd.enabled ? cmd.enabled() : true);
  }

  why(cmd: Command): string {
    return this.blocked ? this.blocked(cmd) : '';
  }

  keyLabel(cmd: Command): string {
    return cmd.keys && cmd.keys.length ? formatKeys(cmd.keys[0]) : '';
  }

  forEvent(e: KeyboardEvent): Command | undefined {
    const combo = comboOf(e);
    return combo ? this.byKey.get(combo) : undefined;
  }

  run(id: string, params?: Record<string, unknown>): boolean {
    const cmd = this.map.get(id);
    if (!cmd || !this.enabled(cmd)) return false;
    const label = this.label(cmd);
    cmd.run(params);
    this.onRun?.(cmd, label);
    return true;
  }

  async call(id: string, params?: Record<string, unknown>): Promise<{ ran: boolean; result?: unknown }> {
    const cmd = this.map.get(id);
    if (!cmd || !this.enabled(cmd)) return { ran: false };
    const label = this.label(cmd);
    const result = await cmd.run(params);
    this.onRun?.(cmd, label);
    return { ran: true, result };
  }
}

export const commands = new Commands();
