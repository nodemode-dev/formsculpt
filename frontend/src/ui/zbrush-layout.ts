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
 * sections. The look is formsculpt's, in ZBrush's grey; no ZBrush artwork is used.
 */

import { el, setAttr, setText } from './dom';
import { t } from './i18n';
import { alphaPreview, materialPreview, strokePreview } from './previews';
import { DragValue, glyph, write } from './tablet-layout';

const STORE = 'formsculpt.zbrush';
const TRAY_STORE = 'formsculpt.zbrush.tray';

/** ZBrush's palettes that formsculpt has counterparts for, in ZBrush's alphabetical order: [name, the panel's
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

/** Line drawings of the primitives for LightBox's Tool shelf (currentColor, 24 unit box). */
const PRIMITIVE_ICONS: Record<string, string> = {
  sphere: '<circle cx="12" cy="12" r="8"/><path d="M4 12c2 2.5 14 2.5 16 0"/>',
  cube: '<path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z"/><path d="M12 12l8-4.5M12 12v9M12 12L4 7.5"/>',
  cylinder: '<ellipse cx="12" cy="6" rx="7" ry="2.5"/><path d="M5 6v12c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5V6"/>',
  cone: '<path d="M12 3L5 18M12 3l7 15"/><ellipse cx="12" cy="18" rx="7" ry="2.5"/>',
  torus: '<ellipse cx="12" cy="12" rx="9" ry="5"/><ellipse cx="12" cy="11.5" rx="3.5" ry="1.6"/>',
  plane: '<path d="M3 15l7-6h11l-7 6z"/>',
  quad_sphere: '<circle cx="12" cy="12" r="8"/><path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>',
  uv_sphere: '<circle cx="12" cy="12" r="8"/><path d="M4 12h16M12 4c-3 3-3 13 0 16M12 4c3 3 3 13 0 16"/>',
  icosahedron: '<path d="M12 3l8 5v8l-8 5-8-5V8z"/><path d="M12 3L7 13h10zM4 8l3 5M20 8l-3 5M7 13l5 8 5-8"/>',
  head: '<path d="M12 3c-4 0-7 3-7 7.5 0 3 1.5 5 3 6.5v4h7v-2.5h2.5v-3.5l1.5-1-1.5-3C17.5 6 15.5 3 12 3z"/>',
  triplanar: '<path d="M4 12h16M12 4v16"/><path d="M6 7l12 10"/>',
};

function primitiveIcon(kind: string): SVGSVGElement {
  if (!PRIMITIVE_ICONS[kind]) return glyph('scene');
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('aria-hidden', 'true');
  svg.setAttribute('fill', 'none');
  svg.setAttribute('stroke', 'currentColor');
  svg.setAttribute('stroke-width', '1.6');
  svg.setAttribute('stroke-linecap', 'round');
  svg.setAttribute('stroke-linejoin', 'round');
  svg.innerHTML = PRIMITIVE_ICONS[kind];
  return svg;
}

/** LightBox's shelves: the Starting Guide names Project and Alpha; the rest are the kinds ZBrush's pop-ups hold here. */
type LightBoxShelf = 'project' | 'tool' | 'brush' | 'stroke' | 'alpha' | 'texture' | 'material';

/** The Project shelf's service calls (the app wires them to the engine service). */
export interface LightBoxProjects {
  list: () => Promise<{ name: string; saved: number; preview: boolean }[] | string>;
  /** Saves the scene under a name: null when saved, else why not. */
  save: (name: string) => Promise<string | null>;
  /** Opens a saved project, replacing the scene: null when opened, else why not. */
  open: (name: string) => Promise<string | null>;
  preview: (name: string, saved: number) => string;
}

/** The textures: the images Spotlight holds, which one is active, and loading another from a file. */
export interface ZBrushTextures {
  list: () => { image: number; name: string; active: boolean; src: string }[];
  activate: (image: number) => void;
  load: () => void;
}

export interface ZBrushParts {
  root: HTMLElement;
  panel: HTMLElement;
  brushButtons: HTMLButtonElement[];
  /** The Add buttons (one per primitive, data-kind), LightBox's Tool shelf. */
  addButtons: HTMLButtonElement[];
  radius: HTMLInputElement;
  /** Z Intensity: the sculpting brushes' strength, kept apart from the Paint brush's even while it is in hand. */
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
  // LightBox: a browser of shelves (Project, Tool, Brush, Stroke, Alpha, Material) along the bottom of the canvas.
  private readonly lightbox = el('section', 'zb-lightbox');
  private readonly lbTabs = el('div', 'zb-lb-tabs');
  private readonly lbStrip = el('div', 'zb-lb-strip');
  private readonly lbHead = el('div', 'zb-lb-head');
  private readonly lbButton = el('button', 'zb-small zb-lb-button');
  private lbShelf: LightBoxShelf = 'project';
  private readonly brushShown = el('span', 'zb-word');
  private readonly brushIcon = el('span', 'zb-brush-icon');
  private readonly layoutPick = el('select', 'zb-layout-pick');
  private readonly sliders: DragValue[] = [];
  private quickSlider: DragValue | null = null;
  private readonly modes = new Map<string, HTMLButtonElement>();
  private toolNow = '';

  /** Set by the app: the scene's counts for the title bar. */
  statsOf: (() => ZBrushStats) | null = null;
  /** Set by the app: LightBox's Project shelf (saved projects with pictures), through the engine service. */
  projects: LightBoxProjects | null = null;
  /** Set by the app: the textures (Spotlight's images) for the Texture swatch, F5 and LightBox's Texture shelf. */
  textures: ZBrushTextures | null = null;
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
    const name = el('span', 'zb-title-name', `formsculpt ${parts.version}`);
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
    const agents = el('span', 'zb-title-tools');
    for (const [g, id, label] of [['agent', 'agent.claude', 'Claude Code'], ['agentChat', 'agent.chat', 'Chat with Claude'], ['terminal', 'view.terminal', 'Terminal']]) {
      const b = el('button', 'zb-title-tool');
      b.type = 'button';
      b.dataset.cmd = id;
      b.setAttribute('aria-label', t(label));
      b.append(glyph(g));
      agents.append(b);
    }
    const presets = el('span', 'zb-title-right');
    presets.append(agents, this.layoutPick, makeDefault);
    this.title.append(name, this.titleStats, this.titleNote, presets);
    if (document.documentElement.classList.contains('has-overlay-titlebar')) {
      for (const n of [this.title, name, this.titleStats, this.titleNote, presets, agents]) n.setAttribute('data-tauri-drag-region', '');
    }

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
    // The utilities the Starting Guide lists on the top shelf; LightBox is the one built (comma, as Maxon's sheet has it).
    const utilities = group();
    this.lbButton.type = 'button';
    this.lbButton.textContent = t('LightBox');
    this.lbButton.title = t('LightBox (,)');
    this.lbButton.setAttribute('aria-pressed', 'false');
    this.lbButton.addEventListener('click', () => this.toggleLightBox());
    utilities.append(this.lbButton);
    this.buildLightBox();

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
    // Texture (F5) sits between Alpha and Material, as the Starting Guide lists the shelf; formsculpt's textures are
    // the images held in Spotlight, where Maxon's shortcut sheet says LightBox loads textures.
    this.left.append(this.swatch('stroke', t('Stroke'), 'F3'), this.swatch('alpha', t('Alpha'), 'F4'), this.textureSwatch(), this.swatch('material', t('Material'), 'F6'));
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

    parts.root.append(this.title, this.menu, this.top, this.left, this.right, this.divider, this.popup, this.quick, this.qmenu, this.lightbox);

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
  private picker: 'brush' | 'stroke' | 'alpha' | 'texture' | 'material' | null = null;
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

  /** LightBox's frame: its shelves as tabs, a head (the Project shelf's save field), and the strip of thumbnails. */
  private buildLightBox(): void {
    this.lightbox.setAttribute('aria-label', t('LightBox'));
    this.lightbox.hidden = true;
    this.lbTabs.setAttribute('role', 'tablist');
    // 'Projects' rather than ZBrush's 'Project': the window already has Project as a verb (project detail), which
    // translates differently.
    const shelves: [LightBoxShelf, string][] = [['project', t('Projects')], ['tool', t('Tool')], ['brush', t('Brush')], ['stroke', t('Stroke')], ['alpha', t('Alpha')], ['texture', t('Texture')], ['material', t('Material')]];
    for (const [shelf, label] of shelves) {
      const tab = el('button', 'zb-lb-tab', label);
      tab.type = 'button';
      tab.dataset.shelf = shelf;
      tab.setAttribute('role', 'tab');
      tab.setAttribute('aria-selected', 'false');
      tab.addEventListener('click', () => void this.showShelf(shelf));
      this.lbTabs.append(tab);
    }
    this.lbStrip.setAttribute('role', 'listbox');
    this.lbStrip.setAttribute('aria-label', t('LightBox items: double-click to load'));
    this.lightbox.append(this.lbTabs, this.lbHead, this.lbStrip);
  }

  /** Shows or hides LightBox (comma, or its shelf button). */
  private toggleLightBox(show = this.lightbox.hidden): void {
    this.lightbox.hidden = !show;
    setAttr(this.lbButton, 'aria-pressed', show ? 'true' : 'false');
    if (show) void this.showShelf(this.lbShelf);
  }

  /** One LightBox thumbnail: a click selects it, a double click or Enter loads it, as ZBrush's LightBox does. */
  private lbTile(label: string, picture: Node, load: () => void, current = false): HTMLButtonElement {
    const tile = el('button', 'zb-lb-tile');
    tile.type = 'button';
    tile.title = label;
    tile.setAttribute('role', 'option');
    tile.setAttribute('aria-selected', 'false');
    if (current) tile.classList.add('current');
    tile.append(picture, el('span', 'zb-word', label));
    tile.addEventListener('click', () => {
      for (const x of this.lbStrip.querySelectorAll('.zb-lb-tile')) setAttr(x as HTMLElement, 'aria-selected', x === tile ? 'true' : 'false');
    });
    tile.addEventListener('dblclick', load);
    tile.addEventListener('keydown', (e) => {
      if (e.key !== 'Enter') return;
      e.preventDefault();
      load();
    });
    return tile;
  }

  /** Fills the strip with one shelf's items. */
  private async showShelf(shelf: LightBoxShelf): Promise<void> {
    this.lbShelf = shelf;
    for (const tab of this.lbTabs.querySelectorAll<HTMLElement>('.zb-lb-tab')) setAttr(tab, 'aria-selected', tab.dataset.shelf === shelf ? 'true' : 'false');
    this.lbHead.replaceChildren();
    const tiles: HTMLElement[] = [];
    if (shelf === 'project') {
      await this.fillProjects();
      return;
    }
    if (shelf === 'tool') {
      for (const b of this.parts.addButtons) {
        const label = b.textContent?.trim() || (b.dataset.kind ?? '');
        tiles.push(this.lbTile(label, primitiveIcon(b.dataset.kind ?? ''), () => b.click()));
      }
    } else if (shelf === 'brush') {
      for (const b of this.parts.brushButtons) {
        const tool = b.dataset.brush ?? '';
        tiles.push(this.lbTile(t(this.toolLabels.get(tool) ?? tool), glyph(tool), () => b.click(), b.getAttribute('aria-pressed') === 'true'));
      }
    } else if (shelf === 'texture') {
      const tex = this.textures;
      if (tex) {
        tiles.push(this.lbTile(t('Load image'), glyph('files'), () => tex.load()));
        for (const x of tex.list()) {
          tiles.push(this.lbTile(x.name, this.texturePicture(x.src, 56), () => {
            tex.activate(x.image);
            void this.showShelf('texture');
          }, x.active));
        }
      }
    } else {
      const control = this.controlOf(shelf);
      for (const o of Array.from(control.options)) {
        tiles.push(this.lbTile(o.textContent ?? o.value, this.previewOf(shelf, o.value, 56), () => {
          write(control, o.value);
          void this.showShelf(shelf);
        }, control.value === o.value));
      }
    }
    this.lbStrip.replaceChildren(...tiles);
  }

  /** The Project shelf: a name field and Save, then every saved project with its picture, newest first. */
  private async fillProjects(): Promise<void> {
    const name = el('input', 'zb-lb-name') as HTMLInputElement;
    name.type = 'text';
    name.maxLength = 64;
    name.placeholder = t('Project name');
    name.setAttribute('aria-label', t('Project name'));
    const save = el('button', 'zb-small', t('Save to LightBox'));
    save.type = 'button';
    const status = el('span', 'zb-lb-status');
    status.setAttribute('role', 'status');
    this.lbHead.append(name, save, status);
    const api = this.projects;
    if (!api) {
      setText(status, t('Needs the engine service'));
      this.lbStrip.replaceChildren();
      return;
    }
    save.addEventListener('click', () => {
      const wanted = name.value.trim();
      if (!wanted) {
        setText(status, t('Type a name first'));
        name.focus();
        return;
      }
      void api.save(wanted).then((problem) => {
        if (problem) setText(status, problem);
        else void this.showShelf('project');
      });
    });
    name.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') save.click();
    });
    const listed = await api.list();
    if (this.lbShelf !== 'project') return;
    if (typeof listed === 'string') {
      setText(status, listed);
      this.lbStrip.replaceChildren();
      return;
    }
    if (listed.length === 0) setText(status, t('No saved projects yet'));
    this.lbStrip.replaceChildren(
      ...listed.map((p) => {
        let picture: Node = glyph('files');
        if (p.preview) {
          const img = el('img', 'zb-lb-pic') as HTMLImageElement;
          img.src = api.preview(p.name, p.saved);
          img.alt = '';
          img.width = 56;
          img.height = 56;
          picture = img;
        }
        return this.lbTile(p.name, picture, () => {
          void api.open(p.name).then((problem) => {
            setText(status, problem ?? t('Opened {name}').replace('{name}', p.name));
          });
        });
      }),
    );
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

  /** A texture's picture at a size: the Spotlight image, contained in a square. */
  private texturePicture(src: string, size: number): HTMLImageElement {
    const img = el('img', 'zb-tex') as HTMLImageElement;
    img.src = src;
    img.alt = '';
    img.width = size;
    img.height = size;
    return img;
  }

  /** The left shelf's Texture swatch (F5): the active Spotlight image, or an empty frame before any is loaded. */
  private textureSwatch(): HTMLButtonElement {
    const b = el('button', 'zb-swatch zb-pick');
    b.type = 'button';
    const pic = el('span', 'zb-thumb');
    b.append(pic, el('span', 'zb-word', t('Texture')));
    b.title = t('Texture (F5)');
    b.setAttribute('aria-label', t('Texture (F5)'));
    b.addEventListener('click', () => this.openPicker('texture', b));
    this.pickerAnchors.push(b);
    let shown: string | null = null;
    let held = '';
    this.syncs.push(() => {
      const all = this.textures?.list() ?? [];
      // An open Texture shelf follows images loaded or removed elsewhere (the panel, an agent).
      const now = all.map((x) => `${x.image}${x.active ? '*' : ''}`).join(',');
      if (now !== held) {
        held = now;
        if (!this.lightbox.hidden && this.lbShelf === 'texture') void this.showShelf('texture');
      }
      const active = all.find((x) => x.active);
      const src = active?.src ?? '';
      if (src === shown) return;
      shown = src;
      pic.replaceChildren(src ? this.texturePicture(src, 40) : glyph('files'));
      b.title = active ? `${t('Texture')}: ${active.name} (F5)` : t('Texture (F5)');
    });
    return b;
  }

  /** Opens the pop-up of thumbnails for a kind, beside the button that asked (or at the pointer, from a key). */
  private openPicker(kind: 'brush' | 'stroke' | 'alpha' | 'texture' | 'material', anchor?: HTMLElement): void {
    if (this.picker === kind && !this.popup.hidden) {
      this.togglePopup(false);
      return;
    }
    this.picker = kind;
    this.letters = '';
    const tiles: HTMLElement[] = [];
    if (kind === 'texture') {
      // The textures are Spotlight's images; a click makes one active, Load image adds another.
      this.popup.setAttribute('aria-label', t('Texture'));
      const tex = this.textures;
      const load = el('button', 'zb-tile zb-thumb-tile');
      load.type = 'button';
      load.title = t('Load image');
      load.append(glyph('files'), el('span', 'zb-word', t('Load image')));
      load.addEventListener('click', () => {
        this.togglePopup(false);
        tex?.load();
      });
      tiles.push(load);
      for (const x of tex?.list() ?? []) {
        const tile = el('button', 'zb-tile zb-thumb-tile');
        tile.type = 'button';
        tile.title = x.name;
        setAttr(tile, 'aria-pressed', x.active ? 'true' : 'false');
        tile.append(this.texturePicture(x.src, 44), el('span', 'zb-word', x.name));
        tile.addEventListener('click', () => {
          tex?.activate(x.image);
          this.togglePopup(false);
        });
        tiles.push(tile);
      }
    } else if (kind === 'brush') {
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
    if (e.key === 'Escape' && (!this.popup.hidden || !this.quick.hidden || !this.qmenu.hidden || !this.lightbox.hidden)) {
      this.togglePopup(false);
      this.closeQuick();
      this.qmenu.hidden = true;
      this.toggleLightBox(false);
    } else if (plain && e.key === ',' && !e.repeat) this.toggleLightBox();
    else if (plain && !this.popup.hidden && this.picker === 'brush' && /^[a-z]$/.test(k)) this.brushLetter(k);
    else if (plain && e.key === 'F2') this.openPicker('brush');
    else if (plain && e.key === 'F3') this.openPicker('stroke');
    else if (plain && e.key === 'F4') this.openPicker('alpha');
    else if (plain && e.key === 'F5') this.openPicker('texture');
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
