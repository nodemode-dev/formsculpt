/**
 * The ZBrush layout: a third face of the window, arranged as ZBrush's classic desktop interface is (researched from
 * Pixologic's ZBrush Starting Guide and Maxon's shortcut sheet; see the frontend side log). From the top: a title bar
 * (version and counts on the left, the window's notes in the middle, the layout presets on the right), the palette
 * menu bar with its palettes in alphabetical order, and the top shelf (the Draw, Move, Scale and Rotate modes, Rgb and
 * Mrgb, Zadd and Zsub, Draw Size and Z Intensity). Below it the left shelf holds the brush, stroke, alpha, material and
 * colour; the right shelf the view buttons (Persp, Floor, symmetry, Frame, the Move, Zoom3D and Rotate drag buttons,
 * PolyF, Transp, Solo); and the right tray shows one palette at a time, the Tool palette (with its SubTool list) by
 * default. The keys are ZBrush's while this layout is on.
 *
 * As with the tablet layout, nothing here is a feature of its own: buttons run registry commands (data-cmd, so their
 * pressed state follows the command) or write the panel control they stand for, and the palettes are the panel's own
 * sections. The look is formsculptor's, in ZBrush's grey; no ZBrush artwork is used.
 */

import { el, setAttr, setText } from './dom';
import { t } from './i18n';
import { DragValue, glyph, write } from './tablet-layout';

const STORE = 'formsculptor.zbrush';
const TRAY_STORE = 'formsculptor.zbrush.tray';

/** ZBrush's palettes that formsculptor has counterparts for, in ZBrush's alphabetical order: [name, the panel's
 * sections by their English headings]. */
const PALETTES: [string, string[]][] = [
  ['Brush', ['Brush']],
  ['Edit', ['History']],
  ['File', ['File']],
  ['Layer', ['Layers']],
  ['Light', ['Lighting']],
  ['Material', ['Paint']],
  ['Preferences', ['Keys', 'Engine']],
  ['Texture', ['UV']],
  ['Tool', ['Scene', 'Object', 'Add', 'Build', 'Topology', 'Multiresolution', 'Edit', 'Selection Sets', 'Active mesh']],
  ['Transform', ['View']],
  ['Zplugin', ['Plugins']],
];

export interface ZBrushParts {
  root: HTMLElement;
  panel: HTMLElement;
  brushButtons: HTMLButtonElement[];
  radius: HTMLInputElement;
  strength: HTMLInputElement;
  invert: HTMLInputElement;
  paintMaterial: HTMLInputElement;
  paintColor: HTMLInputElement;
  shading: HTMLSelectElement;
  stroke: HTMLSelectElement;
  alpha: HTMLSelectElement;
  gizmoMode: HTMLSelectElement;
  message: HTMLElement;
  version: string;
  /** Runs a registry command by id; whether it ran. */
  run: (id: string) => boolean;
  /** Whether a registry command is checked (a toggle that is on). */
  checked: (id: string) => boolean;
}

export interface ZBrushStats {
  vertices: number;
  triangles: number;
  active: string | null;
}

const fmt = (n: number): string => (n >= 1e6 ? `${(n / 1e6).toFixed(2)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(1)}k` : String(n));

/** A shelf button: a word, optionally with a glyph over it. */
function shelfButton(word: string, title: string, cls: string, icon?: string): HTMLButtonElement {
  const b = el('button', cls);
  b.type = 'button';
  b.title = t(title);
  b.setAttribute('aria-label', t(title));
  if (icon) b.append(glyph(icon));
  b.append(el('span', 'zb-word', t(word)));
  return b;
}

export class ZBrushLayout {
  private on = false;
  private timer = 0;
  private palette = 'Tool';
  private trayOpen = true;
  private lastSculpt = 'draw';
  private pointer = { x: 0, y: 0 };
  private readonly sectionOf = new Map<Element, string>();
  private readonly toolLabels = new Map<string, string>();
  private readonly title = el('div', 'zb-title');
  private readonly titleStats = el('span', 'zb-title-stats');
  private readonly titleNote = el('span', 'zb-title-note');
  private readonly menu = el('nav', 'zb-menu');
  private readonly top = el('div', 'zb-top');
  private readonly left = el('div', 'zb-left');
  private readonly right = el('div', 'zb-right');
  private readonly divider = el('button', 'zb-divider');
  private readonly trayHead = el('div', 'zb-tray-head');
  private readonly trayTitle = el('span', 'zb-tray-title');
  private readonly popup = el('div', 'zb-pop');
  private readonly quick = el('div', 'zb-quick');
  private readonly brushShown = el('span', 'zb-word');
  private readonly brushIcon = el('span', 'zb-brush-icon');
  private readonly layoutPick = el('select', 'zb-layout-pick');
  private readonly sliders: DragValue[] = [];
  private quickSlider: DragValue | null = null;
  private readonly modes = new Map<string, HTMLButtonElement>();
  private toolNow = '';

  /** Set by the app: the scene's counts for the title bar. */
  statsOf: (() => ZBrushStats) | null = null;
  /** Set by the app: the SubTool list (the Objects and Layers dock), shown at the top of the Tool palette. */
  subtools: HTMLElement | null = null;
  /** Set by the app: a drag on the right shelf's Move, Zoom3D or Rotate button, in pixels. */
  nav: ((kind: 'move' | 'zoom' | 'rotate', dx: number, dy: number) => void) | null = null;
  /** Set by the app: called when the layout is switched on or off. */
  onToggle: ((on: boolean) => void) | null = null;
  /** Set by the app: switch to a layout by name (zbrush, tablet, panel), and keep one as the default. */
  chooseLayout: ((name: string) => void) | null = null;
  setDefault: ((name: string) => void) | null = null;
  defaultLayout: (() => string) | null = null;

  constructor(private parts: ZBrushParts) {
    let section = '';
    for (const n of Array.from(parts.panel.children)) {
      if (n.tagName === 'H2') section = n.textContent ?? '';
      this.sectionOf.set(n, section);
    }
    for (const b of parts.brushButtons) this.toolLabels.set(b.dataset.brush ?? '', b.textContent ?? '');

    // Title bar: name, version and counts; the window's notes; the layout presets and the default.
    const name = el('span', 'zb-title-name', `formsculptor ${parts.version}`);
    for (const [value, label] of [['zbrush', 'ZBrush layout'], ['tablet', 'Tablet layout'], ['panel', 'Panel layout']]) {
      const o = el('option', '', t(label));
      o.value = value;
      this.layoutPick.append(o);
    }
    this.layoutPick.setAttribute('aria-label', t('Layout'));
    this.layoutPick.addEventListener('change', () => this.chooseLayout?.(this.layoutPick.value));
    const makeDefault = el('button', 'zb-title-btn', t('Set as default'));
    makeDefault.type = 'button';
    makeDefault.title = t('Open the window in this layout from now on');
    makeDefault.addEventListener('click', () => {
      this.setDefault?.('zbrush');
      this.syncDefault();
    });
    this.titleNote.setAttribute('role', 'status');
    this.titleNote.setAttribute('aria-live', 'polite');
    const presets = el('span', 'zb-title-right');
    presets.append(this.layoutPick, makeDefault);
    this.title.append(name, this.titleStats, this.titleNote, presets);

    // The palette menu bar: each palette shows in the right tray.
    for (const [p] of PALETTES) {
      const b = el('button', 'zb-menu-item', t(p));
      b.type = 'button';
      b.dataset.palette = p;
      b.setAttribute('aria-pressed', 'false');
      b.addEventListener('click', () => this.showPalette(p));
      this.menu.append(b);
    }

    // The top shelf: modes, paint channels, Zadd and Zsub, and the two sliders.
    const group = (): HTMLElement => {
      const g = el('div', 'zb-group');
      this.top.append(g);
      return g;
    };
    const modes = group();
    for (const [key, word, title, icon] of [['draw', 'Draw', 'Draw (Q)', 'draw'], ['move', 'Move', 'Move (W)', 'grab'], ['scale', 'Scale', 'Scale (E)', 'size'], ['rotate', 'Rotate', 'Rotate (R)', 'angle']] as const) {
      const b = shelfButton(word, title, 'zb-big', icon);
      b.setAttribute('aria-pressed', 'false');
      b.addEventListener('click', () => this.mode(key));
      this.modes.set(key, b);
      modes.append(b);
    }
    const paint = group();
    for (const [key, word, title] of [['mrgb', 'Mrgb', 'Paint colour and material'], ['rgb', 'Rgb', 'Paint colour']] as const) {
      const b = shelfButton(word, title, 'zb-small');
      b.setAttribute('aria-pressed', 'false');
      b.addEventListener('click', () => this.paintMode(key === 'mrgb'));
      this.modes.set(key, b);
      paint.append(b);
    }
    const z = group();
    for (const [key, word, title] of [['zadd', 'Zadd', 'Add to the surface'], ['zsub', 'Zsub', 'Carve into the surface']] as const) {
      const b = shelfButton(word, title, 'zb-small');
      b.setAttribute('aria-pressed', 'false');
      b.addEventListener('click', () => {
        if (parts.invert.checked !== (key === 'zsub')) parts.invert.click();
      });
      this.modes.set(key, b);
      z.append(b);
    }
    const sliders = group();
    sliders.classList.add('zb-sliders');
    const size = new DragValue(parts.radius, t('Draw Size'), false, 'tb-field zb-field');
    const intensity = new DragValue(parts.strength, t('Z Intensity'), false, 'tb-field zb-field');
    this.sliders.push(size, intensity);
    sliders.append(size.node, intensity.node);

    // The left shelf: the brush (opens the brush picker), stroke, alpha, material and colour.
    const brush = el('button', 'zb-swatch zb-brush');
    brush.type = 'button';
    brush.title = t('Brush (B)');
    brush.setAttribute('aria-label', t('Brush (B)'));
    brush.append(this.brushIcon, this.brushShown);
    brush.addEventListener('click', () => this.togglePopup());
    this.left.append(brush);
    const choice = (control: HTMLSelectElement, label: string): void => {
      const wrap = el('label', 'zb-swatch zb-choice');
      const shown = el('span', 'zb-choice-value');
      const s = el('select');
      s.setAttribute('aria-label', t(label));
      for (const o of Array.from(control.options)) {
        const c = el('option', '', o.textContent ?? o.value);
        c.value = o.value;
        s.append(c);
      }
      s.addEventListener('change', () => write(control, s.value));
      const sync = (): void => {
        if (s.value !== control.value) s.value = control.value;
        setText(shown, s.selectedOptions[0]?.textContent ?? '');
      };
      this.syncs.push(sync);
      wrap.append(el('span', 'zb-word', t(label)), shown, s);
      this.left.append(wrap);
    };
    choice(parts.stroke, 'Stroke');
    choice(parts.alpha, 'Alpha');
    choice(parts.shading, 'Material');
    const colour = el('label', 'zb-swatch zb-colour');
    const picker = el('input');
    picker.type = 'color';
    picker.setAttribute('aria-label', t('Colour'));
    picker.addEventListener('input', () => write(parts.paintColor, picker.value));
    this.syncs.push(() => {
      if (document.activeElement !== picker && picker.value !== parts.paintColor.value) picker.value = parts.paintColor.value;
    });
    colour.append(picker, el('span', 'zb-word', t('Colour')));
    this.left.append(colour);

    // The right shelf: view toggles, symmetry, Frame, the drag buttons, display toggles.
    const rcmd = (word: string, title: string, id: string, icon?: string): HTMLButtonElement => {
      const b = shelfButton(word, title, 'zb-side', icon);
      b.dataset.cmd = id;
      this.right.append(b);
      return b;
    };
    const persp = shelfButton('Persp', 'Perspective (P)', 'zb-side', 'ortho');
    persp.setAttribute('aria-pressed', 'true');
    persp.addEventListener('click', () => parts.run('view.ortho'));
    this.right.append(persp);
    this.modes.set('persp', persp);
    rcmd('Floor', 'Floor grid (Shift+P)', 'view.floor_grid', 'grid');
    rcmd('Sym', 'Symmetry X (X)', 'sculpt.mirror_x', 'symmetry');
    this.right.append(el('span', 'zb-gap'));
    rcmd('Frame', 'Frame (F)', 'view.frame', 'home');
    for (const [kind, word, title, icon] of [['move', 'Move', 'Drag to move the view', 'grab'], ['zoom', 'Zoom3D', 'Drag to zoom the view', 'search'], ['rotate', 'Rotate', 'Drag to rotate the view', 'orbit']] as const) {
      const b = shelfButton(word, title, 'zb-side zb-drag', icon);
      b.addEventListener('pointerdown', (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        b.setPointerCapture(e.pointerId);
        let last = { x: e.clientX, y: e.clientY };
        const move = (m: PointerEvent): void => {
          this.nav?.(kind, m.clientX - last.x, m.clientY - last.y);
          last = { x: m.clientX, y: m.clientY };
        };
        const up = (): void => {
          b.removeEventListener('pointermove', move);
          b.removeEventListener('pointerup', up);
          b.removeEventListener('pointercancel', up);
        };
        b.addEventListener('pointermove', move);
        b.addEventListener('pointerup', up);
        b.addEventListener('pointercancel', up);
      });
      this.right.append(b);
    }
    this.right.append(el('span', 'zb-gap'));
    rcmd('PolyF', 'PolyFrame (Shift+F)', 'view.wireframe', 'wireframe');
    rcmd('Transp', 'Transparency', 'view.xray', 'xray');
    rcmd('Solo', 'Solo', 'object.isolate', 'solo');

    // The tray's divider (opens and closes it) and its head.
    this.divider.type = 'button';
    this.divider.title = t('Open or close the tray (Tab)');
    this.divider.setAttribute('aria-label', t('Open or close the tray (Tab)'));
    this.divider.addEventListener('click', () => this.setTray(!this.trayOpen));
    this.trayHead.append(this.trayTitle);
    parts.panel.prepend(this.trayHead);

    // The brush picker: every tool as a tile.
    this.popup.setAttribute('role', 'dialog');
    this.popup.setAttribute('aria-label', t('Brushes'));
    for (const b of parts.brushButtons) {
      const kind = b.dataset.brush ?? '';
      const tile = el('button', 'zb-tile');
      tile.type = 'button';
      tile.dataset.cmd = `tool.brush.${kind}`;
      const label = t(this.toolLabels.get(kind) ?? kind);
      tile.title = label;
      tile.append(glyph(kind), el('span', 'zb-word', label));
      tile.addEventListener('click', () => this.togglePopup(false));
      this.popup.append(tile);
    }
    this.popup.hidden = true;
    this.quick.hidden = true;

    parts.root.append(this.title, this.menu, this.top, this.left, this.right, this.divider, this.popup, this.quick);

    window.addEventListener('pointermove', (e) => {
      this.pointer = { x: e.clientX, y: e.clientY };
    }, { passive: true });
    window.addEventListener('pointerdown', (e) => {
      const target = e.target as Node;
      if (!this.popup.hidden && !this.popup.contains(target) && !brush.contains(target)) this.togglePopup(false);
      if (!this.quick.hidden && !this.quick.contains(target)) this.closeQuick();
    });
    window.addEventListener('keydown', (e) => this.key(e), true);

    try {
      this.trayOpen = localStorage.getItem(TRAY_STORE) !== 'closed';
    } catch {
      this.trayOpen = true;
    }
  }

  private readonly syncs: (() => void)[] = [];
  /** Where the SubTool list lives outside this layout (over the viewport). */
  private dockHome: HTMLElement | null = null;

  get enabled(): boolean {
    return this.on;
  }

  /** Whether the person last left the window in this layout. */
  static stored(): boolean {
    try {
      return localStorage.getItem(STORE) === 'on';
    } catch {
      return false;
    }
  }

  set(on: boolean, keep = true): void {
    const changed = on !== this.on;
    this.on = on;
    this.parts.root.classList.toggle('zb', on);
    if (keep) {
      try {
        localStorage.setItem(STORE, on ? 'on' : 'off');
      } catch {
        // storage blocked; the choice lasts for this session
      }
    }
    window.clearInterval(this.timer);
    if (on) {
      this.setTray(this.trayOpen, false);
      this.showPalette(this.palette, false);
      this.layoutPick.value = 'zbrush';
      this.syncDefault();
      this.sync();
      this.timer = window.setInterval(() => this.sync(), 250);
    } else {
      this.togglePopup(false);
      this.closeQuick();
      for (const n of this.sectionOf.keys()) n.classList.remove('zb-off');
      // The SubTool list goes back over the viewport, where the tablet layout shows it.
      const sub = this.subtools;
      if (sub) {
        sub.classList.remove('zb-in-tray');
        if (this.dockHome && sub.parentElement !== this.dockHome) this.dockHome.append(sub);
      }
    }
    if (changed || !keep) this.onToggle?.(on);
  }

  toggle(): void {
    this.set(!this.on);
  }

  /** Shows a palette in the tray (opening the tray); the Tool palette brings the SubTool list with it. */
  showPalette(name: string, open = true): void {
    const palette = PALETTES.find(([p]) => p === name) ?? PALETTES[8];
    this.palette = palette[0];
    const wanted = new Set(palette[1]);
    for (const [n, section] of this.sectionOf) n.classList.toggle('zb-off', !wanted.has(section));
    setText(this.trayTitle, t(this.palette));
    for (const b of Array.from(this.menu.querySelectorAll<HTMLButtonElement>('[data-palette]'))) setAttr(b, 'aria-pressed', b.dataset.palette === this.palette ? 'true' : 'false');
    const sub = this.subtools;
    if (sub) {
      const inTool = this.palette === 'Tool';
      if (inTool && sub.parentElement !== this.parts.panel) {
        this.dockHome ??= sub.parentElement;
        this.trayHead.after(sub);
      }
      sub.classList.toggle('zb-in-tray', inTool);
    }
    if (open && !this.trayOpen) this.setTray(true);
    this.parts.panel.scrollTop = 0;
  }

  private setTray(open: boolean, keep = true): void {
    this.trayOpen = open;
    this.parts.root.classList.toggle('zb-tray-closed', !open);
    setAttr(this.divider, 'aria-expanded', open ? 'true' : 'false');
    if (!keep) return;
    try {
      localStorage.setItem(TRAY_STORE, open ? 'open' : 'closed');
    } catch {
      // storage blocked
    }
  }

  private togglePopup(show = this.popup.hidden): void {
    this.popup.hidden = !show;
  }

  /** Draw returns to the last sculpting brush; Move, Scale and Rotate pick the gizmo in that mode (ZBrush's W, E, R). */
  private mode(key: 'draw' | 'move' | 'scale' | 'rotate'): void {
    if (key === 'draw') {
      this.parts.run(`tool.brush.${this.lastSculpt}`);
      return;
    }
    if (this.currentTool() !== 'gizmo') this.parts.run('tool.brush.gizmo');
    this.parts.run(`gizmo.mode_${key}`);
  }

  /** Rgb paints colour; Mrgb paints colour with roughness and metalness (the Paint tool's own switch). */
  private paintMode(material: boolean): void {
    if (this.currentTool() !== 'paint') this.parts.run('tool.brush.paint');
    if (this.parts.paintMaterial.checked !== material) this.parts.paintMaterial.click();
  }

  private currentTool(): string {
    return this.parts.brushButtons.find((b) => b.getAttribute('aria-pressed') === 'true')?.dataset.brush ?? '';
  }

  /** ZBrush's S and U: a slider for Draw Size or Z Intensity at the cursor, gone on the next click or Escape. */
  private openQuick(source: HTMLInputElement, label: string): void {
    this.quickSlider = new DragValue(source, t(label), false, 'tb-field zb-field');
    this.quick.replaceChildren(this.quickSlider.node);
    this.quick.style.left = `${Math.max(8, Math.min(window.innerWidth - 230, this.pointer.x - 110))}px`;
    this.quick.style.top = `${Math.max(8, Math.min(window.innerHeight - 50, this.pointer.y - 20))}px`;
    this.quick.hidden = false;
    this.quickSlider.node.focus();
  }

  private closeQuick(): void {
    this.quick.hidden = true;
    this.quickSlider = null;
  }

  /** ZBrush's keys, while this layout is on and no field has the keyboard. */
  private key(e: KeyboardEvent): void {
    if (!this.on || e.isComposing) return;
    const target = e.target instanceof HTMLElement ? e.target : null;
    if (target?.closest('input:not([type=checkbox]):not([type=range]), textarea, select, [contenteditable="true"], .xterm')) return;
    const k = e.key.toLowerCase();
    const mod = e.ctrlKey || e.metaKey;
    let done = true;
    if (e.key === 'Escape' && (!this.popup.hidden || !this.quick.hidden)) {
      this.togglePopup(false);
      this.closeQuick();
    } else if (mod && !e.altKey && k === 'd' && !e.shiftKey) this.parts.run('multires.divide');
    else if (mod && !e.altKey && k === 'i' && !e.shiftKey) this.parts.run('mask.invert');
    else if (mod && e.shiftKey && !e.altKey && k === 'a') this.parts.run('mask.clear');
    else if (mod || e.altKey) done = false;
    else if (e.repeat) done = false;
    else if (k === 'q') this.mode('draw');
    else if (k === 'w') this.mode('move');
    else if (k === 'e') this.mode('scale');
    else if (k === 'r') this.mode('rotate');
    else if (k === 'x' && !e.shiftKey) this.parts.run('sculpt.mirror_x');
    else if (k === 'f' && e.shiftKey) this.parts.run('view.wireframe');
    else if (k === 'f') this.parts.run('view.frame');
    else if (k === 'p' && e.shiftKey) this.parts.run('view.floor_grid');
    else if (k === 'p') this.parts.run('view.ortho');
    else if (k === 'd' && e.shiftKey) this.parts.run('multires.lower');
    else if (k === 'd') this.parts.run('multires.higher');
    else if (k === 'b' && !e.shiftKey) this.togglePopup();
    else if (k === 'n' && !e.shiftKey) this.showPalette('Tool');
    else if (k === 's' && !e.shiftKey) this.openQuick(this.parts.radius, 'Draw Size');
    else if (k === 'u' && !e.shiftKey) this.openQuick(this.parts.strength, 'Z Intensity');
    else if (e.key === 'Tab' && !e.shiftKey) this.setTray(!this.trayOpen);
    else done = false;
    if (!done) return;
    e.preventDefault();
    e.stopImmediatePropagation();
  }

  private syncDefault(): void {
    const b = this.title.querySelector<HTMLButtonElement>('.zb-title-btn');
    if (!b) return;
    const isDefault = this.defaultLayout?.() === 'zbrush';
    setAttr(b, 'aria-pressed', isDefault ? 'true' : 'false');
    setText(b, isDefault ? t('Default layout') : t('Set as default'));
  }

  private sync(): void {
    for (const s of this.sliders) s.sync();
    this.quickSlider?.sync();
    for (const s of this.syncs) s();
    const tool = this.currentTool();
    if (tool !== this.toolNow) {
      this.toolNow = tool;
      if (tool && tool !== 'gizmo' && tool !== 'paint') this.lastSculpt = tool;
      setText(this.brushShown, t(this.toolLabels.get(tool) ?? tool));
      this.brushIcon.replaceChildren(glyph(tool));
    }
    const gizmo = tool === 'gizmo';
    const mode = this.parts.gizmoMode.value;
    const pressed: [string, boolean][] = [
      ['draw', !gizmo],
      ['move', gizmo && mode === 'move'],
      ['scale', gizmo && mode === 'scale'],
      ['rotate', gizmo && mode === 'rotate'],
      ['rgb', tool === 'paint' && !this.parts.paintMaterial.checked],
      ['mrgb', tool === 'paint' && this.parts.paintMaterial.checked],
      ['zadd', !this.parts.invert.checked],
      ['zsub', this.parts.invert.checked],
      ['persp', !this.parts.checked('view.ortho')],
    ];
    for (const [key, on] of pressed) {
      const b = this.modes.get(key);
      if (b) setAttr(b, 'aria-pressed', on ? 'true' : 'false');
    }
    const st = this.statsOf?.();
    if (st) setText(this.titleStats, `${t('Vertices')} ${fmt(st.vertices)} · ${t('Triangles')} ${fmt(st.triangles)}${st.active ? ` · ${st.active}` : ''}`);
    setText(this.titleNote, this.parts.message.textContent ?? '');
  }
}
