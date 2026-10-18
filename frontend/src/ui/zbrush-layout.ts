/**
 * The ZBrush layout: a third face of the window, arranged as ZBrush's classic desktop interface is (researched from
 * Pixologic's ZBrush Starting Guide and Maxon's shortcut sheet; see the frontend side log). From the top: a title bar
 * (version and counts on the left, the window's notes in the middle, the layout presets on the right), the palette
 * menu bar with its palettes in alphabetical order, and the top shelf (the Draw, Move, Scale and Rotate modes, Rgb and
 * Mrgb, Zadd and Zsub, Draw Size, Focal Shift, Z Intensity and Rgb Intensity). Below it the left shelf holds the brush, stroke, alpha, material and
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
import { alphaPreview, materialPreview, strokePreview } from './previews';
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
  /** The Paint brush's own strength, ZBrush's Rgb Intensity. */
  paintStrength: HTMLInputElement;
  /** Focal Shift over the falloff, -100 to 100. */
  focalShift: HTMLInputElement;
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
    // Words wrapped in t() here so the string extractor sees them (the helper's own t() then leaves them as they are).
    for (const [key, word, title, icon] of [['draw', t('Draw'), t('Draw (Q)'), 'draw'], ['move', t('Move'), t('Move (W)'), 'grab'], ['scale', t('Scale'), t('Scale (E)'), 'size'], ['rotate', t('Rotate'), t('Rotate (R)'), 'angle']] as const) {
      const b = shelfButton(word, title, 'zb-big', icon);
      b.setAttribute('aria-pressed', 'false');
      b.addEventListener('click', () => this.mode(key));
      this.modes.set(key, b);
      modes.append(b);
    }
    const paint = group();
    for (const [key, word, title] of [['mrgb', t('Mrgb'), t('Paint colour and material')], ['rgb', t('Rgb'), t('Paint colour')]] as const) {
      const b = shelfButton(word, title, 'zb-small');
      b.setAttribute('aria-pressed', 'false');
      b.addEventListener('click', () => this.paintMode(key === 'mrgb'));
      this.modes.set(key, b);
      paint.append(b);
    }
    const z = group();
    for (const [key, word, title] of [['zadd', t('Zadd'), t('Add to the surface')], ['zsub', t('Zsub'), t('Carve into the surface')]] as const) {
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
    // Rgb Intensity (I) sets the Paint brush's strength, Z Intensity (U) every other brush's, Focal Shift (O) the
    // falloff for both, as the ZBrush Starting Guide's top shelf and Maxon's shortcut sheet have them.
    const size = new DragValue(parts.radius, t('Draw Size'), false, 'tb-field zb-field');
    const focal = new DragValue(parts.focalShift, t('Focal Shift'), false, 'tb-field zb-field');
    const intensity = new DragValue(parts.strength, t('Z Intensity'), false, 'tb-field zb-field');
    const rgb = new DragValue(parts.paintStrength, t('Rgb Intensity'), false, 'tb-field zb-field');
    this.sliders.push(size, focal, intensity, rgb);
    sliders.append(size.node, focal.node, intensity.node, rgb.node);

    // The left shelf: the brush, stroke, alpha and material as preview icons that open pop-ups of thumbnails (as
    // ZBrush's Starting Guide describes its left shelf), and the colour.
    const brush = el('button', 'zb-swatch zb-brush');
    brush.type = 'button';
    brush.title = t('Brush (B)');
    brush.setAttribute('aria-label', t('Brush (B)'));
    brush.append(this.brushIcon, this.brushShown);
    brush.addEventListener('click', () => this.openPicker('brush', brush));
    this.left.append(brush);
    this.pickerAnchors.push(brush);
    this.left.append(this.swatch('stroke', t('Stroke'), 'F3'), this.swatch('alpha', t('Alpha'), 'F4'), this.swatch('material', t('Material'), 'F6'));
    const colour = el('label', 'zb-swatch zb-colour');
    const picker = el('input');
    picker.type = 'color';
    picker.setAttribute('aria-label', t('Colour'));
    picker.addEventListener('input', () => write(parts.paintColor, picker.value));
    this.syncs.push(() => {
      if (document.activeElement !== picker && picker.value !== parts.paintColor.value) picker.value = parts.paintColor.value;
    });
    colour.append(picker, el('span', 'zb-word', t('Colour')));
    // The palette names, through t() literally so the extractor finds them (the menu bar reads them by name).
    this.paletteWords = { Brush: t('Brush'), Edit: t('Edit'), File: t('File'), Layer: t('Layer'), Light: t('Light'), Material: t('Material'), Preferences: t('Preferences'), Texture: t('Texture'), Tool: t('Tool'), Transform: t('Transform'), Zplugin: t('Zplugin') };
    this.left.append(colour);

    // The right shelf: view toggles, symmetry, Frame, the drag buttons, display toggles.
    const rcmd = (word: string, title: string, id: string, icon?: string): HTMLButtonElement => {
      const b = shelfButton(word, title, 'zb-side', icon);
      b.dataset.cmd = id;
      this.right.append(b);
      return b;
    };
    const persp = shelfButton(t('Persp'), t('Perspective (P)'), 'zb-side', 'ortho');
    persp.setAttribute('aria-pressed', 'true');
    persp.addEventListener('click', () => parts.run('view.ortho'));
    this.right.append(persp);
    this.modes.set('persp', persp);
    rcmd(t('Floor'), t('Floor grid (Shift+P)'), 'view.floor_grid', 'grid');
    rcmd(t('Sym'), t('Symmetry X (X)'), 'sculpt.mirror_x', 'symmetry');
    this.right.append(el('span', 'zb-gap'));
    rcmd(t('Frame'), t('Frame (F)'), 'view.frame', 'home');
    for (const [kind, word, title, icon] of [['move', t('Move'), t('Drag to move the view'), 'grab'], ['zoom', t('Zoom3D'), t('Drag to zoom the view'), 'search'], ['rotate', t('Rotate'), t('Drag to rotate the view'), 'orbit']] as const) {
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
    rcmd(t('PolyF'), t('PolyFrame (Shift+F)'), 'view.wireframe', 'wireframe');
    rcmd(t('Transp'), t('Transparency'), 'view.xray', 'xray');
    rcmd(t('Solo'), t('Solo'), 'object.isolate', 'solo');

    // The tray's divider (opens and closes it) and its head.
    this.divider.type = 'button';
    this.divider.title = t('Open or close the tray (Tab)');
    this.divider.setAttribute('aria-label', t('Open or close the tray (Tab)'));
    this.divider.addEventListener('click', () => this.setTray(!this.trayOpen));
    this.trayHead.append(this.trayTitle);
    parts.panel.prepend(this.trayHead);
    // The SubTool list's place in the Tool palette, kept from the first paint: the list itself arrives once the engine
    // is connected, and without the place the palette's sections would jump down when it did.
    this.trayHead.after(this.subSlot);

    // The pop-up of thumbnails (brushes, strokes, alphas or materials), filled when it opens.
    this.popup.setAttribute('role', 'dialog');
    this.popup.hidden = true;
    this.quick.hidden = true;

    // The QuickMenu (Space, as ZBrush's): the brush, stroke, alpha and material, the two sliders and Zadd and Zsub at
    // the pointer.
    this.qmenu.setAttribute('role', 'dialog');
    this.qmenu.setAttribute('aria-label', t('QuickMenu'));
    const qBrush = el('button', 'zb-swatch zb-brush zb-mini');
    qBrush.type = 'button';
    qBrush.title = t('Brush (B)');
    qBrush.setAttribute('aria-label', t('Brush (B)'));
    qBrush.append(this.qBrushIcon);
    qBrush.addEventListener('click', () => this.openPicker('brush', qBrush));
    this.pickerAnchors.push(qBrush);
    const qRow = el('div', 'zb-qrow');
    qRow.append(qBrush, this.swatch('stroke', t('Stroke'), 'F3', true), this.swatch('alpha', t('Alpha'), 'F4', true), this.swatch('material', t('Material'), 'F6', true));
    const qSize = new DragValue(parts.radius, t('Draw Size'), false, 'tb-field zb-field');
    const qFocal = new DragValue(parts.focalShift, t('Focal Shift'), false, 'tb-field zb-field');
    const qIntensity = new DragValue(parts.strength, t('Z Intensity'), false, 'tb-field zb-field');
    const qRgb = new DragValue(parts.paintStrength, t('Rgb Intensity'), false, 'tb-field zb-field');
    this.sliders.push(qSize, qFocal, qIntensity, qRgb);
    const qZ = el('div', 'zb-qrow');
    for (const [key, word] of [['zadd', t('Zadd')], ['zsub', t('Zsub')]] as const) {
      const b = shelfButton(word, word, 'zb-small');
      b.addEventListener('click', () => {
        if (parts.invert.checked !== (key === 'zsub')) parts.invert.click();
      });
      this.qmodes.push([key, b]);
      qZ.append(b);
    }
    this.qmenu.append(qRow, qSize.node, qFocal.node, qIntensity.node, qRgb.node, qZ);
    this.qmenu.hidden = true;

    parts.root.append(this.title, this.menu, this.top, this.left, this.right, this.divider, this.popup, this.quick, this.qmenu);

    window.addEventListener('pointermove', (e) => {
      this.pointer = { x: e.clientX, y: e.clientY };
    }, { passive: true });
    window.addEventListener('pointerdown', (e) => {
      const target = e.target as Node;
      const onAnchor = this.pickerAnchors.some((a) => a.contains(target));
      if (!this.popup.hidden && !this.popup.contains(target) && !onAnchor) this.togglePopup(false);
      if (!this.quick.hidden && !this.quick.contains(target)) this.closeQuick();
      if (!this.qmenu.hidden && !this.qmenu.contains(target) && !this.popup.contains(target)) this.qmenu.hidden = true;
    });
    window.addEventListener('keydown', (e) => this.key(e), true);

    try {
      this.trayOpen = localStorage.getItem(TRAY_STORE) !== 'closed';
    } catch {
      this.trayOpen = true;
    }
  }

  private readonly syncs: (() => void)[] = [];
  /** The pop-up's kind while open, and the letters typed so far to narrow the brushes (ZBrush's B, then letters). */
  private picker: 'brush' | 'stroke' | 'alpha' | 'material' | null = null;
  private letters = '';
  /** Buttons that open the pop-up (a click on one does not count as a click outside it). */
  private readonly pickerAnchors: HTMLElement[] = [];
  private readonly qmenu = el('div', 'zb-qmenu');
  private readonly qBrushIcon = el('span', 'zb-brush-icon');
  private readonly qmodes: [string, HTMLButtonElement][] = [];
  private readonly subSlot = el('div', 'zb-subtool-slot');
  /** Where the SubTool list lives outside this layout (over the viewport). */
  private dockHome: HTMLElement | null = null;
  /** The palettes' names in the window's language, by their English name. */
  private paletteWords: Record<string, string> = {};

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
    setText(this.trayTitle, this.paletteWords[this.palette] ?? t(this.palette));
    for (const b of Array.from(this.menu.querySelectorAll<HTMLButtonElement>('[data-palette]'))) setAttr(b, 'aria-pressed', b.dataset.palette === this.palette ? 'true' : 'false');
    this.subSlot.hidden = this.palette !== 'Tool';
    const sub = this.subtools;
    if (sub) {
      const inTool = this.palette === 'Tool';
      if (inTool && sub.parentElement !== this.subSlot) {
        this.dockHome ??= sub.parentElement;
        this.subSlot.append(sub);
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
    if (show && !this.picker) {
      this.openPicker('brush');
      return;
    }
    this.popup.hidden = !show;
    if (!show) {
      this.picker = null;
      this.letters = '';
    }
  }

  /** The control a pop-up kind writes (the brush is a command instead). */
  private controlOf(kind: 'stroke' | 'alpha' | 'material'): HTMLSelectElement {
    return kind === 'stroke' ? this.parts.stroke : kind === 'alpha' ? this.parts.alpha : this.parts.shading;
  }

  private previewOf(kind: 'stroke' | 'alpha' | 'material', value: string, size: number): HTMLImageElement {
    return kind === 'stroke' ? strokePreview(value, size) : kind === 'alpha' ? alphaPreview(value, size) : materialPreview(value, size);
  }

  /** A shelf swatch: the current stroke, alpha or material as a thumbnail over its name; a click opens its pop-up. */
  private swatch(kind: 'stroke' | 'alpha' | 'material', label: string, key: string, mini = false): HTMLButtonElement {
    const b = el('button', `zb-swatch zb-pick${mini ? ' zb-mini' : ''}`);
    b.type = 'button';
    b.title = `${label} (${key})`;
    b.setAttribute('aria-label', label);
    const pic = el('span', 'zb-thumb');
    b.append(pic);
    if (!mini) b.append(el('span', 'zb-word', label));
    b.addEventListener('click', () => this.openPicker(kind, b));
    this.pickerAnchors.push(b);
    let shown = '';
    this.syncs.push(() => {
      const v = this.controlOf(kind).value;
      if (v === shown) return;
      shown = v;
      pic.replaceChildren(this.previewOf(kind, v, mini ? 30 : 40));
      b.title = `${label}: ${this.controlOf(kind).selectedOptions[0]?.textContent ?? v} (${key})`;
    });
    return b;
  }

  /** Opens the pop-up of thumbnails for a kind, beside the button that asked (or at the pointer, from a key). */
  private openPicker(kind: 'brush' | 'stroke' | 'alpha' | 'material', anchor?: HTMLElement): void {
    if (this.picker === kind && !this.popup.hidden) {
      this.togglePopup(false);
      return;
    }
    this.picker = kind;
    this.letters = '';
    const tiles: HTMLElement[] = [];
    if (kind === 'brush') {
      this.popup.setAttribute('aria-label', t('Brushes'));
      for (const b of this.parts.brushButtons) {
        const tool = b.dataset.brush ?? '';
        const label = t(this.toolLabels.get(tool) ?? tool);
        const tile = el('button', 'zb-tile');
        tile.type = 'button';
        tile.dataset.cmd = `tool.brush.${tool}`;
        tile.dataset.name = label;
        tile.title = label;
        tile.append(glyph(tool), el('span', 'zb-word', label), el('span', 'zb-key'));
        tile.addEventListener('click', () => this.togglePopup(false));
        tiles.push(tile);
      }
    } else {
      const control = this.controlOf(kind);
      this.popup.setAttribute('aria-label', kind === 'stroke' ? t('Stroke') : kind === 'alpha' ? t('Alpha') : t('Material'));
      for (const o of Array.from(control.options)) {
        const tile = el('button', 'zb-tile zb-thumb-tile');
        tile.type = 'button';
        const label = o.textContent ?? o.value;
        tile.title = kind === 'material' && o.value === 'pbr' ? `${label}: ${t('lit by the scene lights; this sphere is an approximation')}` : label;
        setAttr(tile, 'aria-pressed', control.value === o.value ? 'true' : 'false');
        tile.append(this.previewOf(kind, o.value, 44), el('span', 'zb-word', label));
        tile.addEventListener('click', () => {
          write(control, o.value);
          this.togglePopup(false);
        });
        tiles.push(tile);
      }
    }
    this.popup.replaceChildren(...tiles);
    const at = anchor?.getBoundingClientRect();
    const x = at ? at.right + 6 : this.pointer.x + 8;
    const y = at ? at.top : this.pointer.y - 20;
    this.popup.style.left = `${Math.max(8, Math.min(window.innerWidth - 460, x))}px`;
    this.popup.style.top = `${Math.max(8, Math.min(window.innerHeight - 200, y))}px`;
    this.popup.hidden = false;
  }

  /** ZBrush's brush keys: after B, a letter narrows the brushes to those starting with it and gives each a second
   * letter; that letter picks it (B, S, T for Standard). Returns whether the key was taken. */
  private brushLetter(letter: string): boolean {
    const tiles = Array.from(this.popup.querySelectorAll<HTMLButtonElement>('.zb-tile'));
    if (this.letters === '') {
      const hits = tiles.filter((x) => (x.dataset.name ?? '').toLowerCase().startsWith(letter));
      if (hits.length === 0) return true;
      this.letters = letter;
      const used = new Set<string>();
      for (const x of tiles) {
        const keep = hits.includes(x);
        x.hidden = !keep;
        const badge = x.querySelector('.zb-key');
        if (!keep || !badge) continue;
        // The first letter of its name (after the first) not taken by an earlier brush.
        const name = (x.dataset.name ?? '').toLowerCase().replace(/[^a-z]/g, '');
        const pick = [...name.slice(1)].find((c) => !used.has(c)) ?? '';
        used.add(pick);
        x.dataset.key = pick;
        badge.textContent = pick.toUpperCase();
      }
      return true;
    }
    const chosen = tiles.find((x) => !x.hidden && x.dataset.key === letter);
    if (chosen) chosen.click();
    return true;
  }

  /** The QuickMenu at the pointer (Space), or closed. */
  private toggleQuickMenu(): void {
    if (!this.qmenu.hidden) {
      this.qmenu.hidden = true;
      return;
    }
    this.qmenu.style.left = `${Math.max(8, Math.min(window.innerWidth - 260, this.pointer.x - 120))}px`;
    this.qmenu.style.top = `${Math.max(8, Math.min(window.innerHeight - 200, this.pointer.y - 60))}px`;
    this.qmenu.hidden = false;
  }

  /** ZBrush's [ and ]: a step of the draw size (a twentieth of its range). */
  private stepSize(dir: 1 | -1): void {
    const r = this.parts.radius;
    const lo = Number(r.min || 0);
    const hi = Number(r.max || 100);
    write(r, String(Math.max(lo, Math.min(hi, Number(r.value) + (dir * (hi - lo)) / 20))));
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

  /** ZBrush's S, O, U and I: a slider for Draw Size, Focal Shift, Z Intensity or Rgb Intensity at the cursor, gone on
   * the next click or Escape. */
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
    const plain = !mod && !e.altKey && !e.shiftKey;
    if (e.key === 'Escape' && (!this.popup.hidden || !this.quick.hidden || !this.qmenu.hidden)) {
      this.togglePopup(false);
      this.closeQuick();
      this.qmenu.hidden = true;
    } else if (plain && !this.popup.hidden && this.picker === 'brush' && /^[a-z]$/.test(k)) this.brushLetter(k);
    else if (plain && e.key === 'F2') this.openPicker('brush');
    else if (plain && e.key === 'F3') this.openPicker('stroke');
    else if (plain && e.key === 'F4') this.openPicker('alpha');
    else if (plain && e.key === 'F6') this.openPicker('material');
    else if (plain && e.key === ' ' && !e.repeat) this.toggleQuickMenu();
    else if (plain && e.key === '[') this.stepSize(-1);
    else if (plain && e.key === ']') this.stepSize(1);
    else if (mod && !e.altKey && k === 'd' && !e.shiftKey) this.parts.run('multires.divide');
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
    else if (k === 'o' && !e.shiftKey) this.openQuick(this.parts.focalShift, 'Focal Shift');
    else if (k === 'i' && !e.shiftKey) this.openQuick(this.parts.paintStrength, 'Rgb Intensity');
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
      this.qBrushIcon.replaceChildren(glyph(tool));
    }
    for (const [key, b] of this.qmodes) setAttr(b, 'aria-pressed', (key === 'zsub') === this.parts.invert.checked ? 'true' : 'false');
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
