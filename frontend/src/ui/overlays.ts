// Display overlays: darkening the objects that are not active, the wire on the active object only, and a statistics
// readout over the viewport. The choices are kept in this browser and travel in layout files.

import { el, setText } from './dom';
import { t } from './i18n';

export const OVERLAYS_STORE = 'formsculpt.overlays';

export type WireScope = 'all' | 'active';

export interface OverlayPrefs {
  /** How much the objects that are not active are darkened, in percent (0 is off). */
  dimOthers: number;
  /** Which objects the wireframe is drawn on. */
  wireScope: WireScope;
  /** The statistics readout over the viewport. */
  stats: boolean;
}

export const DEFAULT_OVERLAYS: OverlayPrefs = { dimOthers: 0, wireScope: 'all', stats: false };

/** The darkening a toggle switches on when it was off. */
export const DIM_ON = 50;

export function parseOverlays(raw: unknown): OverlayPrefs {
  const o = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {};
  const dim = typeof o.dimOthers === 'number' && Number.isFinite(o.dimOthers) ? Math.round(Math.max(0, Math.min(90, o.dimOthers))) : DEFAULT_OVERLAYS.dimOthers;
  return {
    dimOthers: dim,
    wireScope: o.wireScope === 'active' ? 'active' : 'all',
    stats: typeof o.stats === 'boolean' ? o.stats : DEFAULT_OVERLAYS.stats,
  };
}

export function loadOverlays(): OverlayPrefs {
  try {
    return parseOverlays(JSON.parse(localStorage.getItem(OVERLAYS_STORE) ?? 'null'));
  } catch {
    return { ...DEFAULT_OVERLAYS };
  }
}

export function saveOverlays(p: OverlayPrefs): void {
  try {
    localStorage.setItem(OVERLAYS_STORE, JSON.stringify(p));
  } catch {
    // Storage blocked: the choice lasts until the page closes.
  }
}

export interface StatsScene {
  objects: { id: number; name: string; visible: boolean; vertices: number; triangles: number; level: number; levels: number }[];
  active: number | null;
}

const count = (n: number): string => n.toLocaleString('en-US');

/** The statistics readout: the scene's visible objects and their counts, the active object's, and the frame time.
 * It sits over the viewport's top-left corner, takes no pointer and is absolutely placed, so it never moves
 * anything; each line is written only when its text changes. */
export class StatsOverlay {
  readonly element: HTMLElement;
  private readonly scene: HTMLElement;
  private readonly active: HTMLElement;
  private readonly frame: HTMLElement;
  private frameAt = 0;

  constructor() {
    this.element = el('div', 'stats-overlay');
    this.element.setAttribute('aria-label', 'Statistics');
    this.element.setAttribute('role', 'status');
    this.element.hidden = true;
    this.scene = el('div', '', '');
    this.active = el('div', '', '');
    this.frame = el('div', 'stats-frame', '');
    this.element.append(this.scene, this.active, this.frame);
  }

  show(on: boolean): void {
    if (this.element.hidden === on) this.element.hidden = !on;
  }

  update(s: StatsScene): void {
    const shown = s.objects.filter((o) => o.visible);
    const verts = shown.reduce((a, o) => a + o.vertices, 0);
    const tris = shown.reduce((a, o) => a + o.triangles, 0);
    setText(this.scene, `${t('Objects')} ${shown.length}/${s.objects.length} · ${t('Vertices')} ${count(verts)} · ${t('Triangles')} ${count(tris)}`);
    const a = s.objects.find((o) => o.id === s.active);
    const level = a && a.levels > 1 ? ` · ${t('Level')} ${a.level}/${a.levels - 1}` : '';
    setText(this.active, a ? `${a.name} · ${count(a.vertices)} / ${count(a.triangles)}${level}` : t('No active object'));
  }

  /** The time the last frame took to draw, written at most four times a second. */
  frameTime(ms: number): void {
    if (this.element.hidden) return;
    const now = performance.now();
    if (now - this.frameAt < 250) return;
    this.frameAt = now;
    setText(this.frame, `${t('Frame')} ${ms.toFixed(1)} ms`);
  }
}
