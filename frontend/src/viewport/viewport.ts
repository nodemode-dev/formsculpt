import type { MeshBuffers, VertexUpdate } from '../engine/service';
import { OrbitCamera, type ViewName } from './camera';
import { GlRenderer, hexColor } from './gl-renderer';
import type { Vec3 } from './math';

export interface ViewportStats {
  renderer: string;
  frameMs: number;
}

export interface Ray {
  origin: Vec3;
  dir: Vec3;
}

export interface StrokeHandler {
  enabled(): boolean;
  begin(ray: Ray, pressure: number, worldPerPixel: number, invert: boolean, tilt: number): void;
  move(ray: Ray, pressure: number, tilt: number): void;
  end(): void;
  missed(): boolean;
}

export class Viewport {
  readonly camera = new OrbitCamera();
  readonly renderer: GlRenderer;
  private canvas: HTMLCanvasElement;
  private dirty = true;
  private frame = 0;
  private lastFrameMs = 0;
  private drag: { button: number; x: number; y: number; mode: 'camera' | 'stroke' | 'pending' } | null = null;
  onFrame: ((stats: ViewportStats) => void) | null = null;
  strokes: StrokeHandler | null = null;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.renderer = new GlRenderer(canvas);
    const css = getComputedStyle(document.body);
    const bg = css.getPropertyValue('--canvas-bg');
    if (bg) this.renderer.setClearColor(bg);
    const b = this.renderer.backdrop;
    b.top = hexColor(css.getPropertyValue('--viewport-top')) ?? b.top;
    b.bottom = hexColor(css.getPropertyValue('--viewport-bottom')) ?? b.bottom;
    b.gridColor = hexColor(css.getPropertyValue('--viewport-grid')) ?? b.gridColor;
    new ResizeObserver(() => this.resize()).observe(canvas);
    canvas.addEventListener('pointerdown', (e) => this.down(e));
    canvas.addEventListener('pointermove', (e) => this.move(e));
    canvas.addEventListener('pointerup', (e) => this.up(e));
    canvas.addEventListener('pointercancel', (e) => this.up(e));
    canvas.addEventListener('wheel', (e) => this.wheel(e), { passive: false });
    canvas.addEventListener('contextmenu', (e) => e.preventDefault());
    this.resize();
  }

  private resize(): void {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const w = Math.max(1, Math.round(this.canvas.clientWidth * dpr));
    const h = Math.max(1, Math.round(this.canvas.clientHeight * dpr));
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    this.camera.aspect = w / h;
    this.invalidate();
  }

  invalidate(): void {
    this.dirty = true;
    if (!this.frame) this.frame = requestAnimationFrame(() => this.draw());
  }

  private draw(): void {
    this.frame = 0;
    if (!this.dirty) return;
    this.dirty = false;
    const t0 = performance.now();
    this.renderer.render(this.camera, this.canvas.width, this.canvas.height);
    this.lastFrameMs = performance.now() - t0;
    this.onFrame?.({ renderer: this.renderer.kind, frameMs: this.lastFrameMs });
  }

  setMesh(mesh: MeshBuffers): void {
    this.renderer.upload(mesh);
    this.invalidate();
  }

  patchMesh(update: VertexUpdate): void {
    if (this.renderer.patch(update)) this.invalidate();
  }

  removeMesh(id: number): void {
    this.renderer.remove(id);
    this.invalidate();
  }

  setActive(id: number | null): void {
    this.renderer.active = id;
    this.invalidate();
  }

  frameBounds(min: Vec3, max: Vec3): void {
    this.camera.frame(min, max);
    this.invalidate();
  }

  snap(view: ViewName): void {
    this.camera.snap(view);
    this.invalidate();
  }

  toggleOrtho(): boolean {
    this.camera.orthographic = !this.camera.orthographic;
    this.invalidate();
    return this.camera.orthographic;
  }

  /** A ray through a point given in client pixels (for picking on double-click). */
  rayAtClient(x: number, y: number): Ray | null {
    const r = this.canvas.getBoundingClientRect();
    return this.camera.ray(x - r.left, y - r.top, r.width, r.height);
  }

  /** Orbit around `point` from here on, without moving the eye: the view does not jump. */
  pivotAt(point: Vec3): void {
    this.camera.pivotAt(point);
    this.invalidate();
  }

  setFov(degrees: number): void {
    this.camera.fov = (Math.max(10, Math.min(120, degrees)) * Math.PI) / 180;
    this.invalidate();
  }

  private rayAt(e: PointerEvent): Ray | null {
    const r = this.canvas.getBoundingClientRect();
    return this.camera.ray(e.clientX - r.left, e.clientY - r.top, r.width, r.height);
  }

  private pressure(e: PointerEvent): number {
    return e.pointerType === 'mouse' ? 1 : Math.max(0.05, e.pressure || 0.5);
  }

  /** The direction a pen leans, in radians from the screen's up (0 when upright, or for a mouse or finger). */
  private tilt(e: PointerEvent): number {
    if (e.pointerType !== 'pen' || (Math.abs(e.tiltX) < 3 && Math.abs(e.tiltY) < 3)) return 0;
    return Math.atan2(e.tiltX, -e.tiltY);
  }

  /** Fingers on the canvas, by pointer id, and the last two-finger span and midpoint while a gesture runs. */
  private touches = new Map<number, [number, number]>();
  private gesture: { span: number; mid: [number, number] } | null = null;

  private gestureFrame(): { span: number; mid: [number, number] } {
    const [a, b] = [...this.touches.values()];
    return { span: Math.hypot(a[0] - b[0], a[1] - b[1]), mid: [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2] };
  }

  private down(e: PointerEvent): void {
    if (e.pointerType === 'touch') {
      this.touches.set(e.pointerId, [e.clientX, e.clientY]);
      if (this.touches.size === 2) {
        // A second finger ends any stroke the first one began and starts pinch to zoom and two-finger pan.
        if (this.drag?.mode === 'stroke') this.strokes?.end();
        this.drag = null;
        this.gesture = this.gestureFrame();
        this.canvas.setPointerCapture(e.pointerId);
        return;
      }
      if (this.touches.size > 2) return;
    }
    try {
      this.canvas.setPointerCapture(e.pointerId);
    } catch {
      // the pointer is already gone (or was never live, as with a synthesised event); the press still counts
    }
    this.canvas.focus({ preventScroll: true });
    // A pen touching down with its barrel button held, or with its eraser end, draws with the tool bound to that
    // button's hold (a temporary tool) instead of moving the camera as a right button would.
    let held = false;
    if (e.pointerType === 'pen') {
      const which = e.button === 5 || (e.buttons & 32) !== 0 ? 'eraser' : e.button === 2 || (e.buttons & 2) !== 0 ? 'barrel' : null;
      if (which) {
        this.penHeld = which;
        this.penButtonAt = performance.now();
        held = this.penHold?.(which, true) ?? false;
      }
    }
    // Alt and Shift turn and pan the view, except on a tool handle that takes them as modifiers (the gizmo's).
    const modified = e.altKey || e.shiftKey;
    // In fly navigation every drag looks around; nothing is sculpted.
    const sculpt = !this.flying && (e.button === 0 || held) && (!modified || (this.grabsModified?.(e.offsetX, e.offsetY) ?? false)) && this.strokes?.enabled();
    // A finger's stroke waits until it has moved a little, so a second finger arriving for a pinch leaves no dab.
    const pending = sculpt && e.pointerType === 'touch';
    this.drag = { button: e.button, x: e.clientX, y: e.clientY, mode: pending ? 'pending' : sculpt ? 'stroke' : 'camera' };
    this.pendingStart = pending ? { ray: this.rayAt(e), pressure: this.pressure(e), ctrl: e.ctrlKey || e.metaKey } : null;
    if (sculpt && !pending) this.beginStroke(this.rayAt(e), this.pressure(e), e.ctrlKey || e.metaKey, this.tilt(e));
  }

  private pendingStart: { ray: Ray | null; pressure: number; ctrl: boolean } | null = null;

  private beginStroke(ray: Ray | null, pressure: number, ctrl: boolean, tilt: number): void {
    if (ray) this.strokes?.begin(ray, pressure, this.camera.worldPerPixel(this.canvas.clientHeight), ctrl, tilt);
  }

  private move(e: PointerEvent): void {
    // The barrel button pressed and let go while the pen hovers (no contact) is reported as a press and a release.
    if (e.pointerType === 'pen' && !this.drag) {
      const barrel = (e.buttons & 2) !== 0;
      if (barrel !== this.hoverBarrel) {
        this.hoverBarrel = barrel;
        this.penButtonAt = performance.now();
        this.penHover?.(barrel);
      }
    }
    if (e.pointerType === 'touch' && this.touches.has(e.pointerId)) {
      this.touches.set(e.pointerId, [e.clientX, e.clientY]);
      if (this.gesture && this.touches.size === 2) {
        const next = this.gestureFrame();
        this.camera.pan(next.mid[0] - this.gesture.mid[0], next.mid[1] - this.gesture.mid[1], this.canvas.clientHeight);
        if (next.span > 1 && this.gesture.span > 1) this.camera.zoom(this.gesture.span / next.span);
        this.gesture = next;
        this.invalidate();
        return;
      }
    }
    if (!this.drag) return;
    if (this.drag.mode === 'pending') {
      if (Math.hypot(e.clientX - this.drag.x, e.clientY - this.drag.y) < 6 || !this.pendingStart) return;
      this.beginStroke(this.pendingStart.ray, this.pendingStart.pressure, this.pendingStart.ctrl, 0);
      this.pendingStart = null;
      this.drag.mode = 'stroke';
    }
    if (this.drag.mode === 'stroke') {
      if (this.strokes?.missed()) {
        this.strokes.end();
        this.drag.mode = 'camera';
      } else {
        const ray = this.rayAt(e);
        if (ray) this.strokes?.move(ray, this.pressure(e), this.tilt(e));
        this.drag.x = e.clientX;
        this.drag.y = e.clientY;
        return;
      }
    }
    const dx = e.clientX - this.drag.x;
    const dy = e.clientY - this.drag.y;
    this.drag.x = e.clientX;
    this.drag.y = e.clientY;
    const panning = this.drag.button === 1 || this.drag.button === 2 || e.shiftKey;
    if (this.flying && !panning) this.camera.look(dx, dy);
    else if (panning) this.camera.pan(dx, dy, this.canvas.clientHeight);
    else if (e.ctrlKey || e.metaKey) this.camera.zoom(Math.exp(dy * 0.01));
    else this.camera.orbit(dx, dy);
    this.invalidate();
  }

  private up(e: PointerEvent): void {
    if (this.canvas.hasPointerCapture(e.pointerId)) this.canvas.releasePointerCapture(e.pointerId);
    if (e.pointerType === 'touch') {
      this.touches.delete(e.pointerId);
      if (this.gesture) {
        // The gesture ends when a finger lifts; the remaining finger does nothing until it lifts too.
        if (this.touches.size < 2) this.gesture = null;
        this.drag = null;
        return;
      }
    }
    if (this.drag?.mode === 'pending' && this.pendingStart) {
      // A tap: one dab where the finger touched.
      this.beginStroke(this.pendingStart.ray, this.pendingStart.pressure, this.pendingStart.ctrl, 0);
      this.strokes?.end();
    } else if (this.drag?.mode === 'stroke') this.strokes?.end();
    this.pendingStart = null;
    this.drag = null;
    if (this.penHeld) {
      const which = this.penHeld;
      this.penHeld = null;
      this.hoverBarrel = false;
      this.penButtonAt = performance.now();
      this.penHold?.(which, false);
    }
  }

  /** Set by the app: the pen's barrel button or eraser end touched down (true) or lifted (false); returns whether a
   * temporary tool is in use, so the touch sculpts. */
  penHold: ((which: 'barrel' | 'eraser', down: boolean) => boolean) | null = null;
  /** Set by the app: the barrel button pressed (true) or let go (false) while the pen hovers. */
  penHover: ((down: boolean) => void) | null = null;
  /** Fly navigation (first person): drags look around from the eye instead of orbiting, and never sculpt. The keys
   * that walk are the app's. */
  flying = false;
  /** Set by the app: whether a press with Alt or Shift at this canvas point lands on a tool handle that takes the
   * modifier itself (the gizmo's), rather than turning or panning the view. */
  grabsModified: ((x: number, y: number) => boolean) | null = null;
  /** When a pen button last changed, so a context menu it raises can be ignored. */
  penButtonAt = -1e9;
  private penHeld: 'barrel' | 'eraser' | null = null;
  private hoverBarrel = false;

  private wheel(e: WheelEvent): void {
    e.preventDefault();
    this.camera.zoom(Math.exp(e.deltaY * 0.0015));
    this.invalidate();
  }
}
