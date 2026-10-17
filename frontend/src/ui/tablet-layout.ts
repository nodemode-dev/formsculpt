/**
 * The tablet layout: the arrangement the owner asked for from their recording of a tablet sculpting app, as a second
 * face of the same window. The viewport fills the screen on black. Floating bars sit in the corners: top left the scene
 * menus (files, scene, remesh, material, lighting, camera, history, more) with the scene's counts under them; top right
 * the tool menus with view buttons and a view cube under them. The tool palette runs down the right edge (a column of
 * the usual tools, or every tool in a grid), a narrow strip on the left holds symmetry, quick tool swaps and tall Radius
 * and Intensity sliders, the active tool's options sit in a card at the bottom, and a quick bar at the bottom left holds
 * undo, redo and the view toggles. Menus open as floating panels with a title bar.
 *
 * Nothing here is a feature of its own: every tool, toggle and view button carries data-cmd and runs its registry
 * command (so its pressed and disabled state follows the command), every slider and option writes the panel control it
 * stands for, as a person would, and reads it back, and the floating panels are the panel's own sections.
 */

import { el, setAttr, setText } from './dom';
import { t } from './i18n';
import { familyOf } from './tool-family';

const STORE = 'formsculptor.tablet';
const WIDE_STORE = 'formsculptor.tablet.tools';
/** Set once this layout has been opened as the default (0.75.1), so an "off" kept from before then is not obeyed. */
const DEFAULT_SEEN = 'formsculptor.tablet.default';
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
  /** The options shown in the bottom card for a tool (by its brush kind). */
  options: (tool: string) => TabletOption[];
  /** The Voxel panel's controls: the panel's voxel resolution and thicken fields. */
  voxelResolution: HTMLInputElement;
  voxelThicken: HTMLInputElement;
  /** The undo history list, read for the undo and redo counts. */
  history: HTMLUListElement;
}

export interface TabletStats {
  vertices: number;
  triangles: number;
  active: { name: string; vertices: number } | null;
}


/** The palette's column, in order; the grid shows every tool. */
const COLUMN = ['clay', 'draw', 'grab', 'smooth', 'flatten', 'crease', 'pinch', 'inflate', 'layer', 'trim', 'mask', 'paint', 'gizmo', 'tube', 'orbit'];

/** Line glyphs on a 24 by 24 grid. */
const GLYPHS: Record<string, string> = {
  draw: '<path d="M4 16c3-6 6-8 8-8s3 3 5 3 3-1 3-1"/>',
  clay: '<path d="M5 9c4-3 10-3 14 0M5 13c4-3 10-3 14 0M5 17c4-3 10-3 14 0"/>',
  inflate: '<circle cx="12" cy="12" r="6"/><path d="M12 9v6M9 12h6"/>',
  smooth: '<path d="M4 14c3-3 5-3 8 0s5 3 8 0"/>',
  flatten: '<path d="M4 15h16M12 5v6M9.5 8.5L12 11l2.5-2.5"/>',
  crease: '<path d="M4 8l8 8 8-8"/>',
  pinch: '<path d="M5 6l6 6-6 6M19 6l-6 6 6 6"/>',
  grab: '<path d="M12 4v16M4 12h16M12 4l-2.5 2.5M12 4l2.5 2.5M12 20l-2.5-2.5M12 20l2.5-2.5M4 12l2.5-2.5M4 12l2.5 2.5M20 12l-2.5-2.5M20 12l-2.5 2.5"/>',
  snake_hook: '<path d="M6 19c0-7 4-12 10-12 3 0 4 2 4 4s-2 3-3 3"/>',
  layer: '<path d="M4 15h16M7 15v-4h10v4"/>',
  trim: '<path d="M5 18L18 5M8 18h10V8"/>',
  mask: '<path d="M12 3l7 3v5c0 5-3 8-7 10-4-2-7-5-7-10V6z"/>',
  paint: '<path d="M14.5 4.5l5 5L10 19l-5 .5.5-5z"/>',
  smudge: '<path d="M5 15c2-4 5-6 8-6s4 2 6 1M7 19c2-2 4-3 7-3"/>',
  tube: '<path d="M5 18c2-8 6-12 14-12"/><circle cx="5" cy="18" r="1.6"/><circle cx="19" cy="6" r="1.6"/>',
  curve: '<path d="M4 18C8 4 16 20 20 6"/>',
  gizmo: '<circle cx="12" cy="12" r="7"/><path d="M5 12h14M12 5v14" opacity="0.7"/>',
  zmodeler: '<path d="M5 5h14v14H5z"/><circle cx="12" cy="12" r="1.6" fill="currentColor"/>',
  orbit: '<rect x="4" y="7" width="16" height="11" rx="2"/><circle cx="12" cy="12.5" r="3"/><path d="M9 7l1.5-2h3L15 7"/>',
  relax: '<path d="M4 12h3l2-3 3 6 2-3h6"/>',
  planar: '<path d="M4 16l8-4 8 4-8 4z"/>',
  noise: '<path d="M4 13l2-3 2 4 2-5 2 6 2-4 2 3 2-2 2 2"/>',
  elastic: '<path d="M6 12c0-4 3-6 6-6s6 2 6 6-3 6-6 6"/>',
  polish: '<path d="M4 16h16M7 12l5-5 5 5"/>',
  lattice: '<path d="M5 5h14v14H5zM5 12h14M12 5v14"/>',
  insert: '<rect x="4" y="9" width="10" height="10" rx="2"/><path d="M17 4v6M14 7h6"/>',
  cut: '<path d="M6 6l12 12M18 6L6 18"/>',
  eyedropper: '<path d="M14 4l6 6-9 9H5v-6z"/>',
  mask_box: '<rect x="5" y="6" width="14" height="12" rx="1" stroke-dasharray="3 2"/>',
  mask_lasso: '<path d="M6 14c-2-4 1-9 7-9s8 4 6 7-7 4-10 3"/><path d="M9 15l-2 5"/>',
  mask_topo: '<circle cx="12" cy="12" r="2"/><circle cx="12" cy="12" r="6" stroke-dasharray="2 2"/>',
  mask_group: '<rect x="4" y="4" width="7" height="7" rx="1"/><rect x="13" y="13" width="7" height="7" rx="1"/>',
  hide_group: '<path d="M3 12s3-6 9-6 9 6 9 6-3 6-9 6-9-6-9-6z"/><path d="M4 20L20 4"/>',
  group_brush: '<rect x="4" y="4" width="7" height="7" rx="1"/><path d="M14 18l5-5"/>',
  layer_reset: '<path d="M4 15h16M7 15v-4h10v4"/><path d="M8 6l8 0" stroke-dasharray="2 2"/>',
  stamp: '<path d="M9 4h6v6h3l-1 6H7l-1-6h3zM6 20h12"/>',
  dragdot: '<circle cx="8" cy="16" r="3"/><path d="M11 13l7-7M14 6h4v4"/>',
  silhouette: '<path d="M8 20V9l4-5 4 5v11z"/>',
  plugin: '<path d="M9 4h6v4a2 2 0 1 0 0 4v4h-6v-4a2 2 0 1 1 0-4z"/>',
  vdm: '<path d="M6 14c0-5 3-9 6-9s6 4 6 9z"/><path d="M10 14v5h4v-5"/>',
  trim_shape: '<path d="M6 14c-2-4 1-9 7-9s8 4 6 7-7 4-10 3"/><path d="M4 20L20 4"/>',
  slice_curve: '<path d="M4 16C8 6 16 18 20 8"/><path d="M4 20L20 4" opacity="0.5"/>',
  trim_curve: '<path d="M4 16C8 6 16 18 20 8"/><path d="M4 20h16"/>',
  insert_brush: '<path d="M4 18c4-8 8-8 16-12"/><circle cx="8" cy="15" r="1.5" fill="currentColor"/><circle cx="14" cy="10" r="1.5" fill="currentColor"/>',
  retopo: '<path d="M5 5h14v14H5zM5 12h14M12 5v14"/>',
  retopo_patch: '<path d="M4 8l8-4 8 4v8l-8 4-8-4z"/><path d="M4 12l8 4 8-4"/>',
  edge_loop: '<ellipse cx="12" cy="12" rx="8" ry="4"/><path d="M4 12v0"/>',
  bevel_edge: '<path d="M5 19V10l5-5h9"/>',
  clip: '<path d="M4 14h16"/><path d="M7 14V8h10v6" opacity="0.5"/>',
  zsphere: '<circle cx="8" cy="15" r="4"/><circle cx="17" cy="8" r="2.5"/><path d="M11 12l4-2.5"/>',
  pose_joint: '<circle cx="12" cy="12" r="2"/><path d="M12 12L6 18M12 12l6-6M14 6h4v4"/>',
  groom_comb: '<path d="M5 6h14M7 6v10M10 6v12M13 6v10M16 6v12"/>',
  groom_cut: '<path d="M7 7v10M11 7v6M15 7v10"/><path d="M4 15l16-4"/>',
  files: '<path d="M3.5 7.5a2 2 0 0 1 2-2H10l2 2.5h6.5a2 2 0 0 1 2 2v7.5a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>',
  scene: '<circle cx="6" cy="6" r="2"/><circle cx="18" cy="8" r="2"/><circle cx="10" cy="18" r="2"/><path d="M8 6.5l8 1M7 8l2.5 8M16.5 9.5l-5 7"/>',
  remesh: '<path d="M8 4l4 2v4l-4 2-4-2V6zM16 10l4 2v4l-4 2-4-2v-4zM8 12l4 2v4l-4 2-4-2v-4z"/>',
  material: '<circle cx="12" cy="12" r="8"/><path d="M4 12h16M12 4c3 3 3 13 0 16M12 4c-3 3-3 13 0 16"/>',
  light: '<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2.5M12 19v2.5M2.5 12H5M19 12h2.5M5.3 5.3l1.8 1.8M16.9 16.9l1.8 1.8M5.3 18.7l1.8-1.8M16.9 7.1l1.8-1.8"/>',
  camera: '<rect x="3" y="7" width="13" height="10" rx="2"/><path d="M16 11l5-3v8l-5-3"/>',
  history: '<circle cx="12" cy="12" r="8"/><path d="M12 7v5l3 2"/>',
  more: '<circle cx="6" cy="12" r="1.3"/><circle cx="12" cy="12" r="1.3"/><circle cx="18" cy="12" r="1.3"/>',
  tool: '<circle cx="12" cy="12" r="3"/><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M5.6 18.4l2.1-2.1M16.3 7.7l2.1-2.1"/>',
  stroke: '<path d="M4 18c4 0 4-12 8-12s4 12 8 12"/>',
  symmetry: '<path d="M12 3v18" stroke-dasharray="2 2"/><path d="M10 6L4 18h6zM14 6l6 12h-6z"/>',
  dyntopo: '<path d="M4 18L12 4l8 14zM8 11h8M12 4v14"/>',
  layers: '<path d="M12 4l8 4-8 4-8-4zM4 12l8 4 8-4M4 16l8 4 8-4"/>',
  interface: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/>',
  search: '<circle cx="11" cy="11" r="6"/><path d="M16 16l4 4"/>',
  palette: '<rect x="4" y="4" width="6" height="6" rx="1"/><rect x="14" y="4" width="6" height="6" rx="1"/><rect x="4" y="14" width="6" height="6" rx="1"/><rect x="14" y="14" width="6" height="6" rx="1"/>',
  ortho: '<path d="M4 8l8-4 8 4-8 4z"/><path d="M4 8v8l8 4 8-4V8M12 12v8"/>',
  fullscreen: '<path d="M4 9V5h4M16 5h4v4M20 15v4h-4M8 19H4v-4"/>',
  grid: '<path d="M3 9h18M3 15h18M9 3v18M15 3v18"/>',
  home: '<path d="M4 11l8-7 8 7M6 10v9h12v-9"/>',
  undo: '<path d="M9 7L4.5 11.5 9 16"/><path d="M5 11.5h9a5 5 0 0 1 0 10h-3"/>',
  redo: '<path d="M15 7l4.5 4.5L15 16"/><path d="M19 11.5h-9a5 5 0 0 0 0 10h3"/>',
  solo: '<circle cx="10" cy="10" r="6"/><path d="M14.5 14.5L20 20"/>',
  xray: '<circle cx="8" cy="12" r="3.5"/><circle cx="16" cy="12" r="3.5"/><path d="M11.5 12h1"/>',
  voxel: '<path d="M8 4l4 2v4l-4 2-4-2V6zM16 10l4 2v4l-4 2-4-2v-4zM8 12l4 2v4l-4 2-4-2v-4z"/>',
  wireframe: '<circle cx="12" cy="12" r="8"/><path d="M4 12h16M12 4c3 3 3 13 0 16M12 4c-3 3-3 13 0 16"/>',
  inspector: '<circle cx="11" cy="11" r="6"/><path d="M16 16l4 4M8 11h6"/>',
  logs: '<rect x="5" y="4" width="14" height="16" rx="2"/><path d="M8 9h8M8 13h8M8 17h5"/>',
  collapse: '<path d="M7 10l5 5 5-5"/>',
  menu: '<path d="M5 7h14M5 12h14M5 17h14"/>',
  close: '<path d="M6 6l12 12M18 6L6 18"/>',
  pin: '<path d="M9 4h6l-1 6 3 3H7l3-3zM12 13v7"/>',
  // Option glyphs, by what the option does.
  invert: '<circle cx="12" cy="12" r="7"/><path d="M12 5a7 7 0 0 1 0 14z" fill="currentColor"/>',
  mirror: '<path d="M12 3v18" stroke-dasharray="2 2"/><path d="M10 6L4 18h6zM14 6l6 12h-6z"/>',
  radial: '<circle cx="12" cy="12" r="7"/><path d="M12 5v14M5 12h14M7 7l10 10M17 7L7 17"/>',
  lazy: '<path d="M4 18c3-6 6 0 9-6s5-6 7-6"/><circle cx="20" cy="6" r="1.5" fill="currentColor"/>',
  front: '<path d="M4 12h12M12 8l4 4-4 4M19 5v14"/>',
  accumulate: '<path d="M5 18h14M5 14h14M5 10h14"/>',
  closed: '<circle cx="12" cy="12" r="7"/>',
  shape: '<path d="M4 18C8 4 16 20 20 6"/>',
  round: '<path d="M5 19V11a6 6 0 0 1 6-6h8"/>',
  size: '<circle cx="12" cy="12" r="7"/><path d="M12 12h7"/>',
  cap: '<path d="M6 8h12M6 16h12M6 8v8M18 8v8"/>',
  wall: '<circle cx="12" cy="12" r="7"/><circle cx="12" cy="12" r="3.5"/>',
  apply: '<path d="M5 12l5 5 9-10"/>',
  clear: '<path d="M6 7h12M9 7V5h6v2M8 7l1 12h6l1-12"/>',
  colour: '<circle cx="12" cy="12" r="7"/>',
  rough: '<path d="M4 15l3-3 3 3 3-3 3 3 3-3"/>',
  metal: '<path d="M6 18l6-12 6 12z"/>',
  preset: '<rect x="5" y="5" width="14" height="14" rx="3"/><path d="M9 12h6"/>',
  texture: '<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M4 12h16M12 4v16"/>',
  action: '<path d="M5 12h14M13 6l6 6-6 6"/>',
  amount: '<path d="M4 18h16M4 18V8M9 18v-5M14 18V6M19 18v-8"/>',
  pivot: '<circle cx="12" cy="12" r="2.5"/><path d="M12 3v5M12 16v5M3 12h5M16 12h5"/>',
  pick: '<path d="M5 5l5 13 2-5 5-2z"/>',
  angle: '<path d="M5 19h14M5 19L16 6"/><path d="M11 19a6 6 0 0 0-1.7-4.2"/>',
  snap: '<path d="M5 12h12M13 7l5 5-5 5"/>',
  mask_action: '<path d="M12 3l7 3v5c0 5-3 8-7 10-4-2-7-5-7-10V6z"/>',
};

/** An option's glyph from its label (the English one, before translation). */
function optionGlyph(label: string): string {
  const l = label.toLowerCase();
  const pairs: [string, string][] = [
    ['invert', 'invert'], ['mirror', 'mirror'], ['radial', 'radial'], ['dyntopo', 'dyntopo'], ['lazy', 'lazy'], ['front faces', 'front'],
    ['accumulate', 'accumulate'], ['closed', 'closed'], ['path shape', 'shape'], ['corner', 'round'], ['radius', 'size'], ['ends', 'cap'],
    ['wall', 'wall'], ['profile', 'shape'], ['apply', 'apply'], ['clear', 'clear'], ['colour', 'colour'], ['rough', 'rough'], ['metal', 'metal'],
    ['preset', 'preset'], ['texture', 'texture'], ['action', 'action'], ['amount', 'amount'], ['loop', 'amount'], ['crease', 'crease'],
    ['pivot', 'pivot'], ['pick', 'pick'], ['angle', 'angle'], ['snap', 'snap'], ['mask', 'mask_action'], ['grow', 'mask_action'], ['shrink', 'mask_action'], ['blur', 'mask_action'],
  ];
  for (const [key, g] of pairs) if (l.includes(key)) return g;
  return 'tool';
}

function glyph(name: string): SVGSVGElement {
  const svg = document.createElementNS(SVG, 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('aria-hidden', 'true');
  svg.setAttribute('fill', 'none');
  svg.setAttribute('stroke', 'currentColor');
  svg.setAttribute('stroke-width', '2');
  svg.setAttribute('stroke-linecap', 'round');
  svg.setAttribute('stroke-linejoin', 'round');
  svg.innerHTML = GLYPHS[name] ?? GLYPHS.tool;
  return svg;
}

/** An icon-over-label button (the bars, the quick bar, the options card); `label` is English and shown translated. */
function labelled(icon: string, label: string, cls: string): HTMLButtonElement {
  const b = el('button', cls);
  b.type = 'button';
  b.title = t(label);
  b.setAttribute('aria-label', t(label));
  b.append(glyph(icon), el('span', 'tb-label', t(label)));
  return b;
}

function iconOnly(icon: string, label: string, cls: string): HTMLButtonElement {
  const b = el('button', cls);
  b.type = 'button';
  b.title = t(label);
  b.setAttribute('aria-label', t(label));
  b.append(glyph(icon));
  return b;
}

/** Sets a panel control's value as a person would, so its own listeners run. */
function write(control: HTMLInputElement | HTMLSelectElement, value: string): void {
  if (control.value === value) return;
  control.value = value;
  control.dispatchEvent(new Event('input', { bubbles: true }));
  control.dispatchEvent(new Event('change', { bubbles: true }));
}

const fmt = (n: number): string => (n >= 1e6 ? `${(n / 1e6).toFixed(2)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(n >= 1e5 ? 0 : 1)}k` : String(n));

/** A value dragged along a bar, its share filled: horizontal (a field with its name inside) or vertical (a tall slider). */
class DragValue {
  readonly node: HTMLElement;
  private readonly fill = el('span', 'tb-fill');
  private readonly value = el('span', 'tb-num');

  constructor(private source: HTMLInputElement, label: string, vertical: boolean, cls: string) {
    this.node = el('div', `${cls}${vertical ? ' vertical' : ''}`);
    this.node.tabIndex = 0;
    this.node.setAttribute('role', 'slider');
    this.node.setAttribute('aria-label', t(label));
    const name = el('span', 'tb-name', t(label));
    // A second copy of the name inside the fill, so the name reads light on the empty part and dark on the fill.
    if (!vertical) this.fill.append(el('span', 'tb-name-on', t(label)));
    this.node.append(this.fill, name, this.value);
    const range = (): [number, number, number] => [Number(source.min || 0), Number(source.max || 100), Number(source.step) || 0];
    const set = (share: number): void => {
      const [lo, hi, step] = range();
      let v = lo + Math.max(0, Math.min(1, share)) * (hi - lo);
      if (step > 0) v = Math.round(v / step) * step;
      write(source, String(Number(v.toFixed(4))));
      this.sync();
    };
    this.node.addEventListener('pointerdown', (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      this.node.setPointerCapture(e.pointerId);
      const at = (m: PointerEvent): void => {
        const r = this.node.getBoundingClientRect();
        set(vertical ? (r.bottom - m.clientY) / r.height : (m.clientX - r.left) / r.width);
      };
      at(e);
      const move = (m: PointerEvent): void => at(m);
      const up = (): void => {
        this.node.removeEventListener('pointermove', move);
        this.node.removeEventListener('pointerup', up);
        this.node.removeEventListener('pointercancel', up);
      };
      this.node.addEventListener('pointermove', move);
      this.node.addEventListener('pointerup', up);
      this.node.addEventListener('pointercancel', up);
    });
    this.node.addEventListener('keydown', (e) => {
      const [lo, hi, step] = range();
      const by = step > 0 ? step : (hi - lo) / 100;
      const now = Number(source.value);
      if (e.key === 'ArrowUp' || e.key === 'ArrowRight') write(source, String(Math.min(hi, now + by)));
      else if (e.key === 'ArrowDown' || e.key === 'ArrowLeft') write(source, String(Math.max(lo, now - by)));
      else return;
      e.preventDefault();
      this.sync();
    });
    this.sync();
  }

  sync(): void {
    const lo = Number(this.source.min || 0);
    const hi = Number(this.source.max || 100);
    const v = Number(this.source.value);
    const share = hi > lo ? (v - lo) / (hi - lo) : 0;
    this.fill.style.setProperty('--share', `${(Math.max(0, Math.min(1, share)) * 100).toFixed(1)}%`);
    setText(this.value, this.source.value);
    setAttr(this.node, 'aria-valuenow', this.source.value);
    this.node.classList.toggle('disabled', this.source.disabled);
  }
}

/** Menus the corner bars open as floating panels: [glyph, title, the panel's sections by their English headings]. */
const LEFT_MENUS: [string, string, string[]][] = [
  ['files', 'File', ['File']],
  ['scene', 'Scene', ['Scene', 'Object', 'Add', 'Build', 'Selection Sets']],
  ['remesh', 'Topology', ['Topology', 'Multiresolution', 'Edit']],
  ['material', 'Paint and material', ['Paint', 'Layers', 'UV']],
  ['light', 'Lighting', ['Lighting']],
  ['camera', 'View', ['View']],
  ['history', 'History', ['History']],
  ['more', 'More', ['Keys', 'Plugins', 'Active mesh', 'Engine']],
];
const RIGHT_MENUS: [string, string, string[]][] = [['tool', 'Tool settings', ['Brush']]];

export class TabletLayout {
  private on = false;
  private sheet: string | null = null;
  private tool = '';
  private timer = 0;
  private wide = false;
  private readonly sectionOf = new Map<Element, string>();
  private readonly toolLabels = new Map<string, string>();
  private readonly menus = new Map<string, [string, string[], 'left' | 'right']>();
  private readonly tl = el('div', 'tb-bar tb-tl');
  private readonly stats = el('div', 'tb-stats');
  private readonly statsScene = el('div', 'tb-stat');
  private readonly statsActive = el('div', 'tb-stat tb-stat-active');
  private readonly tr = el('div', 'tb-bar tb-tr');
  private readonly views = el('div', 'tb-views');
  private readonly cube = el('div', 'tb-cube');
  private readonly cubeBody = el('div', 'tb-cube-body');
  private readonly tools = el('div', 'tb-tools');
  private readonly left = el('div', 'tb-left');
  private readonly card = el('div', 'tb-card');
  private readonly cardTitle = el('span', 'tb-card-title');
  private readonly cardBody = el('div', 'tb-card-body');
  private readonly pill = el('div', 'tb-pill');
  private readonly quick = el('div', 'tb-bar tb-quick');
  private readonly undoCount = el('span', 'tb-count');
  private readonly redoCount = el('span', 'tb-count');
  private readonly sheetHead = el('div', 'tb-sheet-head');
  private readonly sheetTitle = el('span', 'tb-sheet-title');
  private readonly voxel = el('div', 'tb-float tb-voxel');
  private readonly sliders: DragValue[] = [];
  private readonly voxelFields: DragValue[] = [];
  /** Re-reads every stand-in in the options card from the control it mirrors; set when the card is drawn. */
  private syncs: (() => void)[] = [];

  /** Set by the app: the scene's counts for the stats under the top left bar. */
  statsOf: (() => TabletStats) | null = null;
  /** Set by the app: the camera's turn and tilt in radians, for the view cube. */
  camera: (() => { yaw: number; pitch: number }) | null = null;
  /** Set by the app: called when the layout is switched on or off (the black background and orange clay go with it). */
  onToggle: ((on: boolean) => void) | null = null;

  constructor(private parts: TabletParts) {
    // Headings are read now, before the page translator changes them, so menus keep finding their sections.
    let section = '';
    for (const n of Array.from(parts.panel.children)) {
      if (n.tagName === 'H2') section = n.textContent ?? '';
      this.sectionOf.set(n, section);
    }
    for (const b of parts.brushButtons) this.toolLabels.set(b.dataset.brush ?? '', b.textContent ?? '');

    // Top left: the scene menus, and the counts under them.
    for (const [g, title, sections] of LEFT_MENUS) {
      this.menus.set(g, [title, sections, 'left']);
      const b = iconOnly(g, title, 'tb-icon');
      b.dataset.menu = g;
      b.setAttribute('aria-pressed', 'false');
      b.addEventListener('click', () => this.openSheet(this.sheet === g ? null : g));
      this.tl.append(b);
    }
    this.stats.append(this.statsScene, this.statsActive);

    // Top right: tool settings, the stroke toggles, the palette's grid and the way back to the panel layout.
    for (const [g, title, sections] of RIGHT_MENUS) {
      this.menus.set(g, [title, sections, 'right']);
      const b = iconOnly(g, title, 'tb-icon');
      b.dataset.menu = g;
      b.setAttribute('aria-pressed', 'false');
      b.addEventListener('click', () => this.openSheet(this.sheet === g ? null : g));
      this.tr.append(b);
    }
    const cmd = (g: string, label: string, id: string, into: HTMLElement, cls = 'tb-icon'): HTMLButtonElement => {
      const b = iconOnly(g, label, cls);
      b.dataset.cmd = id;
      into.append(b);
      return b;
    };
    cmd('symmetry', t('Mirror X'), 'sculpt.mirror_x', this.tr);
    cmd('dyntopo', t('Dynamic Topology'), 'sculpt.dyntopo', this.tr);
    cmd('stroke', t('Invert Brush'), 'sculpt.invert', this.tr);
    cmd('search', t('Command palette'), 'app.palette', this.tr);
    cmd('interface', t('Panel layout'), 'view.tablet_layout', this.tr);
    const grid = iconOnly('palette', t('All tools'), 'tb-icon tb-grid-toggle');
    grid.addEventListener('click', () => this.setWide(!this.wide));
    this.tr.append(grid);

    // Under the top right bar: view buttons and the view cube.
    cmd('ortho', t('Orthographic'), 'view.ortho', this.views, 'tb-small');
    const full = iconOnly('fullscreen', t('Full screen'), 'tb-small');
    full.addEventListener('click', () => {
      if (document.fullscreenElement) void document.exitFullscreen().catch(() => undefined);
      else void document.documentElement.requestFullscreen().catch(() => undefined);
    });
    this.views.append(full);
    cmd('grid', t('Floor Grid'), 'view.floor_grid', this.views, 'tb-small');
    cmd('home', t('Frame active object'), 'view.frame', this.views, 'tb-small');
    for (const [face, view, label] of [['front', 'view.front', 'Front'], ['back', 'view.back', 'Back'], ['right', 'view.right', 'Right'], ['left', 'view.left', 'Left'], ['top', 'view.top', 'Top'], ['bottom', 'view.bottom', 'Bottom']]) {
      const f = el('button', `tb-face ${face}`, t(label));
      f.type = 'button';
      f.dataset.cmd = view;
      f.setAttribute('aria-label', t(`View ${label}`));
      this.cubeBody.append(f);
    }
    this.cube.append(this.cubeBody);

    // Right: the tool palette.
    for (const b of parts.brushButtons) {
      const kind = b.dataset.brush ?? '';
      const label = this.toolLabels.get(kind) ?? kind;
      const tile = el('button', `tb-tool ${familyOf(kind)}`);
      tile.type = 'button';
      tile.dataset.cmd = `tool.brush.${kind}`;
      tile.dataset.kind = kind;
      tile.title = t(label);
      tile.setAttribute('aria-label', t(label));
      const disc = el('span', 'tb-disc');
      if (GLYPHS[kind]) disc.append(glyph(kind));
      else disc.append(el('span', 'tb-letter', t(label).slice(0, 1)));
      tile.append(disc, el('span', 'tb-label', t(label)));
      const at = COLUMN.indexOf(kind);
      tile.style.setProperty('--order', String(at < 0 ? 100 : at));
      tile.classList.toggle('in-column', at >= 0);
      this.tools.append(tile);
    }

    // Left: symmetry, quick tool swaps and the two tall sliders.
    const sym = labelled('symmetry', t('Sym'), 'tb-side');
    sym.dataset.cmd = 'sculpt.mirror_x';
    this.left.append(sym);
    for (const [kind, label] of [['smooth', 'Smooth'], ['mask', 'Mask'], ['gizmo', 'Transform']] as const) {
      const b = labelled(kind, label, 'tb-side');
      b.dataset.cmd = `tool.brush.${kind}`;
      this.left.append(b);
    }
    const radius = new DragValue(parts.radius, t('Radius'), true, 'tb-vslider');
    const intensity = new DragValue(parts.strength, t('Intensity'), true, 'tb-vslider');
    this.sliders.push(radius, intensity);
    const pair = el('div', 'tb-vsliders');
    pair.append(radius.node, intensity.node);
    this.left.append(pair);

    // Bottom: the active tool's pill and options card.
    const collapse = iconOnly('collapse', t('Fold the options'), 'tb-head-btn');
    collapse.addEventListener('click', () => this.card.classList.toggle('folded'));
    const menu = iconOnly('menu', t('Tool settings'), 'tb-head-btn');
    menu.addEventListener('click', () => this.openSheet(this.sheet === 'tool' ? null : 'tool'));
    const head = el('div', 'tb-card-head');
    head.append(collapse, this.cardTitle, menu);
    this.card.append(head, this.cardBody);

    // Bottom left: the quick bar.
    const undo = labelled('undo', t('Undo'), 'tb-qbtn');
    undo.dataset.cmd = 'history.undo';
    undo.querySelector('.tb-label')?.replaceWith(this.undoCount);
    const redo = labelled('redo', t('Redo'), 'tb-qbtn');
    redo.dataset.cmd = 'history.redo';
    redo.querySelector('.tb-label')?.replaceWith(this.redoCount);
    this.quick.append(undo, redo, el('span', 'tb-sep'));
    const q = (g: string, label: string, id: string): void => {
      const b = labelled(g, label, 'tb-qbtn');
      b.dataset.cmd = id;
      this.quick.append(b);
    };
    q('solo', t('Solo'), 'object.isolate');
    q('xray', t('X-ray'), 'view.xray');
    const vox = labelled('voxel', t('Voxel'), 'tb-qbtn');
    vox.setAttribute('aria-pressed', 'false');
    vox.addEventListener('click', () => this.showVoxel(this.voxel.hidden !== false));
    this.quick.append(vox);
    q('grid', t('Grid'), 'view.floor_grid');
    q('wireframe', t('Wireframe'), 'view.wireframe');
    const inspector = labelled('inspector', t('Inspector'), 'tb-qbtn');
    inspector.setAttribute('aria-pressed', 'true');
    inspector.addEventListener('click', () => {
      this.stats.hidden = !this.stats.hidden;
      setAttr(inspector, 'aria-pressed', this.stats.hidden ? 'false' : 'true');
    });
    this.quick.append(inspector);
    const logs = labelled('logs', t('History'), 'tb-qbtn');
    logs.addEventListener('click', () => this.openSheet(this.sheet === 'history' ? null : 'history'));
    this.quick.append(logs);

    // The Voxel panel: resolution, thicken thin parts and the remesh button, as floating panels look.
    const vhead = el('div', 'tb-float-head');
    const vclose = iconOnly('close', t('Close'), 'tb-head-btn close');
    vclose.addEventListener('click', () => this.showVoxel(false));
    vhead.append(el('span', 'tb-float-title', t('Voxel')), vclose);
    const res = new DragValue(parts.voxelResolution, t('Resolution'), false, 'tb-field');
    const thick = new DragValue(parts.voxelThicken, t('Thicken thin parts'), false, 'tb-field yellow');
    this.voxelFields.push(res, thick);
    const go = labelled('voxel', t('Remesh'), 'tb-big');
    go.dataset.cmd = 'topo.remesh_voxel';
    const vbody = el('div', 'tb-float-body');
    vbody.append(res.node, thick.node, go);
    this.voxel.append(vhead, vbody);
    this.voxel.hidden = true;
    this.draggable(this.voxel, vhead);

    // The menus' floating panel gets a title bar of its own (hidden in the panel layout).
    const close = iconOnly('close', t('Close'), 'tb-head-btn close');
    close.addEventListener('click', () => this.openSheet(null));
    this.sheetHead.append(this.sheetTitle, close);
    parts.panel.prepend(this.sheetHead);

    parts.stage.append(this.tl, this.stats, this.tr, this.views, this.cube, this.tools, this.left, this.pill, this.card, this.quick, this.voxel);

    try {
      this.wide = localStorage.getItem(WIDE_STORE) !== 'column';
    } catch {
      this.wide = true;
    }
    this.setWide(this.wide, false);
    // This layout, rebuilt to the owner's recording, is the default: it opens unless the person switched it off. A
    // browser that ran an earlier version may hold "off" from before the rebuild, so the first start of this version
    // opens it once regardless (DEFAULT_SEEN marks that), and from then on the person's choice holds.
    let saved = true;
    try {
      if (localStorage.getItem(DEFAULT_SEEN) === null) {
        localStorage.setItem(DEFAULT_SEEN, '1');
        localStorage.setItem(STORE, 'on');
      }
      saved = localStorage.getItem(STORE) !== 'off';
    } catch {
      // storage blocked; the layout starts as this one
    }
    this.set(saved, false);
  }

  get enabled(): boolean {
    return this.on;
  }

  /** Switches between the tablet layout and the panel layout. */
  set(on: boolean, keep = true): void {
    const changed = on !== this.on;
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
      this.showVoxel(false);
    }
    if (changed || !keep) this.onToggle?.(on);
  }

  toggle(): void {
    this.set(!this.on);
  }

  /** The palette as one column of the usual tools, or every tool in a grid. */
  private setWide(wide: boolean, keep = true): void {
    this.wide = wide;
    this.tools.classList.toggle('wide', wide);
    const b = this.tr.querySelector('.tb-grid-toggle');
    if (b) setAttr(b, 'aria-pressed', wide ? 'true' : 'false');
    if (!keep) return;
    try {
      localStorage.setItem(WIDE_STORE, wide ? 'grid' : 'column');
    } catch {
      // storage blocked; the choice lasts for this session
    }
  }

  private showVoxel(show: boolean): void {
    this.voxel.hidden = !show;
    for (const q of Array.from(this.quick.querySelectorAll<HTMLButtonElement>('.tb-qbtn'))) if (q.getAttribute('aria-label') === t('Voxel')) setAttr(q, 'aria-pressed', show ? 'true' : 'false');
    if (show) for (const f of this.voxelFields) f.sync();
  }

  /** Lets a floating panel be moved by its title bar. */
  private draggable(node: HTMLElement, handle: HTMLElement): void {
    handle.addEventListener('pointerdown', (e) => {
      if (e.button !== 0 || (e.target as HTMLElement).closest('button')) return;
      e.preventDefault();
      handle.setPointerCapture(e.pointerId);
      const box = this.parts.stage.getBoundingClientRect();
      const start = { x: e.clientX, y: e.clientY, left: node.offsetLeft, top: node.offsetTop };
      const move = (m: PointerEvent): void => {
        node.style.left = `${Math.max(0, Math.min(box.width - 60, start.left + m.clientX - start.x))}px`;
        node.style.top = `${Math.max(0, Math.min(box.height - 30, start.top + m.clientY - start.y))}px`;
        node.style.right = 'auto';
        node.style.bottom = 'auto';
      };
      const up = (): void => {
        handle.removeEventListener('pointermove', move);
        handle.removeEventListener('pointerup', up);
      };
      handle.addEventListener('pointermove', move);
      handle.addEventListener('pointerup', up);
    });
  }

  /** Shows one menu (the panel, floating, with only that menu's sections, on the side its button sits) or none. */
  openSheet(key: string | null): void {
    const menu = key ? this.menus.get(key) : undefined;
    this.sheet = menu ? key : null;
    const wanted = new Set(menu?.[1] ?? []);
    for (const [n, section] of this.sectionOf) n.classList.toggle('tl-off', this.sheet !== null && !wanted.has(section));
    this.parts.panel.classList.toggle('tl-sheet', this.sheet !== null);
    this.parts.panel.classList.toggle('tb-right', menu?.[2] === 'right');
    setText(this.sheetTitle, menu ? t(menu[0]) : '');
    for (const b of Array.from(this.parts.stage.querySelectorAll<HTMLButtonElement>('[data-menu]'))) setAttr(b, 'aria-pressed', b.dataset.menu === this.sheet ? 'true' : 'false');
    if (this.sheet !== null) this.parts.panel.scrollTop = 0;
  }

  private currentTool(): string {
    return this.parts.brushButtons.find((b) => b.getAttribute('aria-pressed') === 'true')?.dataset.brush ?? '';
  }

  private sync(): void {
    for (const s of this.sliders) s.sync();
    if (!this.voxel.hidden) for (const f of this.voxelFields) f.sync();
    const tool = this.currentTool();
    if (tool !== this.tool) {
      this.tool = tool;
      const label = t(this.toolLabels.get(tool) ?? tool);
      setText(this.cardTitle, label);
      this.pill.replaceChildren(glyph(GLYPHS[tool] ? tool : 'tool'), el('span', '', label));
      this.pill.className = `tb-pill ${familyOf(tool)}`;
      this.drawCard(tool);
    }
    for (const s of this.syncs) s();
    // Undo and redo counts, from the history list (Start, then one row per step; rows past the current are undone).
    const rows = Array.from(this.parts.history.children);
    const at = rows.findIndex((r) => r.getAttribute('aria-current') === 'true');
    setText(this.undoCount, String(Math.max(0, at)));
    setText(this.redoCount, String(at < 0 ? 0 : rows.length - 1 - at));
    const st = this.statsOf?.();
    if (st) {
      setText(this.statsScene, `${t('Vertices')} ${fmt(st.vertices)} · ${t('Triangles')} ${fmt(st.triangles)}`);
      setText(this.statsActive, st.active ? `${st.active.name} · ${fmt(st.active.vertices)}` : t('No object selected'));
    }
    const cam = this.camera?.();
    if (cam) {
      const tf = `rotateX(${(-cam.pitch * 180) / Math.PI}deg) rotateY(${(cam.yaw * 180) / Math.PI}deg)`;
      if (this.cubeBody.style.transform !== tf) this.cubeBody.style.transform = tf;
    }
  }

  /** The card of the tool's options: a toggle per checkbox, a button per action, a field per value or choice. */
  private drawCard(tool: string): void {
    this.syncs = [];
    const items: HTMLElement[] = [];
    for (const { label, control } of this.parts.options(tool)) {
      if (control instanceof HTMLButtonElement) {
        const b = labelled(optionGlyph(label), label, 'tb-opt');
        b.addEventListener('click', () => control.click());
        this.syncs.push(() => {
          b.disabled = control.disabled;
        });
        items.push(b);
      } else if (control instanceof HTMLSelectElement) {
        // The choice's current value over its name; a transparent select on top opens the list.
        const wrap = el('label', 'tb-opt tb-choice');
        const shown = el('span', 'tb-choice-value');
        const s = el('select');
        s.setAttribute('aria-label', t(label));
        for (const o of Array.from(control.options)) {
          const c = el('option', '', o.textContent ?? o.value);
          c.value = o.value;
          s.append(c);
        }
        s.value = control.value;
        s.addEventListener('change', () => write(control, s.value));
        this.syncs.push(() => {
          if (s.value !== control.value) s.value = control.value;
          setText(shown, s.selectedOptions[0]?.textContent ?? '');
        });
        wrap.append(shown, el('span', 'tb-label', t(label)), s);
        items.push(wrap);
      } else if (control.type === 'checkbox') {
        const b = labelled(optionGlyph(label), label, 'tb-opt');
        b.addEventListener('click', () => control.click());
        this.syncs.push(() => {
          setAttr(b, 'aria-pressed', control.checked ? 'true' : 'false');
          b.disabled = control.disabled;
        });
        items.push(b);
      } else if (control.type === 'color') {
        const wrap = el('label', 'tb-opt tb-colour');
        const i = el('input');
        i.type = 'color';
        i.value = control.value;
        i.setAttribute('aria-label', t(label));
        i.addEventListener('input', () => write(control, i.value));
        this.syncs.push(() => {
          if (document.activeElement !== i && i.value !== control.value) i.value = control.value;
        });
        wrap.append(i, el('span', 'tb-label', t(label)));
        items.push(wrap);
      } else {
        const f = new DragValue(control, label, false, 'tb-field compact');
        this.syncs.push(() => f.sync());
        items.push(f.node);
      }
    }
    this.cardBody.replaceChildren(...items);
    this.card.hidden = items.length === 0;
    for (const s of this.syncs) s();
  }
}
