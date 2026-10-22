/**
 * Interface customization: every panel section folds away under its heading, the panel can sit on either side of the
 * viewport, and the panel can be drawn smaller or larger. Choices are kept in this browser (a blocked storage keeps
 * them for the session only). A fold is the person's own action, so it never counts as an unexpected layout shift.
 */

const STORE = 'formsculpt.layout';

interface LayoutPrefs {
  collapsed: string[];
  panelLeft: boolean;
  size: string;
  /** Panel buttons pinned to the tool rail, as command ids (older saves hold "Section/Label" and are migrated on load). */
  rail: string[];
}

/** The rail a first start shows, so the panel layout never opens on an empty strip: the everyday sculpting tools,
 * the mask and the transform gizmo. A rail the person has changed (even emptied) is kept as it is. */
export const DEFAULT_RAIL = ['draw', 'clay', 'smooth', 'grab', 'inflate', 'crease', 'flatten', 'pinch', 'mask', 'gizmo'].map((b) => `tool.brush.${b}`);

function load(): LayoutPrefs {
  try {
    const raw = localStorage.getItem(STORE);
    const p = raw ? (JSON.parse(raw) as Partial<LayoutPrefs>) : {};
    const strings = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : []);
    return { collapsed: strings(p.collapsed), panelLeft: p.panelLeft === true, size: typeof p.size === 'string' ? p.size : 'normal', rail: Array.isArray(p.rail) ? strings(p.rail) : [...DEFAULT_RAIL] };
  } catch {
    return { collapsed: [], panelLeft: false, size: 'normal', rail: [...DEFAULT_RAIL] };
  }
}

/** Two letters for a rail button: the first two of one word, or the first of each of two. */
function short(label: string): string {
  const words = label.split(/\s+/).filter(Boolean);
  if (words.length > 1) return words[0][0].toUpperCase() + words[1][0].toLowerCase();
  return label.slice(0, 2);
}

function save(p: LayoutPrefs): void {
  try {
    localStorage.setItem(STORE, JSON.stringify(p));
  } catch {
    // storage blocked; the layout lasts for this session
  }
}

export class PanelLayout {
  private prefs = load();

  constructor(private root: HTMLElement, private panel: HTMLElement) {}

  /** Turns each section heading into a fold toggle and restores the folds, side and size kept last time. */
  apply(): void {
    const heads = [...this.panel.querySelectorAll<HTMLHeadingElement>(':scope > h2')];
    for (const h of heads) {
      const name = h.textContent ?? '';
      h.classList.add('fold');
      h.tabIndex = 0;
      h.setAttribute('role', 'button');
      // A distinct name, so the heading never shares one with a tool button (Paint, Build, Layers).
      h.setAttribute('aria-label', `${name} section`);
      const toggle = (): void => this.fold(h, !h.classList.contains('folded'));
      h.addEventListener('click', toggle);
      h.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          toggle();
        }
      });
      this.fold(h, this.prefs.collapsed.includes(name), false);
    }
    this.setSide(this.prefs.panelLeft, false);
    this.setSize(this.prefs.size, false);
  }

  /** Hides or shows everything from a heading to the next one. */
  private fold(h: HTMLHeadingElement, folded: boolean, keep = true): void {
    h.classList.toggle('folded', folded);
    h.setAttribute('aria-expanded', folded ? 'false' : 'true');
    for (let n = h.nextElementSibling; n && n.tagName !== 'H2'; n = n.nextElementSibling) {
      // Hidden inputs stay hidden whatever the fold.
      if (n instanceof HTMLInputElement && n.type === 'file') continue;
      (n as HTMLElement).classList.toggle('fold-hidden', folded);
    }
    if (keep) {
      const name = h.textContent ?? '';
      const set = new Set(this.prefs.collapsed);
      if (folded) set.add(name);
      else set.delete(name);
      this.prefs.collapsed = [...set];
      save(this.prefs);
    }
  }

  /** Re-reads the kept layout (after an import) and applies its folds, side, size and rail; listeners stay as they are. */
  reload(): void {
    this.prefs = load();
    for (const h of Array.from(this.panel.querySelectorAll<HTMLHeadingElement>(':scope > h2'))) this.fold(h, this.prefs.collapsed.includes(h.textContent ?? ''), false);
    this.setSide(this.prefs.panelLeft, false);
    this.setSize(this.prefs.size, false);
    this.drawRail();
  }

  get panelLeft(): boolean {
    return this.prefs.panelLeft;
  }

  get size(): string {
    return this.prefs.size;
  }

  setSide(left: boolean, keep = true): void {
    this.root.classList.toggle('panel-left', left);
    this.prefs.panelLeft = left;
    if (keep) save(this.prefs);
  }

  setSize(size: string, keep = true): void {
    for (const s of ['small', 'large']) this.panel.classList.toggle(`panel-${s}`, s === size);
    this.prefs.size = size;
    if (keep) save(this.prefs);
  }

  private sources = new Map<string, HTMLButtonElement>();
  private railEl: HTMLElement | null = null;

  /**
   * Makes the tool rail a toolbar the person builds: the context menu (right button, or the keyboard's menu key) on any
   * panel button pins it to the rail or takes it off, and a rail button presses the panel button it stands for, so it
   * does exactly what that button does and shows the same pressed and disabled state.
   */
  onPinMenu: ((id: string, x: number, y: number) => void) | null = null;
  onDraw: (() => void) | null = null;
  private told: (text: string) => void = () => undefined;

  attachRail(rail: HTMLElement, told: (text: string) => void): void {
    this.railEl = rail;
    this.told = told;
    let section = '';
    const moved = new Map<string, string>();
    for (const n of this.panel.children) {
      if (n.tagName === 'H2') {
        section = n.textContent ?? '';
        continue;
      }
      for (const b of n.tagName === 'BUTTON' ? [n as HTMLButtonElement] : [...n.querySelectorAll('button')]) {
        const id = b.dataset.cmd;
        if (!id || this.sources.has(id)) continue;
        moved.set(`${section}/${b.textContent ?? ''}`, id);
        this.sources.set(id, b);
        b.addEventListener('contextmenu', (e) => {
          e.preventDefault();
          if (this.onPinMenu) this.onPinMenu(id, e.clientX, e.clientY);
          else this.togglePin(id);
        });
      }
    }
    const migrated = [...new Set(this.prefs.rail.map((k) => moved.get(k) ?? k))];
    if (migrated.some((k, i) => k !== this.prefs.rail[i]) || migrated.length !== this.prefs.rail.length) {
      this.prefs.rail = migrated;
      save(this.prefs);
    }
    this.drawRail();
  }

  isPinned(id: string): boolean {
    return this.prefs.rail.includes(id);
  }

  togglePin(id: string): void {
    const pinned = this.isPinned(id);
    this.prefs.rail = pinned ? this.prefs.rail.filter((k) => k !== id) : [...this.prefs.rail, id];
    save(this.prefs);
    this.drawRail();
    const label = this.sources.get(id)?.textContent ?? id;
    this.told(pinned ? `${label} taken off the rail` : `${label} pinned to the rail`);
  }

  canPin(id: string): boolean {
    return this.sources.has(id);
  }

  get pinned(): number {
    return this.prefs.rail.filter((k) => this.sources.has(k)).length;
  }

  clearRail(): void {
    this.prefs.rail = [];
    save(this.prefs);
    this.drawRail();
  }

  private drawRail(): void {
    const rail = this.railEl;
    if (!rail) return;
    rail.replaceChildren();
    for (const key of this.prefs.rail) {
      const source = this.sources.get(key);
      if (!source) continue;
      const label = source.textContent ?? '';
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'rail-btn';
      b.textContent = short(label);
      b.dataset.cmd = key;
      b.setAttribute('aria-label', label);
      b.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        this.prefs.rail = this.prefs.rail.filter((k) => k !== key);
        save(this.prefs);
        this.drawRail();
      });
      rail.append(b);
    }
    this.onDraw?.();
  }
}
