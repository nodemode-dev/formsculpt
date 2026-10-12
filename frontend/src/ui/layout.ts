/**
 * Interface customization: every panel section folds away under its heading, the panel can sit on either side of the
 * viewport, and the panel can be drawn smaller or larger. Choices are kept in this browser (a blocked storage keeps
 * them for the session only). A fold is the person's own action, so it never counts as an unexpected layout shift.
 */

const STORE = 'formsculptor.layout';

interface LayoutPrefs {
  collapsed: string[];
  panelLeft: boolean;
  size: string;
}

function load(): LayoutPrefs {
  try {
    const raw = localStorage.getItem(STORE);
    const p = raw ? (JSON.parse(raw) as Partial<LayoutPrefs>) : {};
    return { collapsed: Array.isArray(p.collapsed) ? p.collapsed : [], panelLeft: p.panelLeft === true, size: typeof p.size === 'string' ? p.size : 'normal' };
  } catch {
    return { collapsed: [], panelLeft: false, size: 'normal' };
  }
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
}
