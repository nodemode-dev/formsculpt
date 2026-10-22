/**
 * Floating toolbars the person builds: each is a named strip of command buttons over the viewport, moved by its grip
 * and laid out across or down. A button is any registry command (data-cmd, so it runs the command and shows its pressed
 * and disabled state like every other command button). Toolbars are kept in this browser and travel with a layout
 * export.
 */

import { el, setAttr, setText } from './dom';
import { t } from './i18n';

export const TOOLBARS_STORE = 'formsculpt.toolbars';

export interface ToolbarDef {
  id: string;
  name: string;
  /** Position of the top left corner as a share of the stage's width and height, so it survives resizing. */
  x: number;
  y: number;
  vertical: boolean;
  items: string[];
}

/** Reads toolbars from a JSON value, dropping anything malformed. */
export function parseToolbars(v: unknown): ToolbarDef[] {
  if (!Array.isArray(v)) return [];
  const clamp = (n: unknown): number => (typeof n === 'number' && Number.isFinite(n) ? Math.max(0, Math.min(0.95, n)) : 0.3);
  return v
    .filter((d): d is Record<string, unknown> => !!d && typeof d === 'object')
    .map((d, i) => ({
      id: typeof d.id === 'string' && d.id ? d.id : `tb${i + 1}`,
      name: typeof d.name === 'string' && d.name ? d.name.slice(0, 60) : `Toolbar ${i + 1}`,
      x: clamp(d.x),
      y: clamp(d.y),
      vertical: d.vertical === true,
      items: Array.isArray(d.items) ? d.items.filter((s): s is string => typeof s === 'string').slice(0, 64) : [],
    }));
}

interface CommandLike {
  label: string | (() => string);
}

export class Toolbars {
  private defs: ToolbarDef[] = [];
  private readonly nodes = new Map<string, HTMLElement>();
  /** The toolbar a grip menu or an item menu was opened on, and the item, for the toolbar commands. */
  target: { toolbar: string; item: number } | null = null;
  /** The toolbar new items go to: the last one made, moved or added to. */
  current: string | null = null;
  onGripMenu: ((toolbar: string, x: number, y: number) => void) | null = null;
  onItemMenu: ((toolbar: string, item: number, x: number, y: number) => void) | null = null;

  constructor(private stage: HTMLElement, private lookup: (id: string) => CommandLike | undefined) {
    this.load();
  }

  /** Reads the toolbars kept in this browser and draws them. */
  load(): void {
    try {
      this.defs = parseToolbars(JSON.parse(localStorage.getItem(TOOLBARS_STORE) ?? '[]'));
    } catch {
      this.defs = [];
    }
    this.current = this.defs.at(-1)?.id ?? null;
    this.draw();
  }

  private save(): void {
    try {
      localStorage.setItem(TOOLBARS_STORE, JSON.stringify(this.defs));
    } catch {
      // storage blocked; the toolbars last for this session
    }
  }

  get list(): readonly ToolbarDef[] {
    return this.defs;
  }

  private def(id: string | null): ToolbarDef | undefined {
    return this.defs.find((d) => d.id === id);
  }

  /** A new empty toolbar near the middle of the stage; returns its id. */
  create(name: string): string {
    let n = this.defs.length + 1;
    while (this.defs.some((d) => d.id === `tb${n}`)) n++;
    const id = `tb${n}`;
    this.defs.push({ id, name: name.trim().slice(0, 60) || `Toolbar ${n}`, x: 0.35, y: 0.12 + 0.06 * (this.defs.length % 6), vertical: false, items: [] });
    this.current = id;
    this.save();
    this.draw();
    return id;
  }

  /** Adds a command to a toolbar (the current one, or a new one when there is none); false when it is already there. */
  add(command: string, toolbar: string | null = this.current): boolean {
    let d = this.def(toolbar) ?? this.def(this.current);
    if (!d) d = this.def(this.create(t('Toolbar')));
    if (!d || d.items.includes(command)) return false;
    d.items.push(command);
    this.current = d.id;
    this.save();
    this.draw();
    return true;
  }

  removeItem(toolbar: string, item: number): void {
    const d = this.def(toolbar);
    if (!d || item < 0 || item >= d.items.length) return;
    d.items.splice(item, 1);
    this.save();
    this.draw();
  }

  rename(toolbar: string, name: string): void {
    const d = this.def(toolbar);
    if (!d || !name.trim()) return;
    d.name = name.trim().slice(0, 60);
    this.save();
    this.draw();
  }

  turn(toolbar: string): void {
    const d = this.def(toolbar);
    if (!d) return;
    d.vertical = !d.vertical;
    this.save();
    this.draw();
  }

  remove(toolbar: string): void {
    this.defs = this.defs.filter((d) => d.id !== toolbar);
    if (this.current === toolbar) this.current = this.defs.at(-1)?.id ?? null;
    this.save();
    this.draw();
  }

  /** Redraws every toolbar from the definitions, reusing each toolbar's element so nothing flickers. */
  draw(): void {
    for (const [id, node] of this.nodes) {
      if (!this.def(id)) {
        node.remove();
        this.nodes.delete(id);
      }
    }
    for (const d of this.defs) {
      let node = this.nodes.get(d.id);
      if (!node) {
        node = this.build(d.id);
        this.stage.append(node);
        this.nodes.set(d.id, node);
      }
      node.classList.toggle('vertical', d.vertical);
      node.style.left = `${(d.x * 100).toFixed(2)}%`;
      node.style.top = `${(d.y * 100).toFixed(2)}%`;
      const grip = node.firstElementChild as HTMLElement;
      setAttr(grip, 'title', d.name);
      setText(grip.querySelector('.ftb-name') as HTMLElement, d.name);
      const buttons = Array.from(node.querySelectorAll<HTMLButtonElement>('.ftb-btn'));
      d.items.forEach((cmd, i) => {
        let b = buttons[i];
        if (!b) {
          b = el('button', 'ftb-btn');
          b.type = 'button';
          node.append(b);
        }
        const c = this.lookup(cmd);
        const label = c ? t(typeof c.label === 'function' ? c.label() : c.label) : cmd;
        b.dataset.cmd = cmd;
        b.dataset.index = String(i);
        setAttr(b, 'title', label);
        setText(b, label);
      });
      for (const b of buttons.slice(d.items.length)) b.remove();
      node.classList.toggle('empty', d.items.length === 0);
    }
    this.onDraw?.();
  }

  /** Runs after every redraw, so new buttons pick up their pressed and disabled state at once. */
  onDraw: (() => void) | null = null;

  private build(id: string): HTMLElement {
    const node = el('div', 'ftb');
    node.dataset.toolbar = id;
    const grip = el('div', 'ftb-grip');
    grip.append(el('span', 'ftb-dots', ''), el('span', 'ftb-name', ''));
    node.append(grip);
    // The grip moves the toolbar; the move is kept as a share of the stage when the press ends.
    grip.addEventListener('pointerdown', (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      grip.setPointerCapture(e.pointerId);
      const box = this.stage.getBoundingClientRect();
      const start = { x: e.clientX, y: e.clientY, left: node.offsetLeft, top: node.offsetTop };
      const move = (m: PointerEvent): void => {
        const left = Math.max(0, Math.min(box.width - 40, start.left + m.clientX - start.x));
        const top = Math.max(0, Math.min(box.height - 30, start.top + m.clientY - start.y));
        node.style.left = `${left}px`;
        node.style.top = `${top}px`;
      };
      const up = (): void => {
        grip.removeEventListener('pointermove', move);
        grip.removeEventListener('pointerup', up);
        grip.removeEventListener('pointercancel', up);
        const d = this.def(id);
        if (d) {
          d.x = Math.max(0, Math.min(0.95, node.offsetLeft / Math.max(1, box.width)));
          d.y = Math.max(0, Math.min(0.95, node.offsetTop / Math.max(1, box.height)));
          this.current = id;
          this.save();
          this.draw();
        }
      };
      grip.addEventListener('pointermove', move);
      grip.addEventListener('pointerup', up);
      grip.addEventListener('pointercancel', up);
    });
    grip.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      this.target = { toolbar: id, item: -1 };
      this.current = id;
      this.onGripMenu?.(id, e.clientX, e.clientY);
    });
    node.addEventListener('contextmenu', (e) => {
      const b = (e.target as HTMLElement).closest<HTMLButtonElement>('.ftb-btn');
      if (!b) return;
      e.preventDefault();
      e.stopPropagation();
      const item = Number(b.dataset.index);
      this.target = { toolbar: id, item };
      this.onItemMenu?.(id, item, e.clientX, e.clientY);
    });
    return node;
  }
}
