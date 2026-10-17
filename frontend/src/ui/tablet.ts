/**
 * The tablet layout, after the way tablet sculpting apps arrange their screen: the viewport fills the window, a mini
 * rail of tool icons runs down the left edge with tall Radius and Intensity sliders beside it, the active tool's name
 * sits in a pill above a strip of that tool's options, a quick bar holds the most used toggles, and a cluster of icons
 * at the top opens the panel's sections as a floating sheet. Nothing here is a new feature: every rail and quick-bar
 * button is a registry command (data-cmd, so its pressed and disabled state follows the command), and every slider
 * and option is a stand-in that writes to the panel control it mirrors and reads it back.
 */

import { el, setAttr, setText } from './dom';
import { t } from './i18n';

const STORE = 'formsculptor.tablet';
const SVG = 'http://www.w3.org/2000/svg';

export type TabletControl = HTMLInputElement | HTMLSelectElement | HTMLButtonElement;

export interface TabletOption {
  label: string;
  control: TabletControl;
}

export interface TabletParts {
  root: HTMLElement;
  stage: HTMLElement;
  panel: HTMLElement;
  brushButtons: HTMLButtonElement[];
  radius: HTMLInputElement;
  strength: HTMLInputElement;
  /** The options shown in the strip for a tool (by its brush kind). */
  options: (tool: string) => TabletOption[];
}

/** Tools on the rail, in order, by brush kind. */
const RAIL_TOOLS = ['draw', 'clay', 'inflate', 'smooth', 'flatten', 'crease', 'pinch', 'grab', 'snake_hook', 'layer', 'trim', 'mask', 'paint', 'tube', 'curve', 'gizmo', 'zmodeler', 'orbit'];

/** Line glyphs (24 by 24) for the rail's tools and the sheets; a tool without one shows its initials. */
const GLYPHS: Record<string, string> = {
  draw: '<path d="M4 16c3-6 6-8 8-8s3 3 5 3 3-1 3-1"/><path d="M4 20h16"/>',
  clay: '<rect x="4" y="12" width="16" height="6" rx="2"/><path d="M7 12V9h10v3"/>',
  inflate: '<circle cx="12" cy="12" r="7"/><path d="M12 8v8M8 12h8"/>',
  smooth: '<path d="M3 14c3-3 6-3 9 0s6 3 9 0"/><path d="M3 9c3-2 6-2 9 0s6 2 9 0" opacity="0.5"/>',
  flatten: '<path d="M4 15h16"/><path d="M6 11c2-3 4-3 6 0s4 3 6 0" opacity="0.5"/><path d="M12 4v5M10 7l2 2 2-2"/>',
  crease: '<path d="M3 8l9 9 9-9"/><path d="M8 8l4 4 4-4" opacity="0.5"/>',
  pinch: '<path d="M4 6l6 6-6 6M20 6l-6 6 6 6"/>',
  grab: '<path d="M8 13V6.5a1.5 1.5 0 0 1 3 0V12M11 11V5a1.5 1.5 0 0 1 3 0v6M14 11V6.5a1.5 1.5 0 0 1 3 0V14c0 4-2.5 6-5.5 6S6 18 5 15.5l-1-3a1.5 1.5 0 0 1 2.7-1.2L8 13"/>',
  snake_hook: '<path d="M5 19c0-7 4-12 10-12 3 0 4 2 4 4s-2 3-3 3"/><circle cx="16" cy="14" r="1"/>',
  layer: '<path d="M4 15h16M7 15v-4h10v4"/><path d="M4 19h16" opacity="0.5"/>',
  trim: '<path d="M4 18h16"/><path d="M6 14l6-8 6 8"/><path d="M3 10h18" stroke-dasharray="2 2"/>',
  mask: '<circle cx="12" cy="12" r="8"/><path d="M12 4a8 8 0 0 1 0 16z" fill="currentColor"/>',
  paint: '<path d="M14.5 4.5l5 5L10 19l-5 .5.5-5z"/><path d="M12.5 6.5l5 5"/>',
  tube: '<path d="M4 18c2-8 6-12 16-12"/><path d="M4 14c2-6 5-9 12-10" opacity="0.5"/>',
  curve: '<path d="M4 18C8 4 16 20 20 6"/><circle cx="4" cy="18" r="1.5"/><circle cx="20" cy="6" r="1.5"/>',
  gizmo: '<path d="M12 12V4M12 12l7 4M12 12l-7 4"/><path d="M10 6l2-2 2 2M17 14l2 2-3 0M7 14l-2 2 3 0"/>',
  zmodeler: '<path d="M5 5h14v14H5z"/><path d="M5 12h14M12 5v14" opacity="0.5"/><circle cx="12" cy="12" r="1.5" fill="currentColor"/>',
  orbit: '<circle cx="12" cy="12" r="3"/><path d="M4 12a8 4 0 0 0 16 0 8 4 0 0 0-16 0"/>',
  files: '<path d="M3.5 7.5a2 2 0 0 1 2-2H10l2 2.5h6.5a2 2 0 0 1 2 2v7.5a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>',
  scene: '<circle cx="6" cy="6" r="2"/><circle cx="18" cy="8" r="2"/><circle cx="10" cy="18" r="2"/><path d="M8 6.5l8 1M7 8l2.5 8M16.5 9.5l-5 7"/>',
  tools: '<path d="M4 7h10M18 7h2M4 17h4M12 17h8"/><circle cx="16" cy="7" r="2"/><circle cx="10" cy="17" r="2"/>',
  topology: '<path d="M4 4h16v16H4z"/><path d="M4 10h16M4 15h16M10 4v16M15 4v16" opacity="0.6"/>',
  material: '<circle cx="12" cy="12" r="8"/><path d="M7 9a6 6 0 0 1 5-3" opacity="0.6"/>',
  light: '<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2.5M12 19v2.5M2.5 12H5M19 12h2.5M5.3 5.3l1.8 1.8M16.9 16.9l1.8 1.8M5.3 18.7l1.8-1.8M16.9 7.1l1.8-1.8"/>',
  view: '<rect x="3" y="6" width="13" height="12" rx="2"/><path d="M16 10l5-3v10l-5-3"/>',
  more: '<circle cx="6" cy="12" r="1.3"/><circle cx="12" cy="12" r="1.3"/><circle cx="18" cy="12" r="1.3"/>',
  undo: '<path d="M9 7L4.5 11.5 9 16"/><path d="M5 11.5h9a5 5 0 0 1 0 10h-3"/>',
  redo: '<path d="M15 7l4.5 4.5L15 16"/><path d="M19 11.5h-9a5 5 0 0 0 0 10h3"/>',
  voxel: '<path d="M4 8l8-4 8 4v8l-8 4-8-4z"/><path d="M4 8l8 4 8-4M12 12v8"/>',
  grid: '<path d="M3 9h18M3 15h18M9 3v18M15 3v18"/>',
  solo: '<circle cx="12" cy="12" r="4"/><path d="M3 12h3M18 12h3M12 3v3M12 18v3"/>',
  wireframe: '<path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z"/><path d="M12 12l8-4.5M12 12v9M12 12L4 7.5M4 7.5l16 9M20 7.5l-16 9" opacity="0.5"/>',
  xray: '<circle cx="12" cy="12" r="8" stroke-dasharray="3 2"/><circle cx="12" cy="12" r="3"/>',
  frame: '<path d="M4 9V5h4M16 5h4v4M20 15v4h-4M8 19H4v-4"/><circle cx="12" cy="12" r="2.5"/>',
  ortho: '<path d="M4 8l8-4 8 4-8 4z"/><path d="M4 8v8l8 4 8-4V8M12 12v8"/>',
  palette: '<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4.5 4.5"/>',
};

/** Sheets the top cluster opens: the panel's sections shown in each, by their English headings. */
const SHEETS: [string, string, string[]][] = [
  ['files', 'File', ['File']],
  ['scene', 'Scene', ['Add', 'Build', 'Object', 'Scene', 'Selection Sets']],
  ['tools', 'Tool settings', ['Brush']],
  ['topology', 'Topology', ['Topology', 'Multiresolution', 'Edit']],
  ['material', 'Paint and material', ['Paint', 'Layers', 'UV']],
  ['light', 'Lighting', ['Lighting']],
  ['view', 'View', ['View']],
  ['more', 'More', ['History', 'Keys', 'Plugins', 'Active mesh', 'Engine']],
];

/** Quick-bar and top-right buttons: [glyph, label, command id]. */
const QUICK: [string, string, string][] = [
  ['undo', 'Undo', 'history.undo'],
  ['redo', 'Redo', 'history.redo'],
  ['voxel', 'Voxel remesh', 'topo.remesh_voxel'],
  ['wireframe', 'Wireframe', 'view.wireframe'],
  ['xray', 'X-ray', 'view.xray'],
  ['grid', 'Floor grid', 'view.floor_grid'],
  ['solo', 'Solo', 'object.isolate'],
];
const CORNER: [string, string, string][] = [
  ['frame', 'Frame active object', 'view.frame'],
  ['ortho', 'Orthographic', 'view.ortho'],
  ['palette', 'Command palette', 'app.palette'],
];

function glyph(name: string, fallback: string): SVGSVGElement | HTMLSpanElement {
  const body = GLYPHS[name];
  if (!body) {
    const s = el('span', 'tl-initials', fallback.slice(0, 2));
    return s;
  }
  const svg = document.createElementNS(SVG, 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('aria-hidden', 'true');
  svg.setAttribute('fill', 'none');
  svg.setAttribute('stroke', 'currentColor');
  svg.setAttribute('stroke-width', '1.6');
  svg.setAttribute('stroke-linecap', 'round');
  svg.setAttribute('stroke-linejoin', 'round');
  svg.innerHTML = body;
  return svg;
}

function iconButton(glyphName: string, label: string, cls: string): HTMLButtonElement {
  const b = el('button', cls);
  b.type = 'button';
  b.title = label;
  b.setAttribute('aria-label', label);
  b.append(glyph(glyphName, label));
  return b;
}

/** Sets a panel control's value as a person would, so its own listeners run. */
function write(control: HTMLInputElement | HTMLSelectElement, value: string): void {
  if (control.value === value) return;
  control.value = value;
  control.dispatchEvent(new Event('input', { bubbles: true }));
  control.dispatchEvent(new Event('change', { bubbles: true }));
}

export class TabletLayout {
  private on = false;
  private sheet: string | null = null;
  private tool = '';
  private timer = 0;
  private readonly sectionOf = new Map<Element, string>();
  private readonly toolLabels = new Map<string, string>();
  private readonly rail = el('div', 'tl-rail');
  private readonly sliders = el('div', 'tl-sliders');
  private readonly radius: HTMLInputElement;
  private readonly strength: HTMLInputElement;
  private readonly radiusValue = el('span', 'tl-value');
  private readonly strengthValue = el('span', 'tl-value');
  private readonly top = el('div', 'tl-top');
  private readonly corner = el('div', 'tl-corner');
  private readonly bottom = el('div', 'tl-bottom');
  private readonly pill = el('div', 'tl-pill');
  private readonly strip = el('div', 'tl-strip');
  private readonly quick = el('div', 'tl-quick');
  /** Re-reads every stand-in from the control it mirrors; set when the strip is drawn. */
  private syncs: (() => void)[] = [];

  constructor(private parts: TabletParts) {
    // Headings are read now, before the page translator changes them, so sheets keep finding their sections.
    let section = '';
    for (const n of Array.from(parts.panel.children)) {
      if (n.tagName === 'H2') section = n.textContent ?? '';
      this.sectionOf.set(n, section);
    }
    for (const b of parts.brushButtons) this.toolLabels.set(b.dataset.brush ?? '', b.textContent ?? '');

    for (const kind of RAIL_TOOLS) {
      if (!this.toolLabels.has(kind)) continue;
      const label = this.toolLabels.get(kind) ?? kind;
      const b = iconButton(kind, label, 'tl-tool');
      b.dataset.cmd = `tool.brush.${kind}`;
      this.rail.append(b);
    }

    const slider = (label: string, source: HTMLInputElement, value: HTMLSpanElement): HTMLInputElement => {
      const wrap = el('label', 'tl-slider');
      const range = el('input');
      range.type = 'range';
      range.min = source.min;
      range.max = source.max;
      range.step = source.step || '1';
      range.value = source.value;
      range.setAttribute('aria-label', label);
      range.addEventListener('input', () => {
        write(source, range.value);
        setText(value, range.value);
      });
      wrap.append(el('span', 'tl-slider-name', label), range, value);
      this.sliders.append(wrap);
      return range;
    };
    this.radius = slider('Radius', parts.radius, this.radiusValue);
    this.strength = slider('Intensity', parts.strength, this.strengthValue);

    for (const [key, label] of SHEETS) {
      const b = iconButton(key, label, 'tl-top-btn');
      b.dataset.sheet = key;
      b.setAttribute('aria-pressed', 'false');
      b.addEventListener('click', () => this.openSheet(this.sheet === key ? null : key));
      this.top.append(b);
    }
    for (const [g, label, cmd] of CORNER) {
      const b = iconButton(g, label, 'tl-top-btn');
      b.dataset.cmd = cmd;
      this.corner.append(b);
    }
    for (const [g, label, cmd] of QUICK) {
      const b = iconButton(g, label, 'tl-quick-btn');
      b.dataset.cmd = cmd;
      b.append(el('span', 'tl-quick-label', label));
      this.quick.append(b);
    }
    this.bottom.append(this.pill, this.strip);
    parts.stage.append(this.rail, this.sliders, this.top, this.corner, this.bottom, this.quick);

    let saved = false;
    try {
      saved = localStorage.getItem(STORE) === 'on';
    } catch {
      // storage blocked; the layout starts as the panel layout
    }
    this.set(saved, false);
  }

  get enabled(): boolean {
    return this.on;
  }

  /** Switches between the tablet layout and the panel layout. */
  set(on: boolean, keep = true): void {
    this.on = on;
    this.parts.root.classList.toggle('tablet', on);
    if (keep) {
      try {
        localStorage.setItem(STORE, on ? 'on' : 'off');
      } catch {
        // storage blocked; the choice lasts for this session
      }
    }
    window.clearInterval(this.timer);
    if (on) {
      this.sync();
      // The panel's controls change from hotkeys, menus and agents as well as from here, so the stand-ins re-read
      // them a few times a second; writes happen only on a change, so nothing redraws while nothing changes.
      this.timer = window.setInterval(() => this.sync(), 250);
    } else {
      this.openSheet(null);
    }
  }

  toggle(): void {
    this.set(!this.on);
  }

  /** Shows one sheet (the panel, floating, with only that sheet's sections) or none. */
  openSheet(key: string | null): void {
    this.sheet = key;
    const wanted = new Set(SHEETS.find(([k]) => k === key)?.[2] ?? []);
    for (const [n, section] of this.sectionOf) n.classList.toggle('tl-off', key !== null && !wanted.has(section));
    this.parts.panel.classList.toggle('tl-sheet', key !== null);
    for (const b of Array.from(this.top.querySelectorAll<HTMLButtonElement>('[data-sheet]'))) setAttr(b, 'aria-pressed', b.dataset.sheet === key ? 'true' : 'false');
    if (key !== null) this.parts.panel.scrollTop = 0;
  }

  private currentTool(): string {
    return this.parts.brushButtons.find((b) => b.getAttribute('aria-pressed') === 'true')?.dataset.brush ?? '';
  }

  private sync(): void {
    setText(this.radiusValue, this.parts.radius.value);
    setText(this.strengthValue, this.parts.strength.value);
    if (this.radius.value !== this.parts.radius.value) this.radius.value = this.parts.radius.value;
    if (this.strength.value !== this.parts.strength.value) this.strength.value = this.parts.strength.value;
    const tool = this.currentTool();
    if (tool !== this.tool) {
      this.tool = tool;
      setText(this.pill, t(this.toolLabels.get(tool) ?? tool));
      this.drawStrip(tool);
    }
    for (const s of this.syncs) s();
  }

  /** The strip of the tool's options: a chip per checkbox, a small select, slider, colour or button per control. */
  private drawStrip(tool: string): void {
    this.syncs = [];
    const items: HTMLElement[] = [];
    for (const { label, control } of this.parts.options(tool)) {
      if (control instanceof HTMLButtonElement) {
        const b = el('button', 'tl-chip', label);
        b.type = 'button';
        b.addEventListener('click', () => control.click());
        this.syncs.push(() => {
          b.disabled = control.disabled;
        });
        items.push(b);
      } else if (control instanceof HTMLSelectElement) {
        const wrap = el('label', 'tl-opt', label);
        const s = el('select');
        s.setAttribute('aria-label', label);
        for (const o of Array.from(control.options)) {
          const c = el('option', '', o.textContent ?? o.value);
          c.value = o.value;
          s.append(c);
        }
        s.value = control.value;
        s.addEventListener('change', () => write(control, s.value));
        this.syncs.push(() => {
          if (s.value !== control.value) s.value = control.value;
        });
        wrap.append(s);
        items.push(wrap);
      } else if (control.type === 'checkbox') {
        const b = el('button', 'tl-chip', label);
        b.type = 'button';
        b.addEventListener('click', () => control.click());
        this.syncs.push(() => {
          setAttr(b, 'aria-pressed', control.checked ? 'true' : 'false');
          b.disabled = control.disabled;
        });
        items.push(b);
      } else {
        const wrap = el('label', 'tl-opt', label);
        const i = el('input');
        i.type = control.type === 'color' ? 'color' : control.type === 'range' ? 'range' : 'number';
        for (const a of ['min', 'max', 'step'] as const) if (control[a]) i[a] = control[a];
        i.value = control.value;
        i.setAttribute('aria-label', label);
        i.addEventListener('input', () => write(control, i.value));
        this.syncs.push(() => {
          if (document.activeElement !== i && i.value !== control.value) i.value = control.value;
        });
        wrap.append(i);
        items.push(wrap);
      }
    }
    this.strip.replaceChildren(...items);
    this.strip.hidden = items.length === 0;
    for (const s of this.syncs) s();
  }
}
