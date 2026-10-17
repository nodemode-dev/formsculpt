/**
 * Bindings for the pen's buttons and for gamepads, with tap against hold: every input runs one command when tapped
 * and another when held. A held tool command is a temporary tool: it is chosen when the hold starts and the tool before
 * it comes back when the button is let go (the pen's barrel button held while drawing smooths, say). A held command that
 * is not a tool runs once when the hold is recognised. An input with nothing bound to hold runs its tap at once, so it
 * never waits. Gamepad sticks turn and move the camera. Bindings are kept in this browser and travel in layout files.
 */

import { Dialog } from './dialog';
import { el, setText } from './dom';
import { t } from './i18n';

export const BINDINGS_STORE = 'formsculptor.bindings';
/** A press longer than this is a hold. */
export const HOLD_MS = 300;

export interface Binding {
  tap: string;
  hold: string;
}
export type Bindings = Record<string, Binding>;

/** Inputs by id: the pen's barrel button and eraser end, and the 16 buttons of a standard gamepad. */
export const INPUTS: [string, string][] = [
  ['pen.barrel', 'Pen button'],
  ['pen.eraser', 'Pen eraser'],
  ['pad.0', 'Gamepad A'],
  ['pad.1', 'Gamepad B'],
  ['pad.2', 'Gamepad X'],
  ['pad.3', 'Gamepad Y'],
  ['pad.4', 'Left bumper'],
  ['pad.5', 'Right bumper'],
  ['pad.6', 'Left trigger'],
  ['pad.7', 'Right trigger'],
  ['pad.8', 'Back button'],
  ['pad.9', 'Start button'],
  ['pad.10', 'Left stick press'],
  ['pad.11', 'Right stick press'],
  ['pad.12', 'D-pad up'],
  ['pad.13', 'D-pad down'],
  ['pad.14', 'D-pad left'],
  ['pad.15', 'D-pad right'],
];

export const DEFAULT_BINDINGS: Bindings = {
  'pen.barrel': { tap: 'history.undo', hold: 'tool.brush.smooth' },
  'pen.eraser': { tap: '', hold: 'tool.brush.mask' },
  'pad.0': { tap: 'history.undo', hold: '' },
  'pad.1': { tap: 'history.redo', hold: '' },
  'pad.2': { tap: 'sculpt.invert', hold: 'tool.brush.smooth' },
  'pad.3': { tap: 'view.frame', hold: 'view.ortho' },
  'pad.4': { tap: 'brush.radius_down', hold: '' },
  'pad.5': { tap: 'brush.radius_up', hold: '' },
  'pad.6': { tap: '', hold: 'tool.brush.mask' },
  'pad.7': { tap: '', hold: 'tool.brush.smooth' },
  'pad.8': { tap: 'view.tablet_layout', hold: '' },
  'pad.9': { tap: 'app.palette', hold: '' },
  'pad.10': { tap: 'sculpt.mirror_x', hold: '' },
  'pad.11': { tap: 'view.wireframe', hold: '' },
  'pad.12': { tap: 'brush.strength_up', hold: '' },
  'pad.13': { tap: 'brush.strength_down', hold: '' },
  'pad.14': { tap: 'view.left', hold: '' },
  'pad.15': { tap: 'view.right', hold: '' },
};

/** Reads bindings from a JSON value over the defaults, dropping anything malformed. */
export function parseBindings(v: unknown): Bindings {
  const out: Bindings = {};
  for (const [id] of INPUTS) out[id] = { ...DEFAULT_BINDINGS[id] };
  if (!v || typeof v !== 'object' || Array.isArray(v)) return out;
  for (const [id, b] of Object.entries(v as Record<string, unknown>)) {
    if (!out[id] || !b || typeof b !== 'object') continue;
    const r = b as Record<string, unknown>;
    if (typeof r.tap === 'string') out[id].tap = r.tap;
    if (typeof r.hold === 'string') out[id].hold = r.hold;
  }
  return out;
}

const isTool = (id: string): boolean => id.startsWith('tool.brush.');

export interface BindingHost {
  /** Runs a registry command; false when it is unknown or disabled. */
  run: (id: string) => boolean;
  /** The active tool's brush kind. */
  tool: () => string;
  /** Turns, moves and zooms the camera (gamepad sticks), then redraws. */
  orbit: (dx: number, dy: number) => void;
  pan: (dx: number, dy: number) => void;
  zoom: (factor: number) => void;
  /** Tells the person what an input did (a short note). */
  note: (text: string) => void;
}

interface Press {
  at: number;
  /** The tool to come back to when a held tool is let go. */
  back: string | null;
  held: boolean;
  timer: number;
}

export class InputBindings {
  bindings: Bindings = parseBindings(null);
  private readonly presses = new Map<string, Press>();
  private frame = 0;
  private padButtons: boolean[] = [];
  /** Names of the gamepads connected now. */
  readonly pads = new Map<number, string>();
  onPads: (() => void) | null = null;

  constructor(private host: BindingHost) {
    this.load();
    window.addEventListener('gamepadconnected', (e) => {
      this.pads.set(e.gamepad.index, e.gamepad.id);
      this.onPads?.();
      this.poll();
    });
    window.addEventListener('gamepaddisconnected', (e) => {
      this.pads.delete(e.gamepad.index);
      this.onPads?.();
    });
  }

  load(): void {
    try {
      this.bindings = parseBindings(JSON.parse(localStorage.getItem(BINDINGS_STORE) ?? 'null'));
    } catch {
      this.bindings = parseBindings(null);
    }
  }

  save(): void {
    try {
      localStorage.setItem(BINDINGS_STORE, JSON.stringify(this.bindings));
    } catch {
      // storage blocked; the bindings last for this session
    }
  }

  set(input: string, which: 'tap' | 'hold', command: string): void {
    const b = this.bindings[input];
    if (!b) return;
    b[which] = command;
    this.save();
  }

  reset(): void {
    this.bindings = parseBindings(null);
    this.save();
  }

  /**
   * An input went down: with nothing bound to hold its tap runs now; otherwise the hold starts after HOLD_MS (a tool is
   * taken up until the release, any other command runs once). `immediateHold` starts the hold at once (the pen touching
   * the surface with its button held, or the eraser end touching it). Returns whether a held tool is now in use.
   */
  press(input: string, immediateHold = false): boolean {
    const b = this.bindings[input];
    if (!b) return false;
    const already = this.presses.get(input);
    if (already) {
      // Pressed while hovering, then the pen touches down: the hold starts now if it has not yet.
      if (immediateHold && !already.held && b.hold) {
        window.clearTimeout(already.timer);
        already.held = true;
        if (isTool(b.hold)) {
          const before = this.host.tool();
          if (this.host.run(b.hold)) already.back = before;
        } else this.host.run(b.hold);
      }
      return already.back !== null;
    }
    const p: Press = { at: performance.now(), back: null, held: false, timer: 0 };
    this.presses.set(input, p);
    if (!b.hold) {
      if (!immediateHold && b.tap) this.host.run(b.tap);
      return false;
    }
    const startHold = (): void => {
      p.held = true;
      if (isTool(b.hold)) {
        const before = this.host.tool();
        if (this.host.run(b.hold)) p.back = before;
      } else this.host.run(b.hold);
    };
    if (immediateHold) {
      startHold();
      return p.back !== null;
    }
    p.timer = window.setTimeout(startHold, HOLD_MS);
    return false;
  }

  /** An input came up: a short press runs its tap (when a hold is bound), a held tool gives the tool before it back. */
  release(input: string): void {
    const p = this.presses.get(input);
    if (!p) return;
    this.presses.delete(input);
    window.clearTimeout(p.timer);
    const b = this.bindings[input];
    if (!b) return;
    if (!p.held) {
      if (b.hold && b.tap) this.host.run(b.tap);
      return;
    }
    if (p.back && this.host.tool() === b.hold.slice('tool.brush.'.length)) this.host.run(`tool.brush.${p.back}`);
  }

  isDown(input: string): boolean {
    return this.presses.has(input);
  }

  /** Reads every connected gamepad once a frame while one is connected: buttons through press and release, the left
   * stick turns the camera, the right stick moves it sideways and zooms. */
  private poll(): void {
    if (this.frame) return;
    const step = (): void => {
      this.frame = 0;
      const pads = (navigator.getGamepads?.() ?? []).filter((g): g is Gamepad => !!g);
      if (pads.length === 0) return;
      const pressed: boolean[] = [];
      let lx = 0;
      let ly = 0;
      let rx = 0;
      let ry = 0;
      const dead = (v: number): number => (Math.abs(v) < 0.18 ? 0 : v);
      for (const g of pads) {
        g.buttons.forEach((b, i) => {
          pressed[i] = pressed[i] || b.pressed || b.value > 0.5;
        });
        lx += dead(g.axes[0] ?? 0);
        ly += dead(g.axes[1] ?? 0);
        rx += dead(g.axes[2] ?? 0);
        ry += dead(g.axes[3] ?? 0);
      }
      for (let i = 0; i < 16; i++) {
        const now = !!pressed[i];
        if (now && !this.padButtons[i]) this.press(`pad.${i}`);
        else if (!now && this.padButtons[i]) this.release(`pad.${i}`);
        this.padButtons[i] = now;
      }
      if (lx || ly) this.host.orbit(lx * 9, ly * 9);
      if (rx) this.host.pan(rx * 9, 0);
      if (ry) this.host.zoom(Math.exp(ry * 0.03));
      this.frame = requestAnimationFrame(step);
    };
    this.frame = requestAnimationFrame(step);
  }
}

/** The bindings dialog: per input a command for tap and one for hold, chosen from the commands that make sense. */
export class BindingsDialog extends Dialog {
  private readonly table = el('div', 'bind-table');
  private readonly status = el('p', 'bind-status');

  constructor(
    private readonly inputs: InputBindings,
    private readonly choices: () => [string, string][],
  ) {
    // The title is the command's label (view.bindings), so it is in the string tables.
    super('Input Bindings', 560);
    this.body.append(this.status, this.table);
    this.button('Reset All', 'ghost', () => {
      this.inputs.reset();
      this.render();
    });
    this.button('Close', 'primary', () => this.close());
    inputs.onPads = () => this.say();
  }

  show(): void {
    this.render();
    this.open();
  }

  private say(): void {
    const names = [...this.inputs.pads.values()];
    setText(this.status, names.length ? `${t('Gamepad connected')}: ${names.join(', ')}` : t('No gamepad connected: press a button on it to wake it'));
  }

  private render(): void {
    this.say();
    const options = this.choices();
    const select = (input: string, which: 'tap' | 'hold', label: string): HTMLSelectElement => {
      const s = el('select', 'fs-input');
      s.setAttribute('aria-label', `${label}: ${which === 'tap' ? t('Tap') : t('Hold')}`);
      const none = el('option', '', t('Nothing'));
      none.value = '';
      s.append(none);
      for (const [id, name] of options) {
        const o = el('option', '', name);
        o.value = id;
        s.append(o);
      }
      const current = this.inputs.bindings[input]?.[which] ?? '';
      if (current && !options.some(([id]) => id === current)) {
        const o = el('option', '', current);
        o.value = current;
        s.append(o);
      }
      s.value = current;
      s.addEventListener('change', () => this.inputs.set(input, which, s.value));
      return s;
    };
    const rows: HTMLElement[] = [el('span', 'bind-head', t('Input')), el('span', 'bind-head', t('Tap')), el('span', 'bind-head', t('Hold'))];
    for (const [id, label] of INPUTS) {
      rows.push(el('span', 'bind-name', t(label)), select(id, 'tap', t(label)), select(id, 'hold', t(label)));
    }
    this.table.replaceChildren(...rows);
  }
}
