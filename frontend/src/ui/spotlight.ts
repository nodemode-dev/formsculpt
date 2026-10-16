import type { Reply } from '../engine/service';

const SVG = 'http://www.w3.org/2000/svg';

export type SpotlightMode = 'off' | 'dial' | 'brush';

export interface SpotlightInfo {
  image: number;
  name: string;
  width: number;
  height: number;
  center: [number, number];
  size: number;
  angle: number;
  opacity: number;
  tile: boolean;
  flip: boolean;
  active: boolean;
}

export interface SpotlightHost {
  dispatch<T>(name: string, args: Record<string, unknown>): Promise<Reply<T>>;
  viewProj(): number[];
  activeObject(): number | null;
  brushRadius(): number;
  applyArgs(mode?: string): Record<string, unknown>;
  refresh(): Promise<void>;
  note(text: string): void;
}

interface Shown {
  info: SpotlightInfo;
  img: HTMLImageElement;
}

/** The Spotlight over the viewport: its images drawn see-through where the engine places them, a dial (move, turn,
 * size) around the active one, and a brush that paints or sculpts through it. Every change writes transforms and
 * attributes on nodes made once per image; nothing is rebuilt, and an image appears only after it has decoded. */
export class SpotlightDial {
  readonly root: HTMLDivElement;
  private readonly dial: SVGSVGElement;
  private readonly ring: SVGCircleElement;
  private readonly turnKnob: SVGCircleElement;
  private readonly sizeKnob: SVGCircleElement;
  private readonly cursor: SVGCircleElement;
  private readonly shown = new Map<number, Shown>();
  private revision = -1;
  private mode: SpotlightMode = 'off';
  private drag: { kind: 'move' | 'turn' | 'size'; x: number; y: number; start: SpotlightInfo } | null = null;
  private painting = false;
  private busy = false;
  private pending: [number, number] | null = null;

  constructor(private readonly stage: HTMLElement, private readonly host: SpotlightHost) {
    this.root = document.createElement('div');
    this.root.className = 'spotlight';
    this.dial = document.createElementNS(SVG, 'svg');
    this.dial.setAttribute('class', 'spotlight-dial');
    this.dial.setAttribute('aria-hidden', 'true');
    this.ring = document.createElementNS(SVG, 'circle');
    this.ring.setAttribute('class', 'spotlight-ring');
    this.turnKnob = document.createElementNS(SVG, 'circle');
    this.turnKnob.setAttribute('class', 'spotlight-knob');
    this.turnKnob.setAttribute('r', '7');
    this.sizeKnob = document.createElementNS(SVG, 'circle');
    this.sizeKnob.setAttribute('class', 'spotlight-knob');
    this.sizeKnob.setAttribute('r', '7');
    this.cursor = document.createElementNS(SVG, 'circle');
    this.cursor.setAttribute('class', 'spotlight-cursor');
    this.dial.append(this.ring, this.turnKnob, this.sizeKnob, this.cursor);
    this.root.append(this.dial);
    this.dial.style.visibility = 'hidden';
    this.cursor.style.visibility = 'hidden';
    stage.append(this.root);
    new ResizeObserver(() => this.place()).observe(stage);
    this.turnKnob.addEventListener('pointerdown', (e) => this.startDrag(e, 'turn'));
    this.sizeKnob.addEventListener('pointerdown', (e) => this.startDrag(e, 'size'));
    stage.addEventListener('pointerdown', (e) => this.brushDown(e), { capture: true });
    window.addEventListener('pointermove', (e) => this.pointerMove(e));
    window.addEventListener('pointerup', () => void this.pointerUp());
  }

  get images(): SpotlightInfo[] {
    return [...this.shown.values()].map((s) => s.info);
  }

  get activeInfo(): SpotlightInfo | undefined {
    return this.images.find((i) => i.active);
  }

  setMode(mode: SpotlightMode): void {
    this.mode = mode;
    this.root.dataset.mode = mode;
    for (const k of [this.ring, this.turnKnob, this.sizeKnob]) k.style.visibility = mode === 'brush' ? 'hidden' : '';
    if (mode !== 'brush') this.cursor.style.visibility = 'hidden';
    this.place();
  }

  /** Follows the engine: new images are fetched and shown once decoded, removed ones dropped, placement updated. */
  async sync(): Promise<void> {
    const r = await this.host.dispatch<{ images: SpotlightInfo[]; revision: number }>('spotlight_list', {});
    if (!r.ok || r.result.revision === this.revision) return;
    this.revision = r.result.revision;
    const ids = new Set(r.result.images.map((i) => i.image));
    for (const [id, s] of this.shown) {
      if (!ids.has(id)) {
        s.img.remove();
        this.shown.delete(id);
      }
    }
    for (const info of r.result.images) {
      const have = this.shown.get(info.image);
      if (have) {
        have.info = info;
        continue;
      }
      const px = await this.host.dispatch<{ png_base64: string }>('spotlight_image', { image: info.image });
      if (!px.ok) continue;
      const img = new Image();
      img.className = 'spotlight-image';
      img.alt = '';
      img.draggable = false;
      img.src = `data:image/png;base64,${px.result.png_base64}`;
      await img.decode();
      img.addEventListener('pointerdown', (e) => {
        if (this.mode !== 'dial') return;
        const s = [...this.shown.values()].find((x) => x.img === img);
        if (s && !s.info.active) void this.set({ image: s.info.image, activate: true });
        this.startDrag(e, 'move');
      });
      this.shown.set(info.image, { info, img });
      this.place(info.image);
      this.root.insertBefore(img, this.dial);
    }
    this.place();
  }

  /** Writes one image's box (or all of them) and the dial around the active one, in CSS pixels of the stage. */
  private place(only?: number): void {
    const w = this.stage.clientWidth;
    const h = this.stage.clientHeight;
    for (const s of this.shown.values()) {
      if (only !== undefined && s.info.image !== only) continue;
      const i = s.info;
      const ih = i.size * h;
      const iw = (ih * i.width) / i.height;
      s.img.style.width = `${iw}px`;
      s.img.style.height = `${ih}px`;
      s.img.style.transform = `translate(${i.center[0] * w - iw / 2}px, ${i.center[1] * h - ih / 2}px) rotate(${i.angle}deg) scaleX(${i.flip ? -1 : 1})`;
      s.img.style.opacity = String(Math.min(0.85, i.opacity * 0.6 + 0.1));
      s.img.classList.toggle('active', i.active);
    }
    const a = this.activeInfo;
    const showDial = (this.mode === 'dial' && !!a) || this.mode === 'brush';
    this.dial.style.visibility = showDial ? 'visible' : 'hidden';
    if (!a) return;
    const ih = a.size * h;
    const iw = (ih * a.width) / a.height;
    const cx = a.center[0] * w;
    const cy = a.center[1] * h;
    const r = Math.hypot(iw, ih) / 2;
    const turn = ((a.angle - 90) * Math.PI) / 180;
    const corner = ((a.angle + 45) * Math.PI) / 180;
    this.ring.setAttribute('cx', String(cx));
    this.ring.setAttribute('cy', String(cy));
    this.ring.setAttribute('r', String(r));
    this.turnKnob.setAttribute('cx', String(cx + r * Math.cos(turn)));
    this.turnKnob.setAttribute('cy', String(cy + r * Math.sin(turn)));
    this.sizeKnob.setAttribute('cx', String(cx + r * Math.cos(corner)));
    this.sizeKnob.setAttribute('cy', String(cy + r * Math.sin(corner)));
  }

  private startDrag(e: PointerEvent, kind: 'move' | 'turn' | 'size'): void {
    const a = this.activeInfo;
    if (this.mode !== 'dial' || !a || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    this.drag = { kind, x: e.clientX, y: e.clientY, start: { ...a, center: [...a.center] as [number, number] } };
  }

  private local(e: PointerEvent): [number, number] {
    const box = this.stage.getBoundingClientRect();
    return [e.clientX - box.left, e.clientY - box.top];
  }

  private pointerMove(e: PointerEvent): void {
    const [x, y] = this.local(e);
    const r = this.host.brushRadius();
    const inside = x >= 0 && y >= 0 && x <= this.stage.clientWidth && y <= this.stage.clientHeight;
    this.cursor.style.visibility = this.mode === 'brush' && inside ? 'visible' : 'hidden';
    if (this.mode === 'brush') {
      this.cursor.setAttribute('cx', String(x));
      this.cursor.setAttribute('cy', String(y));
      this.cursor.setAttribute('r', String(r));
    }
    if (this.painting) {
      this.queueDab([x, y]);
      return;
    }
    const d = this.drag;
    const a = this.activeInfo;
    if (!d || !a) return;
    const w = this.stage.clientWidth;
    const h = this.stage.clientHeight;
    const s = d.start;
    const cx = s.center[0] * w;
    const cy = s.center[1] * h;
    const box = this.stage.getBoundingClientRect();
    const [x0, y0] = [d.x - box.left, d.y - box.top];
    if (d.kind === 'move') a.center = [s.center[0] + (e.clientX - d.x) / w, s.center[1] + (e.clientY - d.y) / h];
    else if (d.kind === 'turn') {
      const turn = (Math.atan2(y - cy, x - cx) - Math.atan2(y0 - cy, x0 - cx)) * (180 / Math.PI);
      a.angle = ((((s.angle + turn + 180) % 360) + 360) % 360) - 180;
    } else {
      const k = Math.hypot(x - cx, y - cy) / Math.max(1, Math.hypot(x0 - cx, y0 - cy));
      a.size = Math.min(20, Math.max(0.01, s.size * k));
    }
    this.place(a.image);
    this.place();
  }

  private async pointerUp(): Promise<void> {
    if (this.painting) {
      this.painting = false;
      return;
    }
    const d = this.drag;
    const a = this.activeInfo;
    this.drag = null;
    if (!d || !a) return;
    await this.set({ image: a.image, center: a.center, angle: a.angle, size: a.size });
  }

  /** Changes one image's placement or flags in the engine and follows the reply. */
  async set(args: Record<string, unknown>): Promise<void> {
    const r = await this.host.dispatch<{ image: SpotlightInfo; revision: number }>('spotlight_set', args);
    if (!r.ok) {
      this.host.note(r.error.message);
      return;
    }
    if (r.result.image.active) for (const s of this.shown.values()) s.info.active = false;
    const s = this.shown.get(r.result.image.image);
    if (s) s.info = r.result.image;
    this.revision = r.result.revision;
    this.place();
  }

  /** The Spotlight brush: a primary press over the viewport paints or sculpts through the images, one soft dab per
   * position, while the camera keeps its other buttons. */
  private brushDown(e: PointerEvent): void {
    if (this.mode !== 'brush' || e.button !== 0 || e.altKey || !this.activeInfo) return;
    e.preventDefault();
    e.stopPropagation();
    this.painting = true;
    this.queueDab(this.local(e));
  }

  private queueDab(at: [number, number]): void {
    this.pending = at;
    if (!this.busy) void this.drain();
  }

  private async drain(): Promise<void> {
    this.busy = true;
    while (this.pending) {
      const at = this.pending;
      this.pending = null;
      await this.apply({ brush: { center: at, radius: this.host.brushRadius() } });
    }
    this.busy = false;
  }

  /** Applies the active image from the current view, all of it or under one brush dab. */
  async apply(extra: Record<string, unknown> = {}): Promise<void> {
    const id = this.host.activeObject();
    if (id === null || !this.activeInfo) return;
    const r = await this.host.dispatch<unknown>('spotlight_apply', {
      id,
      view_proj: this.host.viewProj(),
      width: Math.max(1, this.stage.clientWidth),
      height: Math.max(1, this.stage.clientHeight),
      ...this.host.applyArgs(typeof extra.mode === 'string' ? extra.mode : undefined),
      ...extra,
    });
    if (!r.ok) this.host.note(r.error.message);
    await this.host.refresh();
  }
}
